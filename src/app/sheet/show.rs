use crate::app::sheet::shared::{
    merge_primary_row_headers, new_id, PrimaryRowContent, PrimaryRowEditor,
};
use crate::Id;
use chrono::NaiveDate;
use leptos::spawn_local;
use leptos::{ev::MouseEvent, *};
use leptos_router::*;
use models::{
    Column, ColumnValue, ConfigValue, HeaderGetter, IdentityDiffsOps, Row, RowIdentity, RowsSort,
    Sheet,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;
use std::{collections::HashMap, rc::Rc};
use tauri_sys::tauri::invoke;
use uuid::Uuid;

use super::shared::{
    alert, confirm, import_sheet_rows, message, open_file, resolve_operation, EditState, InputRow,
    Name, NameArg, SheetHead, ShowNewRows,
};

#[derive(Debug, Clone)]
struct ColumnIdentity {
    row_id: Uuid,
    header: Rc<str>,
    value: ColumnValue<Rc<str>>,
}

const FETCH_LIMIT: i64 = 7;

use itertools::Itertools;

async fn collapse_rows(
    rows: Vec<Row<Rc<str>>>,
    row_identity: RowIdentity<Rc<str>>,
    priorities: Rc<[Rc<str>]>,
) -> (Vec<Row<Rc<str>>>, HashMap<Uuid, Vec<Uuid>>) {
    let row_to_key = |x: &Row<Rc<str>>| {
        x.columns
            .get(&row_identity.id)
            .map(|x| Rc::from(x.value.to_string()))
            .unwrap_or(Rc::from(""))
    };
    fn stack_rows(rows: Vec<Row<Rc<str>>>, rows_ids_id: &Rc<str>) -> Vec<Vec<Row<Rc<str>>>> {
        let key = |x: &Row<Rc<str>>| {
            x.columns
                .get(rows_ids_id)
                .map(|x| x.value.to_string())
                .unwrap_or_default()
        };
        rows.into_iter()
            .into_group_map_by(key)
            .into_iter()
            .map(|(_, group)| group)
            .collect::<Vec<_>>()
    }

    fn column_value_from_identity(
        value: &IdentityDiffsOps,
        rows: &Vec<Row<Rc<str>>>,
        title: &Rc<str>,
    ) -> Option<ColumnValue<Rc<str>>> {
        let row_main_column_value =
            |x: &Row<Rc<str>>| x.columns.get(title).map(|x| x.value.clone());

        let map_rows_to_columns = || rows.iter().map(row_main_column_value).flatten();

        match value {
            IdentityDiffsOps::Nth(n) => rows.iter().map(row_main_column_value).flatten().nth(*n),
            IdentityDiffsOps::Sum => {
                let r = map_rows_to_columns()
                    .map(|x| {
                        if let ColumnValue::Float(n) = x {
                            n
                        } else {
                            0.0
                        }
                    })
                    .sum::<f64>();
                Some(ColumnValue::Float(r))
            }
            IdentityDiffsOps::Prod => {
                let r = map_rows_to_columns()
                    .map(|x| {
                        if let ColumnValue::Float(n) = x {
                            n
                        } else {
                            1.0
                        }
                    })
                    .product::<f64>();
                Some(ColumnValue::Float(r))
            }
            IdentityDiffsOps::Max => map_rows_to_columns()
                .map(|x| {
                    if let ColumnValue::Float(n) = x {
                        Some(n as i64)
                    } else {
                        None
                    }
                })
                .flatten()
                .max()
                .map(|x| ColumnValue::Float(x as f64)),
            IdentityDiffsOps::Min => map_rows_to_columns()
                .map(|x| {
                    if let ColumnValue::Float(n) = x {
                        Some(n as i64)
                    } else {
                        None
                    }
                })
                .flatten()
                .min()
                .map(|x| ColumnValue::Float(x as f64)),
        }
    }

    let (repeated, unique): (Vec<_>, Vec<_>) = {
        let mut unique_tester = HashSet::<Rc<str>>::new();
        let repeated = rows
            .iter()
            .map(row_to_key)
            .collect::<Vec<Rc<str>>>()
            .into_iter()
            .filter(|x| !unique_tester.insert(x.clone()))
            .collect::<Vec<_>>();
        rows.into_iter()
            .partition(|x| repeated.contains(&row_to_key(x)))
    };

    let mut collapsed_rows = Vec::<Row<Rc<str>>>::new();
    let mut collapsed_rows_ids = HashMap::<Uuid, Vec<Uuid>>::new();
    let stacked_rows = stack_rows(repeated, &row_identity.id);
    for rows in stacked_rows {
        let columns = row_identity
            .diff_ops
            .iter()
            .flat_map(
                |(title, value)| match column_value_from_identity(value, &rows, title) {
                    Some(value) => Some((
                        title.clone(),
                        Column {
                            is_basic: true,
                            value,
                        },
                    )),
                    None => None,
                },
            )
            .collect::<HashMap<_, _>>();
        let id = new_id().await;
        collapsed_rows.push(Row { id, columns });
        collapsed_rows_ids.insert(id, rows.iter().map(|x| x.id).collect::<Vec<_>>());
    }
    collapsed_rows.sort_rows(priorities);
    (
        collapsed_rows.into_iter().chain(unique).collect(),
        collapsed_rows_ids,
    )
}
#[component]
fn ColumnEdit<F1, F2, F3>(mode: F1, cancel: F2, push_to_modified: F3) -> impl IntoView
where
    F1: Fn() -> ColumnIdentity + 'static,
    F2: Fn() + 'static + Clone + Copy,
    F3: Fn(ColumnIdentity) + 'static,
{
    let column_value = RwSignal::from(mode().value);
    let on_input = move |ev| {
        let value = event_target_value(&ev).trim().to_string();
        let value = match column_value.get() {
            ColumnValue::Float(_) => ColumnValue::Float(value.parse().unwrap_or_default()),
            ColumnValue::Date(_) => ColumnValue::Date(Some(value.parse().unwrap_or_default())),
            _ => ColumnValue::String(Some(Rc::from(value))),
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

#[component]
pub fn ShowSheet() -> impl IntoView {
    let edit_mode = RwSignal::from(EditState::None);
    let sheet_name = RwSignal::<Rc<str>>::from(Rc::from(""));
    let deleted_rows = RwSignal::from(Vec::<Uuid>::new());
    let added_rows = RwSignal::from(Vec::<Row<Rc<str>>>::new());
    let modified_columns = RwSignal::from(Vec::<ColumnIdentity>::new());
    let modified_primary_columns = RwSignal::from(HashMap::<Rc<str>, Column<Rc<str>>>::new());
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
            invoke::<NameArg, RowIdentity<Rc<str>>>("get_rows_ids", &NameArg { name })
                .await
                .unwrap_or(RowIdentity {
                    id: Rc::from(""),
                    diff_ops: HashMap::new(),
                })
        },
    );

    let sheet_id = move || {
        params.with(|params| match params.get("sheet_id") {
            Some(id) => Uuid::from_str(id).ok(),
            None => None,
        })
    };

    let sheet_resource = Resource::once(move || async move {
        invoke::<Id, (Sheet<Rc<str>>, i64)>("get_sheet", &Id { id: sheet_id() })
            .await
            .ok()
    });

    let rows_offset = RwSignal::from(0);

    let rows_number = Memo::new(move |_| {
        let default = FETCH_LIMIT * 4;
        sheet_resource
            .get()
            .map(|x| x.map(|x| x.1).unwrap_or(default))
            .unwrap_or(default)
    });

    let rows_accumalator = RwSignal::from(Vec::<Row<Rc<str>>>::new());
    let rows_collapser = RwSignal::from(Vec::<Row<Rc<str>>>::new());
    let rows_collapsed_ids = RwSignal::from(HashMap::<Uuid, Vec<Uuid>>::new());
    let rows_updates = RwSignal::from(HashMap::<Uuid, i32>::new());

    const RENDER_EVERY_CALLS_NUMBER: i64 = 3;

    let is_collapsable = move || {
        !rows_ids_resource
            .get()
            .map(|x| x.id)
            .unwrap_or(Rc::from(""))
            .is_empty()
    };

    let sheet_rows_resource = Resource::new(
        move || rows_offset.get(),
        move |offset| async move {
            #[derive(Serialize, Deserialize)]
            struct LimitedId {
                id: Option<Uuid>,
                offset: i64,
                limit: i64,
            }

            let rows_number = rows_number.get();
            let new_rows = invoke::<_, Vec<Row<Rc<str>>>>(
                "get_sheet_rows",
                &LimitedId {
                    id: sheet_id(),
                    offset,
                    limit: FETCH_LIMIT,
                },
            )
            .await
            .unwrap_or_default();

            if offset <= rows_number {
                rows_offset.update(|x| *x += FETCH_LIMIT);
            } else {
                let sheet_priorities = sheet_priorities_resource.get().unwrap_or(Rc::from([]));

                rows_accumalator.update(|xs| xs.sort_rows(sheet_priorities));
            }
            if !new_rows.is_empty() {
                if offset % RENDER_EVERY_CALLS_NUMBER == 0 || offset <= FETCH_LIMIT {
                    rows_accumalator.update(|xs| xs.extend(new_rows));
                } else {
                    rows_accumalator.update_untracked(|xs| xs.extend(new_rows));
                }
            }
        },
    );

    Effect::new(move |_| {
        logging::log!("ran");
        if is_collapsable() && rows_offset.get() > rows_number.get() {
            logging::log!("ran and done");
            let sheet_priorities = sheet_priorities_resource.get().unwrap_or(Rc::from([]));
            let row_identity = rows_ids_resource.get().unwrap_or(RowIdentity {
                id: Rc::from(""),
                diff_ops: HashMap::new(),
            });
            spawn_local(async move {
                let (collapsed_rows, collapsed_rows_ids) = collapse_rows(
                    rows_accumalator.get(),
                    row_identity,
                    sheet_priorities.clone(),
                )
                .await;
                rows_collapser.set(collapsed_rows);
                rows_collapsed_ids.set(collapsed_rows_ids);
            });
        }
    });

    let get_initial_sheet = move || {
        let s = sheet_resource.get().map(|x| x.map(|x| x.0));
        match s {
            Some(Some(s)) => Some(s),
            _ => None,
        }
    };

    #[derive(Clone)]
    enum RenderMode {
        Accumalate,
        Collapse,
        None,
    }
    let render_mode = RwSignal::new(RenderMode::None);

    #[component]
    fn CollapseButton<F>(render_mode: RwSignal<RenderMode>, is_collapsble: F) -> impl IntoView
    where
        F: Fn() -> bool + 'static,
    {
        let toggle = move |_| match render_mode.get() {
            RenderMode::None | RenderMode::Accumalate => render_mode.set(RenderMode::Collapse),
            RenderMode::Collapse => render_mode.set(RenderMode::Accumalate),
        };
        view! {
            <Show when=is_collapsble fallback=|| view! {<></>}>
                <button on:click=toggle>"<>"</button>
            </Show>
        }
    }

    let get_rendered_rows = move || {
        sheet_rows_resource.get();
        let offset = rows_offset.get();
        let rows_number = rows_number.get();

        match render_mode.get() {
            RenderMode::None => (),
            RenderMode::Accumalate => return rows_accumalator.get(),
            RenderMode::Collapse => return rows_collapser.get(),
        }

        if offset <= rows_number {
            return rows_accumalator.get();
        }

        if is_collapsable() {
            rows_collapser.get()
        } else {
            rows_accumalator.get()
        }
    };

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
            invoke::<NameArg, Rc<[Rc<str>]>>("sheet_primary_headers", &NameArg { name })
                .await
                .unwrap_or(Rc::from([]))
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
    let sheet_rows_with_primary_row_with_calc_values = Memo::new(move |_| {
        let sheet_id = get_initial_sheet().map(|x| x.id).unwrap_or_default();
        let c_cols = calc_columns.get();
        get_rendered_rows()
            .into_iter()
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
                            .filter(|x| x.header == header.to_string())
                            .collect::<Vec<_>>();
                        let value = value.first().unwrap();

                        if id != sheet_id {
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
            .collect::<Vec<_>>()
    });

    let sheet_rows_without_primary_row_with_calc_values = Memo::new(move |_| {
        let sheet_id = get_initial_sheet().map(|x| x.id).unwrap_or_default();
        sheet_rows_with_primary_row_with_calc_values
            .get()
            .into_iter()
            .filter(|x| x.id != sheet_id)
            .collect::<Vec<_>>()
    });

    let export = move |_| {
        let mut sheet = match get_initial_sheet() {
            Some(x) => x,
            None => Sheet {
                id: Uuid::nil(),
                sheet_name: Rc::from(""),
                type_name: Rc::from(""),
                insert_date: NaiveDate::default(),
                rows: vec![],
            },
        };
        sheet.rows = sheet_rows_with_primary_row_with_calc_values.get();
        spawn_local(async move {
            #[derive(Serialize, Deserialize)]
            struct Args {
                headers: Vec<Rc<str>>,
                sheet: Sheet<Rc<str>>,
            }
            match invoke::<_, ()>(
                "export_sheet",
                &Args {
                    sheet,
                    headers: basic_headers()
                        .into_iter()
                        .chain(calc_headers())
                        .collect::<Vec<Rc<str>>>(),
                },
            )
            .await
            {
                Ok(_) => message("🏹 👍").await,
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

    let revert_all_edits = move || {
        edit_mode.set(EditState::None);
        sheet_name.set(Rc::from(""));
        deleted_rows.set(Vec::new());
        added_rows.set(Vec::new());
        modified_columns.set(Vec::new());
        modified_primary_columns.set(HashMap::new());
        deleted_primary_columns.set(Vec::new());
    };

    let cancel_edit = move || {
        spawn_local(async move {
            let reset = if has_anything_changed() {
                confirm("سيتم تجاهل كل التعديلات").await
            } else {
                true
            };
            if reset {
                revert_all_edits();
            }
        });
    };
    let append = move |row| {
        added_rows.update_untracked(|xs| xs.push(row));
        added_rows
            .update(|xs| xs.sort_rows(sheet_priorities_resource.get().unwrap_or(Rc::from([]))));
    };
    let primary_row_columns = Memo::new(move |_| {
        let Some(id) = get_initial_sheet().map(|x| x.id) else {
            return HashMap::new();
        };
        get_rendered_rows()
            .into_iter()
            .filter(|x| x.id == id)
            .collect::<Vec<_>>()
            .first()
            .map(|x| x.columns.clone())
            .unwrap_or_default()
    });

    const SAVE_EDITS_TOTAL_TASKS: i32 = 6;

    let save_edits_successes = RwSignal::from(0);
    let save_edits_dones = RwSignal::from(0);

    let save_edits = move |_| {
        let Some((sheetid, sheetname)) = get_initial_sheet().map(|x| (x.id, x.sheet_name)) else {
            return;
        };
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
                get_rendered_rows()
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
                get_rendered_rows()
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
            .map(|x| (sheetid, x))
            .collect::<Vec<_>>();

        fn spawn_my_local_process<T: Serialize + 'static>(
            it_worth_it: bool,
            operation: &'static str,
            args: T,
            save_edits_successes: RwSignal<i32>,
            save_edits_dones: RwSignal<i32>,
        ) {
            spawn_local(async move {
                if it_worth_it {
                    match invoke::<_, ()>(operation, &args).await {
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
        }

        #[derive(Serialize, Deserialize)]
        struct SheetNameArg {
            name: Name,
        }
        spawn_my_local_process(
            !the_sheet_name.is_empty() && the_sheet_name != sheetname,
            "update_sheet_name",
            SheetNameArg {
                name: Name {
                    id: sheetid,
                    the_name: the_sheet_name,
                },
            },
            save_edits_successes,
            save_edits_dones,
        );

        #[derive(Serialize, Deserialize)]
        struct RowsDeleteArg {
            sheetid: Uuid,
            rowsids: Vec<Uuid>,
        }
        spawn_my_local_process(
            !the_deleted_rows.is_empty(),
            "delete_rows_from_sheet",
            RowsDeleteArg {
                sheetid,
                rowsids: the_deleted_rows,
            },
            save_edits_successes,
            save_edits_dones,
        );

        #[derive(Serialize, Deserialize)]
        struct RowsAddArg {
            sheetid: Uuid,
            rows: Vec<Row<Rc<str>>>,
        }
        spawn_my_local_process(
            !the_added_rows.is_empty(),
            "add_rows_to_sheet",
            RowsAddArg {
                sheetid,
                rows: the_added_rows,
            },
            save_edits_successes,
            save_edits_dones,
        );
        #[derive(Serialize, Deserialize)]
        struct DeleteColumnsArgs {
            sheetid: Uuid,
            rowsheaders: Vec<(Uuid, Rc<str>)>,
        }
        spawn_my_local_process(
            !primary_deleted_columnsidentifiers.is_empty(),
            "delete_columns",
            DeleteColumnsArgs {
                sheetid,
                rowsheaders: primary_deleted_columnsidentifiers,
            },
            save_edits_successes,
            save_edits_dones,
        );

        #[derive(Serialize, Deserialize)]
        struct UpdateColumnsArgs {
            sheetid: Uuid,
            columnsidentifiers: Vec<(Uuid, Rc<str>, ColumnValue<Rc<str>>)>,
        }

        spawn_my_local_process(
            !updated_columnsidentifiers.is_empty(),
            "update_columns",
            UpdateColumnsArgs {
                sheetid,
                columnsidentifiers: updated_columnsidentifiers,
            },
            save_edits_successes,
            save_edits_dones,
        );
        spawn_my_local_process(
            !new_columnsidentifiers.is_empty(),
            "save_columns",
            UpdateColumnsArgs {
                sheetid,
                columnsidentifiers: new_columnsidentifiers,
            },
            save_edits_successes,
            save_edits_dones,
        );
    };

    Effect::new(move |_| {
        if save_edits_successes.get() == SAVE_EDITS_TOTAL_TASKS {
            spawn_local(async move {
                message("👍").await;
                save_edits_successes.set(0);
            });
        }
    });

    let patch_changes = move || {
        let sheet_id = sheet_resource
            .get()
            .map(|x| x.map(|x| x.0.id).unwrap_or_default())
            .unwrap_or_default();
        rows_accumalator.update_untracked(|xs| {
            xs.extend(added_rows.get());
            let deleted = deleted_rows.get();
            xs.retain(|x| !deleted.contains(&x.id));
            let rows: HashMap<Uuid, Vec<ColumnIdentity>> = {
                let modified_columns = modified_columns.get().into_iter().chain(
                    modified_primary_columns
                        .get()
                        .into_iter()
                        .map(|(header, column)| ColumnIdentity {
                            row_id: sheet_id,
                            header,
                            value: column.value,
                        })
                        .collect::<Vec<_>>(),
                );
                let mut rows: HashMap<Uuid, Vec<ColumnIdentity>> = HashMap::new();
                for column in modified_columns {
                    let row_id = column.row_id;
                    if let Some(list) = rows.get(&column.row_id) {
                        let list = list
                            .iter()
                            .chain(&vec![column])
                            .cloned()
                            .collect::<Vec<_>>();
                        rows.insert(row_id, list);
                    } else {
                        rows.insert(row_id, vec![column]);
                    }
                }
                rows
            };
            for (id, columns) in rows {
                if let Some(position) = xs.iter().position(|x| x.id == id) {
                    if let Some(row) = xs.get_mut(position) {
                        row.columns = row
                            .columns
                            .clone()
                            .into_iter()
                            .map(|(header, target_column)| {
                                if let Some(position) =
                                    columns.iter().position(|x| x.header == header)
                                {
                                    if let Some(column) = columns.get(position) {
                                        if let Some(update) = rows_updates.get().get(&id) {
                                            rows_updates.update(|x| {
                                                x.insert(id, update.to_owned() + 1);
                                            });
                                        } else {
                                            rows_updates.update(|x| {
                                                x.insert(id, 1);
                                            });
                                        }

                                        (
                                            header,
                                            Column {
                                                is_basic: true,
                                                value: column.value.clone(),
                                            },
                                        )
                                    } else {
                                        (header, target_column)
                                    }
                                } else {
                                    (header, target_column)
                                }
                            })
                            .collect();
                    };
                };
            }
        });
        rows_accumalator
            .update(|xs| xs.sort_rows(sheet_priorities_resource.get().unwrap_or(Rc::from([]))));
    };

    Effect::new(move |_| {
        if save_edits_dones.get() == SAVE_EDITS_TOTAL_TASKS {
            patch_changes();
            sheet_resource.refetch();
            rows_updates.update(|xs| *xs = xs.iter().map(|(id, num)| (*id, *num + 1)).collect());
            revert_all_edits();
            on_edit.set(false);
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
            let sheet_id = get_initial_sheet().map(|x| x.id).unwrap_or_default();
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
        let primary_headers = sheet_primary_headers_resource.get().unwrap_or(Rc::from([]));

        modified_primary_columns
            .get()
            .into_iter()
            .chain(primary_row_columns.get())
            .collect::<HashMap<_, _>>()
            .keys()
            .cloned()
            .filter(|x| !primary_headers.contains(x))
            .collect::<Rc<[_]>>()
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
            <CollapseButton render_mode=render_mode is_collapsble=is_collapsable/>
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
                    view! {<h1>{move || get_initial_sheet().map(|x| x.sheet_name.to_string())}</h1> }
                }
            >
                <input
                    type="text"
                    class="centered-input"
                    placeholder=move || {
                        format!(
                            "{} ({})", "اسم الشيت", get_initial_sheet().map(|x| x.sheet_name.to_string()).unwrap_or_default()
                        )
                    }
                    value=move || sheet_name.get().to_string()
                    on:input=move |ev| sheet_name.set(Rc::from(event_target_value(&ev).trim()))
                />
            </Show>
        <br/>
        <br/>
        <PrimaryRow
          columns=primary_row_columns
          new_columns=modified_primary_columns
          deleted_columns=deleted_primary_columns
          primary_headers=move || sheet_primary_headers_resource.get().unwrap_or(Rc::from([]))
          non_primary_headers=primary_row_non_primary_headers
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
                        rows_updates=rows_updates
                        rows=sheet_rows_without_primary_row_with_calc_values
                        edit_mode=edit_mode
                        is_deleted=is_deleted
                        modified_columns=modified_columns
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
                        sheet_id=move ||get_initial_sheet().map(|x| x.id).unwrap_or_default()
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
fn ShowRows<BH, CH, FD, ID>(
    basic_headers: BH,
    calc_headers: CH,
    delete_row: FD,
    is_deleted: ID,
    rows_updates: RwSignal<HashMap<Uuid, i32>>,
    rows: Memo<Vec<Row<Rc<str>>>>,
    edit_mode: RwSignal<EditState>,
    modified_columns: RwSignal<Vec<ColumnIdentity>>,
) -> impl IntoView
where
    BH: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    CH: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    ID: Fn(Uuid) -> bool + 'static + Clone + Copy,
    FD: Fn(Uuid) + 'static + Clone + Copy,
{
    let edit_column = RwSignal::from(None::<ColumnIdentity>);

    let get_row_id = move |id: Uuid| {
        id.to_string()
            + rows_updates
                .get()
                .get(&id)
                .map(|x| x.to_string())
                .unwrap_or_default()
                .as_str()
    };
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
            each=move || rows.get()
            key=move |row| get_row_id(row.id)
            children=move | Row { columns, id }| {
                let columns = std::rc::Rc::new(columns);
                view! {
                    <tr>
                        {
                            let columns = columns.clone();
                            view! {
                                <For
                                    each=basic_headers
                                    key=|key| key.clone()
                                    children=move |header0| {
                            let header1 = header0.clone();
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
                                        .unwrap_or(ColumnValue::String(Some(Rc::from("Empty"))))
                            }))
                         }
                         >{
                        move || columns2
                        .get(&header0)
                        .map(|x| x.value.to_string())
                        } {
                        move || modified_columns.get()
                        .into_iter().filter(|x| x.row_id == id && x.header ==header3)
                        .collect::<Vec<_>>()
                        .first()
                        .map(|x| format!(" > {}",x.value.to_string()))
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
                                    children=move | column| {
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

#[component]
fn PrimaryRow<FP, FN>(
    primary_headers: FP,
    non_primary_headers: FN,
    columns: Memo<HashMap<Rc<str>, Column<Rc<str>>>>,
    new_columns: RwSignal<HashMap<Rc<str>, Column<Rc<str>>>>,
    deleted_columns: RwSignal<Vec<Rc<str>>>,
    edit_mode: RwSignal<EditState>,
) -> impl IntoView
where
    FP: Fn() -> Rc<[Rc<str>]> + 'static + Clone + Copy,
    FN: Fn() -> Rc<[Rc<str>]> + 'static + Clone + Copy,
{
    let headers = move || merge_primary_row_headers(primary_headers(), non_primary_headers());

    let is_in_edit_mode = move || matches!(edit_mode.get(), EditState::Primary);
    let is_deleted = move |header| deleted_columns.get().into_iter().any(|x| x == header);
    let is_new = move |header| new_columns.get().keys().any(|x| x.clone() == header);

    let delete_fun = move |p: Rc<str>| {
        if is_new(p.clone()) {
            new_columns.update(|xs| xs.retain(|x, _| x.clone() != p));
        } else if is_deleted(p.clone()) {
            deleted_columns.update(|xs| xs.retain(|x| x.clone() != p.clone()))
        } else {
            deleted_columns.update(|xs| {
                if !p.is_empty() {
                    xs.push(p.clone())
                }
            })
        }
    };

    view! {
    <>
    <PrimaryRowContent
        headers=headers
        is_in_edit_mode=is_in_edit_mode
        is_deleted=is_deleted
        delete_fun=delete_fun
        columns=columns
        new_columns=new_columns
    />
    <Show
        when=is_in_edit_mode
        fallback=move|| view!{<></>}
    >
        <PrimaryRowEditor
            new_columns=new_columns
        />
    </Show>
    </>
    }
}
