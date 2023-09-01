use leptos::*;
use leptos_router::*;
use models::{Column, ColumnValue, ConfigValue, HeaderGetter, Row, RowsSort};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::shared::{
    alert, import_sheet_rows, message, new_id, open_file, InputRow, NameArg, SheetHead, ShowNewRows,
};

use chrono::Local;
use std::collections::HashMap;

use crate::Id;
use tauri_sys::tauri::invoke;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct SaveSheetArgs {
    sheetid: Uuid,
    sheetname: String,
    typename: String,
    rows: Vec<Row>,
}

#[component]
pub fn AddSheet() -> impl IntoView {
    let (sheet_name, set_sheet_name) = create_signal(String::from(""));
    let (rows, set_rows) = create_signal(Vec::new());
    let (modified_primary_columns, set_modified_primary_columns) =
        create_signal(HashMap::<String, Column>::new());
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

    let sheet_headers_resource = create_resource(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Vec<ConfigValue>>("sheet_headers", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );
    let sheet_id_resource = create_resource(|| (), move |_| async move { new_id().await });
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

    let append = move |row: Row| {
        set_rows.update(|xs| {
            xs.push(row);
            xs.sort_rows(sheet_priorities_resource.get().unwrap_or_default());
        })
    };

    let delete_row = move |id: Uuid| set_rows.update(|xs| xs.retain(|x| x.id != id));

    let save_sheet = move |_| {
        let id = sheet_id_resource.get().unwrap_or_default();
        spawn_local(async move {
            let mut rows = rows.get();
            let index = rows.iter().position(|x| x.id == id);
            if let Some(index) = index {
                if let Some(primary_row) = rows.get_mut(index) {
                    *primary_row = Row {
                        id: primary_row.id,
                        columns: primary_row
                            .clone()
                            .columns
                            .into_iter()
                            .chain(modified_primary_columns.get())
                            .collect(),
                    };
                };
            }
            match invoke::<_, ()>(
                "save_sheet",
                &SaveSheetArgs {
                    sheetid: id,
                    sheetname: sheet_name.get(),
                    typename: sheet_type_name_resource.get().unwrap_or_default(),
                    rows: rows
                        .into_iter()
                        .map(|Row { id, columns }| Row {
                            id,
                            columns: columns
                                .into_iter()
                                .filter(|(_, Column { is_basic, value: _ })| is_basic.to_owned())
                                .collect(),
                        })
                        .collect::<Vec<_>>(),
                },
            )
            .await
            {
                Ok(_) => {
                    sheet_id_resource.refetch();
                    set_rows.set(Vec::new());
                    message("نجح الحفظ").await
                }
                Err(err) => alert(err.to_string().as_str()).await,
            }
        });
    };

    let sheet_primary_headers_resource = create_resource(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Vec<String>>("sheet_primary_headers", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );

    let primary_non_primary_headers = move || {
        let primary_headers = sheet_primary_headers_resource.get().unwrap_or_default();

        modified_primary_columns
            .get()
            .keys()
            .map(|x| x.to_string())
            .filter(|x| !primary_headers.contains(&x))
            .collect::<Vec<_>>()
    };

    let load_file = move |_| {
        let sheettype = sheet_type_name_resource.get().unwrap_or_default();
        spawn_local(async move {
            let Some(filepath) = open_file().await else {
                return;
            };
            let rows = import_sheet_rows(
                sheet_id_resource.get().unwrap_or_default(),
                sheettype,
                filepath,
            )
            .await;
            let primary_row = rows
                .iter()
                .filter(|x| x.id == sheet_id_resource.get().unwrap_or_default())
                .collect::<Vec<_>>();
            let primary_row = primary_row
                .first()
                .map(|x| x.columns.clone())
                .unwrap_or_default();
            for (header, column) in primary_row {
                set_modified_primary_columns.update(|map| {
                    map.insert(header, column);
                })
            }
            set_rows.update(|xs| {
                xs.extend(rows);
                xs.sort_rows(sheet_priorities_resource.get().unwrap_or_default());
            });
        });
    };

    view! {
        <section>
            <A class="left-corner" href=format!("/sheet/{}", sheet_type_id().unwrap_or_default())>
                "->"
            </A>
            <br/>
            <input
                type="text"
                class="centered-input"
                placeholder=move || {
                    format!(
                        "{} ({})", "اسم الشيت", sheet_type_name_resource.get()
                        .unwrap_or_default()
                    )
                }
                value=move || sheet_name.get()
                on:input=move |ev| set_sheet_name.set(event_target_value(&ev))
            />
            <PrimaryRow
              primary_headers=move || sheet_primary_headers_resource.get().unwrap_or_default()
              non_primary_headers=primary_non_primary_headers
              new_columns=modified_primary_columns
              set_new_columns=set_modified_primary_columns
            /><br/>
            <table>
                <SheetHead basic_headers=basic_headers calc_headers=calc_headers/>
                <tbody>
                    <ShowNewRows
                        delete_row=delete_row
                        basic_headers=basic_headers
                        calc_headers=calc_headers
                        rows=rows
                        set_rows=set_rows
                    sheet_id=move || sheet_id_resource.get().unwrap_or_default()
                    priorities=move || sheet_priorities_resource.get().unwrap_or_default()
                    />
                    <InputRow
                        basic_headers=basic_headers
                        calc_headers=calc_headers
                        append=append
                        basic_columns=basic_columns
                        calc_columns=calc_columns
                    />
                </tbody>
            </table>
            <button on:click=load_file class="centered-button">
                "تحميل ملف"
            </button>
            <button on:click=save_sheet class="centered-button">
                "حفظ الشيت"
            </button>
            <Outlet/>
        </section>
    }
}

#[component]
fn PrimaryRow<FP, FN>(
    primary_headers: FP,
    non_primary_headers: FN,
    new_columns: ReadSignal<HashMap<String, Column>>,
    set_new_columns: WriteSignal<HashMap<String, Column>>,
) -> impl IntoView
where
    FP: Fn() -> Vec<String> + 'static + Clone + Copy,
    FN: Fn() -> Vec<String> + 'static + Clone + Copy,
{
    let (add_what, set_add_what) = create_signal(None::<&str>);
    let (header, set_header) = create_signal(String::from(""));
    let (column_value, set_column_value) = create_signal(ColumnValue::Float(0.0));

    let headers = move || {
        let mut primary_headers = primary_headers();

        let mut non_primary_headers = non_primary_headers();

        let space = non_primary_headers.len() as i32 - primary_headers.len() as i32;

        if space > 0 {
            primary_headers.extend((0..space).map(|_| "".to_string()));
        } else if space < 0 {
            let space = space * -1;
            non_primary_headers.extend((0..space).map(|_| "".to_string()));
        }

        primary_headers
            .into_iter()
            .zip(non_primary_headers)
            .collect::<Vec<_>>()
    };

    let on_value_input = move |ev| {
        set_column_value.update(|x| match x {
            ColumnValue::String(_) => *x = ColumnValue::String(Some(event_target_value(&ev))),
            ColumnValue::Date(_) => {
                *x = ColumnValue::Date(Some(event_target_value(&ev).parse().unwrap_or_default()))
            }
            ColumnValue::Float(_) => {
                *x = ColumnValue::Float(event_target_value(&ev).parse().unwrap_or_default())
            }
        })
    };

    let append = move |_| {
        set_new_columns.update(|map| {
            map.insert(
                header.get(),
                Column {
                    is_basic: true,
                    value: column_value.get(),
                },
            );
        });
        set_add_what.set(None);
    };

    view! {
    <>
    <table>
        <For
        each=move || headers()
        key=|x| x.0.clone() + &x.1
        view=move |(primary,non_primary)| view!{
            <tr>
            <td>{primary.clone()}</td>
            <td class="shapeless">" "</td>
            <td>{move || new_columns
                 .get()
                 .get(&primary)
                 .map(|x| x.value.to_string()
                  + &new_columns
                  .get()
                  .get(&primary)
                  .map(|x| x.value.to_string())
                  .unwrap_or_default()
                 )
            }
            </td>
            <td class="shapeless">" "</td>
            <td class="shapeless">" | "</td>
            <td class="shapeless">" "</td>
            <td>{non_primary.clone()}</td>
            <td class="shapeless">" "</td>
            <td>{move ||new_columns.get().get(&non_primary).map(|x| x.value.to_string())}</td>
            </tr>
        }
        />
    </table>
    <Show
        when=move || add_what.get().is_some()
        fallback=move|| view!{
        <>
            <button
            class="centered-button"
            on:click=move |_| {
            set_add_what.set(Some("date"));
            set_column_value.set(ColumnValue::Date(Some(Local::now().date_naive())))
            }
            >"+ تاريخ"</button>
            <button
            class="centered-button"
            on:click=move |_| {
            set_add_what.set(Some("number"));
            set_column_value.set(ColumnValue::Float(0.0));
            }
            >"+ رقم"</button>
            <button
            class="centered-button"
            on:click=move |_| {
            set_add_what.set(Some("text"));
            set_column_value.set(ColumnValue::String(Some("".to_string())));
            }
            >"+ نص"</button>
            <button
            class="centered-button"
            on:click=move |_| {
            set_new_columns.set(HashMap::new());
            }
            >"الغاء التعديلات"</button>
        </>
        }
        >
            <div>
            <input
            style="width:40%; height:30px;"
            type="text"
            placeholder="العنوان"
                on:input=move |ev| set_header.set(event_target_value(&ev))
            />
            <input
            style="width:40%; height:30px;"
            type=add_what.get().unwrap_or_default()
            placeholder="القيمة"
                on:input=on_value_input
            />
            </div>
            <br/>
        <button
            on:click=append
        class="centered-button"
        >"تاكيد"</button>
        <button
        class="centered-button"
           on:click=move |_| set_add_what.set(None)
        >"الغاء"</button>
    </Show>
    </>
    }
}
