use crate::Id;
use leptos::{ev::MouseEvent, *};
use leptos_router::*;
use models::{
    Column, ColumnValue, ConfigValue, HeaderGetter, Name, Row, RowIdentity, RowsSort, Sheet,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tauri_sys::tauri::invoke;
use uuid::Uuid;

use super::shared::{
    alert, confirm, import_sheet_rows, message, open_file, resolve_operation, EditState, InputRow,
    NameArg, PrimaryRow, SheetHead, ShowNewRows,
};

#[derive(Serialize, Deserialize)]
struct ExportSheetArg {
    headers: Vec<String>,
    sheet: Sheet,
}

#[derive(Serialize, Deserialize)]
struct SheetNameArg {
    name: Name,
}

#[derive(Serialize, Deserialize)]
struct UpdateColumnsArgs {
    sheetid: Uuid,
    columnsidentifiers: Vec<(Uuid, String, ColumnValue)>,
}

#[derive(Serialize, Deserialize)]
struct RowsDeleteArg {
    sheetid: Uuid,
    rowsids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
struct RowsAddArg {
    sheetid: Uuid,
    rows: Vec<Row>,
}

#[derive(Debug, Clone)]
struct ColumnIdentity {
    row_id: Uuid,
    header: String,
    value: ColumnValue,
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
            ColumnValue::Float(_) => ColumnValue::Float(value.parse().unwrap_or_default()),
            ColumnValue::Date(_) => ColumnValue::Date(Some(value.parse().unwrap_or_default())),
            _ => ColumnValue::String(Some(value)),
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
                    ColumnValue::Float(_) => "number",
                    ColumnValue::Date(_) => "date",
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
    let sheet_name = RwSignal::from(String::from(""));
    let deleted_rows = RwSignal::from(Vec::<Uuid>::new());
    let added_rows = RwSignal::from(Vec::<Row>::new());
    let modified_columns = RwSignal::from(Vec::<ColumnIdentity>::new());
    let modified_primary_columns = RwSignal::from(HashMap::<String, Column>::new());
    let on_edit = RwSignal::from(false);
    let params = use_params_map();
    let sheet_type_id = move || {
        params.with(|params| match params.get("sheet_type_id") {
            Some(id) => Uuid::from_str(id).ok(),
            None => None,
        })
    };
    let sheet_type_name_resource = create_resource(
        || (),
        move |_| async move {
            invoke::<Id, String>(
                "sheet_type_name",
                &Id {
                    id: sheet_type_id(),
                },
            )
            .await
            .unwrap_or_default()
        },
    );

    let sheet_priorities_resource = create_resource(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Vec<String>>("get_priorities", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );

    let rows_ids_resource = create_resource(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, RowIdentity>("get_rows_ids", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );

    create_effect(move |_| logging::log!("{:#?}", rows_ids_resource.get()));

    let sheet_id = move || {
        params.with(|params| match params.get("sheet_id") {
            Some(id) => Uuid::from_str(id).ok(),
            None => None,
        })
    };

    let sheet_resource = create_resource(
        || (),
        move |_| async move {
            invoke::<Id, (Sheet, i64)>("get_sheet", &Id { id: sheet_id() })
                .await
                .unwrap_or_default()
        },
    );

    let rows_offset = RwSignal::from(OFFSET_LIMIT);
    let rows_number = create_memo(move |_| {
        sheet_resource
            .get()
            .map(|x| x.1)
            .unwrap_or(OFFSET_LIMIT * 4)
    });

    let rows_accumalator = RwSignal::from(vec![]);

    let sheet_rows_resource = create_resource(
        move || rows_offset.get(),
        move |offset| async move {
            let rows_number = rows_number.get();
            let v = invoke::<_, Vec<Row>>(
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

    let sheet_init_memo = create_memo(move |_| sheet_resource.get().map(|x| x.0));

    let sheet_headers_resource = create_resource(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Vec<ConfigValue>>("sheet_headers", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );
    let sheet_primary_headers_resource = create_resource(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Vec<String>>("sheet_primary_headers", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );

    let basic_columns = create_memo(move |_| {
        sheet_headers_resource
            .get()
            .unwrap_or_default()
            .into_iter()
            .flat_map(|x| match x {
                ConfigValue::Basic(conf) => Some(conf),
                ConfigValue::Calculated(_) => None,
            })
            .collect::<Vec<_>>()
    });

    let calc_columns = create_memo(move |_| {
        sheet_headers_resource
            .get()
            .unwrap_or_default()
            .into_iter()
            .flat_map(|x| match x {
                ConfigValue::Basic(_) => None,
                ConfigValue::Calculated(conf) => Some(conf),
            })
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
            .map(|x| x.header)
            .collect::<Vec<_>>()
    };

    // let basic_rows_columns =
    //     create_memo(move |_| sheet_resource.get().map(|x| x.rows).unwrap_or_default());

    // let calc_rows_columns = create_memo(move |_| {
    //     basic_rows_columns.get().into_iter().map(|x| {
    //         let columns = x.columns;
    //     })
    // });

    let sheet_with_primary_row_with_calc_values = create_memo(move |_| {
        let c_cols = calc_columns.get();
        let mut sheet = sheet_init_memo.get().unwrap_or_default();
        sheet.rows = sheet
            .rows
            .into_iter()
            .chain(get_accumalted_rows())
            .map(|Row { id, columns }| Row {
                id,
                columns: {
                    let mut columns = columns;
                    for header in calc_headers().into_iter() {
                        let mut map = HashMap::new();
                        for (col_header, Column { is_basic: _, value }) in &columns {
                            map.insert(col_header.clone(), value.clone());
                        }
                        let value = &c_cols;
                        let value = value
                            .iter()
                            .filter(|x| x.header == header)
                            .collect::<Vec<_>>();
                        let value = value.first().unwrap();

                        if id != sheet.id {
                            columns.insert(
                                header,
                                Column {
                                    is_basic: false,
                                    value: ColumnValue::Float(
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

    let sheet_without_primary_row_with_calc_values = create_memo(move |_| {
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
                    headers: basic_headers().into_iter().chain(calc_headers()).collect(),
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
                sheet_name.set(String::from(""));
                deleted_rows.set(Vec::new());
                added_rows.set(Vec::new());
                modified_columns.set(Vec::new());
                modified_primary_columns.set(HashMap::new());
            }
        })
    };
    let append = move |row| {
        added_rows.update_untracked(|xs| xs.push(row));
        added_rows.update(|xs| xs.sort_rows(sheet_priorities_resource.get().unwrap_or_default()));
    };
    let primary_row_columns = create_memo(move |_| {
        let Some(Sheet { id, rows, .. }) = sheet_init_memo.get() else {
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
                    .filter(|x| x.id.clone() == *row_id)
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
                    .filter(|x| x.id.clone() == *row_id)
                    .any(|x| x.columns.keys().any(|x| x == header))
            })
            .map(|x| (x.row_id, x.header, x.value))
            .chain(updated_row_primary_columns)
            .collect::<Vec<_>>();
        let mut success = true;
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
                    Ok(_) => (),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        success = false;
                    }
                }
            }
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
                    Ok(_) => (),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        success = false;
                    }
                }
            }
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
                    Ok(_) => (),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        success = false;
                    }
                }
            }

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
                    Ok(_) => (),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        success = false;
                    }
                }
            }

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
                    Ok(_) => (),
                    Err(err) => {
                        alert(err.to_string().as_str()).await;
                        success = false;
                    }
                }
            }

            if success {
                message("👍").await;
            }

            sheet_resource.refetch();
        });

        edit_mode.set(EditState::None);
        sheet_name.set(String::from(""));
        deleted_rows.set(Vec::new());
        added_rows.set(Vec::new());
        modified_columns.set(Vec::new());
        modified_primary_columns.set(HashMap::new());
        on_edit.set(false);
    };

    let load_file = move |_| {
        let sheettype = sheet_type_name_resource.get().unwrap_or_default();
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
                .update(|xs| xs.sort_rows(sheet_priorities_resource.get().unwrap_or_default()));
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
            .map(|x| x.to_string())
            .filter(|x| !primary_headers.contains(&x))
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
                    view! {  <h1>{move || sheet_init_memo.get().unwrap_or_default().sheet_name}</h1> }
                }
            >
                <input
                    type="text"
                    class="centered-input"
                    placeholder=move || {
                        format!(
                            "{} ({})", "اسم الشيت", sheet_init_memo.get().unwrap_or_default()
                            .sheet_name
                        )
                    }
                    value=move || sheet_name.get()
                    on:input=move |ev| sheet_name.set(event_target_value(&ev))
                />
            </Show>
        <PrimaryRow
          columns=primary_row_columns
          non_primary_headers=primary_row_non_primary_headers
          new_columns=modified_primary_columns
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
                    priorities=move || sheet_priorities_resource.get().unwrap_or_default()
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
    BH: Fn() -> Vec<String> + 'static + Clone + Copy,
    CH: Fn() -> Vec<String> + 'static + Clone + Copy,
    FR: Fn() -> Vec<Row> + 'static + Clone + Copy,
    ID: Fn(Uuid) -> bool + 'static + Clone + Copy,
    FD: Fn(Uuid) + 'static + Clone + Copy,
    FI: Fn() -> Uuid + 'static + Clone + Copy,
{
    let edit_column = RwSignal::from(None::<ColumnIdentity>);
    let new_rows = create_memo(move |_| {
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
            view=move | Row { columns, id }| {
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
                                        .unwrap_or(ColumnValue::String(Some("Empty".to_string())))
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
