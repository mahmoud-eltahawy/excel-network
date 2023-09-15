use crate::Id;
use chrono::NaiveDate;
use leptos::spawn_local;
use leptos::{ev::MouseEvent, *};
use leptos_router::*;
use models::{
    ConfigValue, FrontendColumn, FrontendColumnValue, FrontendRow, FrontendSheet, HeaderGetter,
    RowIdentity, RowsSort,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{collections::HashMap, rc::Rc};
use tauri_sys::tauri::invoke;
use uuid::Uuid;

const SAVE_EDITS_TOTAL_TASKS: i32 = 6;

use super::shared::{
    alert, confirm, import_sheet_rows, message, open_file, resolve_operation, EditState, InputRow,
    Name, NameArg, PrimaryRow, SheetHead, ShowNewRows,
};

#[derive(Serialize, Deserialize)]
struct ExportSheetArg {
    headers: Vec<Rc<str>>,
    sheet: FrontendSheet,
}

#[derive(Serialize, Deserialize)]
struct SheetNameArg {
    name: Name,
}

#[derive(Serialize, Deserialize)]
struct UpdateColumnsArgs {
    sheetid: Uuid,
    columnsidentifiers: Vec<(Uuid, Rc<str>, FrontendColumnValue)>,
}

#[derive(Serialize, Deserialize)]
struct DeleteColumnsArgs {
    sheetid: Uuid,
    rowsheaders: Vec<(Uuid, Rc<str>)>,
}

#[derive(Serialize, Deserialize)]
struct RowsDeleteArg {
    sheetid: Uuid,
    rowsids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
struct RowsAddArg {
    sheetid: Uuid,
    rows: Vec<FrontendRow>,
}

#[derive(Debug, Clone)]
struct ColumnIdentity {
    row_id: Uuid,
    header: Rc<str>,
    value: FrontendColumnValue,
}

const OFFSET_LIMIT: i64 = 7;

#[component]
fn ColumnEdit<F1, F2, F3>(mode: F1, cancel: F2, push_to_modified: F3) -> impl IntoView
where
    F1: Fn() -> ColumnIdentity + 'static,
    F2: Fn() + 'static + Clone + Copy,
    F3: Fn(ColumnIdentity) + 'static,
{
    let column_value = RwSignal::from(mode().value);
    let on_input = move |ev| {
        let value = event_target_value(&ev);
        let value = match column_value.get() {
            FrontendColumnValue::Float(_) => {
                FrontendColumnValue::Float(value.parse().unwrap_or_default())
            }
            FrontendColumnValue::Date(_) => {
                FrontendColumnValue::Date(Some(value.parse().unwrap_or_default()))
            }
            _ => FrontendColumnValue::String(Some(Rc::from(value))),
        };
        column_value.set(value);
    };

    let save = move |_| {
        let ColumnIdentity {
            row_id,
            header,
            value: _,
        } = mode();
        push_to_modified(ColumnIdentity {
            row_id,
            header,
            value: column_value.get(),
        });
        cancel();
    };

    view! {
        <div class="popup">
            <input
                type=move || match column_value.get() {
                    FrontendColumnValue::Float(_) => "number",
                    FrontendColumnValue::Date(_) => "date",
                    _ => "text",
                }
                placeholder=move || {
                    format!(
                        "{} ({})", "القيمة الحالية", column_value.get().to_string()
                    )
                }
                on:input=on_input
            />
            <button on:click=move|_| cancel() class="centered-button">
                "الغاء"
            </button>
            <button on:click=save class="centered-button">
                "تاكيد"
            </button>
        </div>
    }
}

#[derive(Serialize, Deserialize)]
struct LimitedId {
    id: Option<Uuid>,
    offset: i64,
    limit: i64,
}

#[component]
pub fn ShowSheet() -> impl IntoView {
    let edit_mode = RwSignal::from(EditState::None);
    let sheet_name = RwSignal::<Rc<str>>::from(Rc::from(""));
    let deleted_rows = RwSignal::from(Vec::<Uuid>::new());
    let added_rows = RwSignal::from(Vec::<FrontendRow>::new());
    let modified_columns = RwSignal::from(Vec::<ColumnIdentity>::new());
    let modified_primary_columns = RwSignal::from(HashMap::<Rc<str>, FrontendColumn>::new());
    let deleted_primary_columns = RwSignal::from(Vec::<Rc<str>>::new());
    let on_edit = RwSignal::from(false);
    let params = use_params_map();
    let sheet_type_id = move || {
        params.with(|params| match params.get("sheet_type_id") {
            Some(id) => Uuid::from_str(id).ok(),
            None => None,
        })
    };

    let sheet_type_name_resource = Resource::once(move || async move {
        invoke::<Id, Rc<str>>(
            "sheet_type_name",
            &Id {
                id: sheet_type_id(),
            },
        )
        .await
        .unwrap_or(Rc::from(""))
    });

    let sheet_priorities_resource = Resource::new(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Rc<[Rc<str>]>>("get_priorities", &NameArg { name })
                .await
                .unwrap_or(Rc::from([]))
        },
    );

    let rows_ids_resource = Resource::new(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, RowIdentity>("get_rows_ids", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );

    Effect::new(move |_| logging::log!("{:#?}", rows_ids_resource.get()));

    let sheet_id = move || {
        params.with(|params| match params.get("sheet_id") {
            Some(id) => Uuid::from_str(id).ok(),
            None => None,
        })
    };

    let sheet_resource = Resource::once(move || async move {
        invoke::<Id, (FrontendSheet, i64)>("get_sheet", &Id { id: sheet_id() })
            .await
            .ok()
    });

    let rows_offset = RwSignal::from(OFFSET_LIMIT);

    let rows_number = Memo::new(move |_| {
        let default = OFFSET_LIMIT * 4;
        sheet_resource
            .get()
            .map(|x| x.map(|x| x.1).unwrap_or(default))
            .unwrap_or(default)
    });

    let rows_accumalator = RwSignal::from(vec![]);

    let sheet_rows_resource = Resource::new(
        move || rows_offset.get(),
        move |offset| async move {
            let rows_number = rows_number.get();
            let v = invoke::<_, Vec<FrontendRow>>(
                "get_sheet_rows",
                &LimitedId {
                    id: sheet_id(),
                    offset,
                    limit: OFFSET_LIMIT,
                },
            )
            .await
            .unwrap_or_default();
            if offset <= rows_number {
                rows_offset.update(|x| *x += OFFSET_LIMIT);
            }
            if !v.is_empty() {
                rows_accumalator.update(|xs| xs.extend(v));
            }
        },
    );

    let get_accumalted_rows = move || {
        sheet_rows_resource.get();
        rows_accumalator.get()
    };

    let sheet_init_memo = Memo::new(move |_| {
        let s = sheet_resource.get().map(|x| x.map(|x| x.0));
        match s {
            Some(Some(s)) => Some(s),
            _ => None,
        }
    });

    let sheet_headers_resource = Resource::new(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Rc<[ConfigValue]>>("sheet_headers", &NameArg { name })
                .await
                .unwrap_or(Rc::from([]))
        },
    );
    let sheet_primary_headers_resource = Resource::new(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Vec<Rc<str>>>("sheet_primary_headers", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );

    let basic_columns = Memo::new(move |_| {
        sheet_headers_resource
            .get()
            .unwrap_or(Rc::from([]))
            .iter()
            .flat_map(|x| match x {
                ConfigValue::Basic(conf) => Some(conf),
                ConfigValue::Calculated(_) => None,
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    let calc_columns = Memo::new(move |_| {
        sheet_headers_resource
            .get()
            .unwrap_or(Rc::from([]))
            .iter()
            .flat_map(|x| match x {
                ConfigValue::Basic(_) => None,
                ConfigValue::Calculated(conf) => Some(conf),
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    let basic_headers = move || {
        basic_columns
            .get()
            .into_iter()
            .map(|x| x.get_header())
            .collect::<Vec<_>>()
    };

    let calc_headers = move || {
        calc_columns
            .get()
            .into_iter()
            .map(|x| Rc::from(x.header))
            .collect::<Vec<Rc<str>>>()
    };

    let sheet_with_primary_row_with_calc_values = Memo::new(move |_| {
        let c_cols = calc_columns.get();
        let mut sheet = match sheet_init_memo.get() {
            Some(x) => x,
            None => {
                return FrontendSheet {
                    id: Uuid::nil(),
                    sheet_name: Rc::from(""),
                    type_name: Rc::from(""),
                    insert_date: NaiveDate::default(),
                    rows: vec![],
                }
            }
        };
        sheet.rows = sheet
            .rows
            .into_iter()
            .chain(get_accumalted_rows())
            .map(|FrontendRow { id, columns }| FrontendRow {
                id,
                columns: {
                    let mut columns = columns;
                    for header in calc_headers().into_iter() {
                        let mut map = HashMap::new();
                        for (col_header, FrontendColumn { is_basic: _, value }) in &columns {
                            map.insert(col_header.clone(), value.clone());
                        }
                        let value = &c_cols;
                        let value = value
                            .iter()
                            .filter(|x| x.header == header.to_string())
                            .collect::<Vec<_>>();
                        let value = value.first().unwrap();

                        if id != sheet.id {
                            columns.insert(
                                header,
                                FrontendColumn {
                                    is_basic: false,
                                    value: FrontendColumnValue::Float(
                                        resolve_operation(&value.value, &map).unwrap_or_default(),
                                    ),
                                },
                            );
                        }
                    }
                    columns
                },
            })
            .collect::<Vec<_>>();
        sheet
    });

    let sheet_without_primary_row_with_calc_values = Memo::new(move |_| {
        let mut sheet = sheet_with_primary_row_with_calc_values.get();
        sheet.rows = sheet
            .rows
            .into_iter()
            .chain(get_accumalted_rows())
            .filter(|x| x.id != sheet_init_memo.get().map(|x| x.id).unwrap_or_default())
            .collect::<Vec<_>>();
        sheet
    });

    let export = move |_| {
        spawn_local(async move {
            match invoke::<_, ()>(
                "export_sheet",
                &ExportSheetArg {
                    sheet: sheet_with_primary_row_with_calc_values.get(),
                    headers: basic_headers()
                        .into_iter()
                        .chain(calc_headers())
                        .collect::<Vec<Rc<str>>>(),
                },
            )
            .await
            {
                Ok(_) => message("نجح التصدير").await,
                Err(err) => alert(err.to_string().as_str()).await,
            }
        })
    };

    let is_deleted = move |id| deleted_rows.get().contains(&id);
    let delete_row = move |id| {
        if deleted_rows.get().contains(&id) {
            deleted_rows.update(|xs| xs.retain(|x| *x != id));
        } else {
            deleted_rows.update(|xs| xs.push(id));
        }
    };
    let delete_new_row = move |id| added_rows.update(|xs| xs.retain(|x| x.id != id));

    let has_anything_changed = move || {
        !sheet_name.get().is_empty()
            || !deleted_rows.get().is_empty()
            || !added_rows.get().is_empty()
            || !modified_columns.get().is_empty()
            || !modified_primary_columns.get().is_empty()
            || !deleted_primary_columns.get().is_empty()
    };

    let cancel_edit = move || {
        spawn_local(async move {
            let reset = if has_anything_changed() {
                confirm("سيتم تجاهل كل التعديلات").await
            } else {
                true
            };
            if reset {
                edit_mode.set(EditState::None);
                sheet_name.set(Rc::from(""));
                deleted_rows.set(Vec::new());
                added_rows.set(Vec::new());
                modified_columns.set(Vec::new());
                modified_primary_columns.set(HashMap::new());
                deleted_primary_columns.set(Vec::new());
            }
        })
    };
    let append = move |row| {
        added_rows.update_untracked(|xs| xs.push(row));
        added_rows
            .update(|xs| xs.sort_rows(sheet_priorities_resource.get().unwrap_or(Rc::from([]))));
    };
    let primary_row_columns = Memo::new(move |_| {
        let Some(FrontendSheet { id, rows, .. }) = sheet_init_memo.get() else {
            return HashMap::new();
        };
        rows.into_iter()
            .chain(get_accumalted_rows())
            .filter(|x| x.id == id)
            .collect::<Vec<_>>()
            .first()
            .map(|x| x.columns.clone())
            .unwrap_or_default()
    });
    let save_edits_successes = RwSignal::from(0);
    let save_edits_dones = RwSignal::from(0);

    let save_edits = move |_| {
        let Some(sheet) = sheet_init_memo.get() else {
            return;
        };
        let sheetid = sheet.id;
        let the_sheet_name = sheet_name.get();
        let the_deleted_rows = deleted_rows.get();
        let the_added_rows = added_rows.get();
        let new_row_primary_columns = modified_primary_columns
            .get()
            .into_iter()
            .filter(|(header, _)| {
                primary_row_columns
                    .get()
                    .iter()
                    .all(|(old_header, _)| header != old_header)
            })
            .map(|(header, column)| (sheetid, header, column.value));
        let updated_row_primary_columns = modified_primary_columns
            .get()
            .into_iter()
            .filter(|(header, _)| {
                primary_row_columns
                    .get()
                    .iter()
                    .any(|(old_header, _)| header == old_header)
            })
            .map(|(header, column)| (sheetid, header, column.value));

        let new_columnsidentifiers = modified_columns
            .get()
            .into_iter()
            .filter(|ColumnIdentity { row_id, header, .. }| {
                sheet
                    .rows
                    .iter()
                    .filter(|x| x.id == *row_id)
                    .any(|x| x.columns.keys().any(|x| x != header))
            })
            .map(|x| (x.row_id, x.header, x.value))
            .chain(new_row_primary_columns)
            .collect::<Vec<_>>();

        let updated_columnsidentifiers = modified_columns
            .get()
            .into_iter()
            .filter(|ColumnIdentity { row_id, header, .. }| {
                sheet
                    .rows
                    .iter()
                    .filter(|x| x.id == *row_id)
                    .any(|x| x.columns.keys().any(|x| x == header))
            })
            .map(|x| (x.row_id, x.header, x.value))
            .chain(updated_row_primary_columns)
            .collect::<Vec<_>>();

        let primary_deleted_columnsidentifiers = deleted_primary_columns
            .get()
            .into_iter()
            .map(|x| (sheetid.clone(), x))
            .collect::<Vec<_>>();
        spawn_local(async move {
            if !the_sheet_name.is_empty() && the_sheet_name != sheet.sheet_name {
                match invoke::<_, ()>(
                    "update_sheet_name",
                    &SheetNameArg {
                        name: Name {
                            id: sheet.id,
                            the_name: the_sheet_name,
                        },
                    },
                )
                .await
                {
                    Ok(_) => save_edits_successes.update(|x| *x += 1),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        save_edits_successes.set(0);
                    }
                }
            } else {
                save_edits_successes.update(|x| *x += 1);
            }
            save_edits_dones.update(|x| *x += 1);
        });
        spawn_local(async move {
            if !the_deleted_rows.is_empty() {
                match invoke::<_, ()>(
                    "delete_rows_from_sheet",
                    &RowsDeleteArg {
                        sheetid: sheet.id,
                        rowsids: the_deleted_rows,
                    },
                )
                .await
                {
                    Ok(_) => save_edits_successes.update(|x| *x += 1),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        save_edits_successes.set(0);
                    }
                }
            } else {
                save_edits_successes.update(|x| *x += 1);
            }
            save_edits_dones.update(|x| *x += 1);
        });
        spawn_local(async move {
            if !the_added_rows.is_empty() {
                match invoke::<_, ()>(
                    "add_rows_to_sheet",
                    &RowsAddArg {
                        sheetid: sheet.id,
                        rows: the_added_rows,
                    },
                )
                .await
                {
                    Ok(_) => save_edits_successes.update(|x| *x += 1),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        save_edits_successes.set(0);
                    }
                }
            } else {
                save_edits_successes.update(|x| *x += 1);
            }
            save_edits_dones.update(|x| *x += 1);
        });
        spawn_local(async move {
            if !primary_deleted_columnsidentifiers.is_empty() {
                match invoke::<_, ()>(
                    "delete_columns",
                    &DeleteColumnsArgs {
                        sheetid,
                        rowsheaders: primary_deleted_columnsidentifiers,
                    },
                )
                .await
                {
                    Ok(_) => save_edits_successes.update(|x| *x += 1),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        save_edits_successes.set(0);
                    }
                }
            } else {
                save_edits_successes.update(|x| *x += 1);
            }
            save_edits_dones.update_untracked(|x| *x += 1);
        });
        spawn_local(async move {
            if !updated_columnsidentifiers.is_empty() {
                match invoke::<_, ()>(
                    "update_columns",
                    &UpdateColumnsArgs {
                        sheetid,
                        columnsidentifiers: updated_columnsidentifiers,
                    },
                )
                .await
                {
                    Ok(_) => save_edits_successes.update_untracked(|x| *x += 1),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        save_edits_successes.set(0);
                    }
                }
            } else {
                save_edits_successes.update(|x| *x += 1);
            }
            save_edits_dones.update(|x| *x += 1);
        });
        spawn_local(async move {
            if !new_columnsidentifiers.is_empty() {
                match invoke::<_, ()>(
                    "save_columns",
                    &UpdateColumnsArgs {
                        sheetid,
                        columnsidentifiers: new_columnsidentifiers,
                    },
                )
                .await
                {
                    Ok(_) => save_edits_successes.update(|x| *x += 1),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        save_edits_successes.set(0);
                    }
                }
            } else {
                save_edits_successes.update(|x| *x += 1);
            }
            save_edits_dones.update(|x| *x += 1);
        });

        edit_mode.set(EditState::None);
        sheet_name.set(Rc::from(""));
        deleted_rows.set(Vec::new());
        added_rows.set(Vec::new());
        modified_columns.set(Vec::new());
        modified_primary_columns.set(HashMap::new());
        deleted_primary_columns.set(Vec::new());
        on_edit.set(false);
    };

    Effect::new(move |_| {
        if save_edits_successes.get() == SAVE_EDITS_TOTAL_TASKS {
            spawn_local(async move {
                message("👍").await;
                save_edits_successes.set(0);
            });
        }
    });

    Effect::new(move |_| {
        if save_edits_dones.get() == SAVE_EDITS_TOTAL_TASKS {
            sheet_resource.refetch();
            save_edits_dones.set(0);
        }
    });

    let load_file = move |_| {
        let sheettype = sheet_type_name_resource.get().unwrap_or(Rc::from(""));
        edit_mode.set(EditState::LoadFile);
        spawn_local(async move {
            let Some(filepath) = open_file().await else {
                return;
            };
            let sheet_id = sheet_init_memo.get().map(|x| x.id).unwrap_or_default();
            let rows = import_sheet_rows(sheet_id, sheettype, filepath).await;

            let primary_row = rows.iter().filter(|x| x.id == sheet_id).collect::<Vec<_>>();
            let primary_row = primary_row
                .first()
                .map(|x| x.columns.clone())
                .unwrap_or_default();
            let old_primary_row = primary_row_columns.get();
            for (header, column) in primary_row {
                if old_primary_row
                    .get(&header)
                    .is_some_and(|x| x.value != column.value)
                {
                    modified_primary_columns.update(|map| {
                        map.insert(header, column);
                    })
                }
            }
            added_rows.update_untracked(|xs| {
                xs.extend(
                    rows.into_iter()
                        .filter(|x| x.id != sheet_id)
                        .collect::<Vec<_>>(),
                )
            });
            added_rows
                .update(|xs| xs.sort_rows(sheet_priorities_resource.get().unwrap_or(Rc::from([]))));
        });
    };

    let primary_row_non_primary_headers = move || {
        let primary_headers = sheet_primary_headers_resource.get().unwrap_or_default();

        modified_primary_columns
            .get()
            .into_iter()
            .chain(primary_row_columns.get())
            .collect::<HashMap<_, _>>()
            .keys()
            .cloned()
            .filter(|x| !primary_headers.contains(x))
            .collect::<Vec<_>>()
    };

    let toggle_edit_mode = move |_| {
        if on_edit.get() {
            on_edit.set(false);
        } else {
            on_edit.set(true);
        }
        cancel_edit()
    };

    view! {
        <section>
            <A class="left-corner" href=format!("/sheet/{}", sheet_type_id().unwrap_or_default())>
                "->"
            </A>
            <button class="right-corner" on:click=export>
                "🏹"
            </button>
            <button on:click=toggle_edit_mode class="right-corner-left">
                {
                    move || if on_edit.get() {
                         "X"
                    } else {
                         "✏️"
                    }
                }
            </button>
            <Show
                when=has_anything_changed
                fallback=|| {
                    view! {  <></> }
                }
            >
                <button on:click=save_edits class="left-corner-right">
                    "💾"
                </button>
            </Show>
            <br/>
            <Show
                when=move || matches!(edit_mode.get(),EditState::Primary)
                fallback=move || {
                    view! {<h1>{move || sheet_init_memo.get().map(|x| x.sheet_name.to_string())}</h1> }
                }
            >
                <input
                    type="text"
                    class="centered-input"
                    placeholder=move || {
                        format!(
                            "{} ({})", "اسم الشيت", sheet_init_memo
                                .get().map(|x| x.sheet_name.to_string()).unwrap_or_default()
                        )
                    }
                    value=move || sheet_name.get().to_string()
                    on:input=move |ev| sheet_name.set(Rc::from(event_target_value(&ev)))
                />
            </Show>
        <PrimaryRow
          columns=primary_row_columns
          non_primary_headers=primary_row_non_primary_headers
          new_columns=modified_primary_columns
          deleted_columns=deleted_primary_columns
          primary_headers=move || sheet_primary_headers_resource.get().unwrap_or_default()
          edit_mode=edit_mode
        /><br/>
        <Show
        when=move || rows_offset.get() < rows_number.get()
        fallback=|| view!{<></>}>
            <progress max=move || rows_number.get() value=move || rows_offset.get()/>
        </Show>
            <table>
                <SheetHead basic_headers=basic_headers calc_headers=calc_headers/>
                <tbody>
                    <ShowRows
                        delete_row=delete_row
                        basic_headers=basic_headers
                        calc_headers=calc_headers
                        rows=move || sheet_without_primary_row_with_calc_values.get().rows
                        edit_mode=edit_mode
                        is_deleted=is_deleted
                        modified_columns=modified_columns
                        sheet_id=move || sheet_init_memo.get().map(|x| x.id).unwrap_or_default()
            />
            <Show
            when=move || !added_rows.get().is_empty()
            fallback=move || view!{<></>}
            >
            <tr><td class="shapeless">r"+"</td></tr>
            </Show>
                    <ShowNewRows
                        delete_row=delete_new_row
                        basic_headers=basic_headers
                        calc_headers=calc_headers
                        rows=added_rows
                    sheet_id=move ||sheet_init_memo.get().map(|x| x.id).unwrap_or_default()
                    priorities=move || sheet_priorities_resource.get().unwrap_or(Rc::from([]))
                    />
                    <Show
                        when=move || matches!(edit_mode.get(),EditState::NonePrimary)
                        fallback=|| {
                            view! {  <></> }
                        }
                    >
                        <InputRow
                            basic_headers=basic_headers
                            calc_headers=calc_headers
                            append=append
                            basic_columns=basic_columns
                            calc_columns=calc_columns
                        />
                    </Show>
                </tbody>
            </table>
            <EditButtons
                edit_mode=edit_mode
                load_file=load_file
                on_edit=on_edit
            />
            <Outlet/>
        </section>
    }
}

#[component]
fn EditButtons<FL>(
    edit_mode: RwSignal<EditState>,
    load_file: FL,
    on_edit: RwSignal<bool>,
) -> impl IntoView
where
    FL: Fn(MouseEvent) + 'static + Clone + Copy,
{
    view! {
        <Show
        when=move || on_edit.get() && matches!(edit_mode.get(),EditState::None)
        fallback=|| view! {<></>}
        >
        <div class="popup">
            <br/>
            <button
                on:click=move |_| edit_mode.set(EditState::Primary)
                class="centered-button"
            >"تعديل العناوين"</button>
            <br/>
            <button
                on:click=move |_| edit_mode.set(EditState::NonePrimary)
                class="centered-button"
            >"تعديل الصفوف"</button>
            <br/>
            <button on:click=load_file class="centered-button">
                "تحميل ملف"
            </button>
            <br/>
            <button on:click=move |_| on_edit.set(false) class="centered-button">
                "الغاء"
            </button>
        </div>
        </Show>
    }
}

#[component]
fn ShowRows<BH, CH, FD, ID, FI, FR>(
    basic_headers: BH,
    calc_headers: CH,
    delete_row: FD,
    is_deleted: ID,
    sheet_id: FI,
    rows: FR,
    edit_mode: RwSignal<EditState>,
    modified_columns: RwSignal<Vec<ColumnIdentity>>,
) -> impl IntoView
where
    BH: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    CH: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    FR: Fn() -> Vec<FrontendRow> + 'static + Clone + Copy,
    ID: Fn(Uuid) -> bool + 'static + Clone + Copy,
    FD: Fn(Uuid) + 'static + Clone + Copy,
    FI: Fn() -> Uuid + 'static + Clone + Copy,
{
    let edit_column = RwSignal::from(None::<ColumnIdentity>);
    let new_rows = Memo::new(move |_| {
        rows()
            .into_iter()
            .filter(|x| x.id != sheet_id())
            .collect::<Vec<_>>()
    });

    view! {
    <>
            <Show
                when=move || edit_column.get().is_some()
                fallback=|| {
                    view! {  <></> }
                }
            >
                <ColumnEdit
                    mode=move || edit_column.get().unwrap()
                    cancel=move || edit_column.set(None)
                push_to_modified=move |col| modified_columns.update(|xs| xs.push(col))
                />
            </Show>
        <For
            each=move || new_rows.get()
            key=|row| row.id
            view=move | FrontendRow { columns, id }| {
                let columns = std::rc::Rc::new(columns);
                view! {
                    <tr>
                        {
                            let columns = columns.clone();
                            view! {
                                <For
                                    each=basic_headers
                                    key=|key| key.clone()
                                    view=move |column| {
                        let header1 = column.clone();
                        let header2 = header1.clone();
                        let header3 = header2.clone();
                        let columns1 = columns.clone();
                        let columns2 = columns1.clone();
                        view! { <td
                                style="cursor: pointer"
                                 on:dblclick=move |_| if matches!(edit_mode.get(),EditState::NonePrimary) {
                            edit_column.set(Some(ColumnIdentity{
                            row_id:id,
                            header:header1.clone(),
                            value :columns1
                                .get(&header2)
                                        .map(|x| x.value.clone())
                                        .unwrap_or(FrontendColumnValue::String(Some(Rc::from("Empty"))))
                            }))
                         }
                         >{
                        move || columns2
                        .get(&column)
                        .map(|x| x.value.to_string())
                        } {
                        move || modified_columns.get()
                        .into_iter().filter(|x| x.row_id == id && x.header ==header3)
                        .collect::<Vec<_>>()
                        .first()
                        .map(|x| format!(" => {}",x.value.to_string()))
                        }</td>
                    }
                                    }
                                />
                            }
                        } <td class="shapeless">"  "</td> {
                            let columns = columns.clone();
                            view! {
                                <For
                                    each=calc_headers
                                    key=|key| key.clone()
                                    view=move | column| {
                                        let columns = columns.clone();
                                        view! {  <td>{move || columns.get(&column).map(|x| x.value.to_string())}</td> }
                                    }
                                />
                            }
                        }
                        <Show
                            when=move || matches!(edit_mode.get(),EditState::NonePrimary)
                            fallback=|| {
                                view! {  <></> }
                            }
                        >
                            <td>
                <button on:click=move |_| {
                    if modified_columns
                    .get()
                    .iter()
                    .any(|x| x.row_id == id) {
                    modified_columns.update(|xs| xs.retain(|x| x.row_id != id))
                    } else {
                    delete_row(id)
                    }
                }>
                                    {move || if is_deleted(id) { "P" } else { "X" }}
                                </button>
                            </td>
                        </Show>
                    </tr>
                }
            }
        />
    </>
    }
}
