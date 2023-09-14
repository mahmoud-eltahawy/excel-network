use leptos::*;
use leptos_router::*;
use models::{
    ConfigValue, FrontendColumn, FrontendColumnValue, FrontendRow, HeaderGetter, RowsSort,
};
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
    sheetname: Rc<str>,
    typename: Rc<str>,
    rows: Vec<FrontendRow>,
}

use std::rc::Rc;

#[component]
pub fn AddSheet() -> impl IntoView {
    let sheet_name = RwSignal::from(Rc::from(""));
    let rows = RwSignal::from(Vec::<FrontendRow>::new());
    let modified_primary_columns = RwSignal::from(HashMap::<Rc<str>, FrontendColumn>::new());
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

    let sheet_headers_resource = Resource::new(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Vec<ConfigValue>>("sheet_headers", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );
    let sheet_id_resource = Resource::once(move || async move { new_id().await });
    let basic_columns = Memo::new(move |_| {
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

    let calc_columns = Memo::new(move |_| {
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
            .map(|x| Rc::from(x.header))
            .collect::<Vec<_>>()
    };

    let append = move |row: FrontendRow| {
        rows.update_untracked(|xs| xs.push(row));
        rows.update(|xs| xs.sort_rows(sheet_priorities_resource.get().unwrap_or(Rc::from([]))));
    };

    let delete_row = move |id: Uuid| rows.update(|xs| xs.retain(|x| x.id != id));

    let save_sheet = move |_| {
        let id = sheet_id_resource.get().unwrap_or_default();
        spawn_local(async move {
            let mut the_rows = rows.get();
            let index = the_rows.iter().position(|x| x.id == id);
            if let Some(index) = index {
                if let Some(primary_row) = the_rows.get_mut(index) {
                    *primary_row = FrontendRow {
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
                    typename: sheet_type_name_resource.get().unwrap_or(Rc::from("")),
                    rows: the_rows
                        .into_iter()
                        .map(|FrontendRow { id, columns }| FrontendRow {
                            id,
                            columns: columns
                                .into_iter()
                                .filter(|(_, FrontendColumn { is_basic, value: _ })| {
                                    is_basic.to_owned()
                                })
                                .collect(),
                        })
                        .collect::<Vec<_>>(),
                },
            )
            .await
            {
                Ok(_) => {
                    sheet_id_resource.refetch();
                    rows.set(Vec::new());
                    message("نجح الحفظ").await
                }
                Err(err) => alert(err.to_string().as_str()).await,
            }
        });
    };

    let sheet_primary_headers_resource = Resource::new(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Vec<Rc<str>>>("sheet_primary_headers", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );

    let primary_non_primary_headers = move || {
        let primary_headers = sheet_primary_headers_resource.get().unwrap_or_default();

        modified_primary_columns
            .get()
            .keys()
            .cloned()
            .filter(|x| !primary_headers.contains(&x))
            .collect::<Vec<_>>()
    };

    let load_file = move |_| {
        let sheettype = sheet_type_name_resource.get().unwrap_or(Rc::from(""));
        spawn_local(async move {
            let Some(filepath) = open_file().await else {
                return;
            };
            let the_rows = import_sheet_rows(
                sheet_id_resource.get().unwrap_or_default(),
                sheettype,
                filepath,
            )
            .await;
            let primary_row = the_rows
                .iter()
                .filter(|x| x.id == sheet_id_resource.get().unwrap_or_default())
                .collect::<Vec<_>>();
            let primary_row = primary_row
                .first()
                .map(|x| x.columns.clone())
                .unwrap_or_default();
            for (header, column) in primary_row {
                modified_primary_columns.update(|map| {
                    map.insert(header, column);
                })
            }
            rows.update_untracked(|xs| xs.extend(the_rows));
            rows.update(|xs| xs.sort_rows(sheet_priorities_resource.get().unwrap_or(Rc::from([]))));
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
                        .unwrap_or(Rc::from(""))
                    )
                }
                value=move || sheet_name.get()
                on:input=move |ev| sheet_name.set(Rc::from(event_target_value(&ev)))
            />
            <PrimaryRow
              primary_headers=move || sheet_primary_headers_resource.get().unwrap_or_default()
              non_primary_headers=primary_non_primary_headers
              new_columns=modified_primary_columns
            /><br/>
            <table>
                <SheetHead basic_headers=basic_headers calc_headers=calc_headers/>
                <tbody>
                    <ShowNewRows
                        delete_row=delete_row
                        basic_headers=basic_headers
                        calc_headers=calc_headers
                        rows=rows
                        sheet_id=move || sheet_id_resource.get().unwrap_or_default()
                        priorities=move || sheet_priorities_resource.get().unwrap_or(Rc::from([]))
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
    new_columns: RwSignal<HashMap<Rc<str>, FrontendColumn>>,
) -> impl IntoView
where
    FP: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    FN: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
{
    let add_what = RwSignal::from(None::<&str>);
    let header = RwSignal::from(Rc::from(""));
    let column_value = RwSignal::from(FrontendColumnValue::Float(0.0));

    let headers = move || {
        let mut primary_headers = primary_headers();

        let mut non_primary_headers = non_primary_headers();

        let space = non_primary_headers.len() as i32 - primary_headers.len() as i32;

        if space > 0 {
            primary_headers.extend((0..space).map(|_| Rc::from("")));
        } else if space < 0 {
            let space = space * -1;
            non_primary_headers.extend((0..space).map(|_| Rc::from("")));
        }

        primary_headers
            .into_iter()
            .zip(non_primary_headers)
            .collect::<Vec<_>>()
    };

    let on_value_input = move |ev| {
        column_value.update(|x| match x {
            FrontendColumnValue::String(_) => {
                *x = FrontendColumnValue::String(Some(Rc::from(event_target_value(&ev))))
            }
            FrontendColumnValue::Date(_) => {
                *x = FrontendColumnValue::Date(Some(
                    event_target_value(&ev).parse().unwrap_or_default(),
                ))
            }
            FrontendColumnValue::Float(_) => {
                *x = FrontendColumnValue::Float(event_target_value(&ev).parse().unwrap_or_default())
            }
        })
    };

    let append = move |_| {
        new_columns.update(|map| {
            map.insert(
                header.get(),
                FrontendColumn {
                    is_basic: true,
                    value: column_value.get(),
                },
            );
        });
        add_what.set(None);
    };

    view! {
    <>
    <table>
        <For
        each=move || headers()
        key=|x| x.0.to_string() + &x.1.to_string()
        view=move |(primary,non_primary)| view!{
            <tr>
            <td>{primary.to_string()}</td>
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
            <td>{non_primary.to_string()}</td>
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
            add_what.set(Some("date"));
            column_value.set(FrontendColumnValue::Date(Some(Local::now().date_naive())))
            }
            >"+ تاريخ"</button>
            <button
            class="centered-button"
            on:click=move |_| {
            add_what.set(Some("number"));
            column_value.set(FrontendColumnValue::Float(0.0));
            }
            >"+ رقم"</button>
            <button
            class="centered-button"
            on:click=move |_| {
            add_what.set(Some("text"));
            column_value.set(FrontendColumnValue::String(Some(Rc::from(""))));
            }
            >"+ نص"</button>
            <button
            class="centered-button"
            on:click=move |_| {
            new_columns.set(HashMap::new());
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
                on:input=move |ev| header.set(Rc::from(event_target_value(&ev)))
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
           on:click=move |_| add_what.set(None)
        >"الغاء"</button>
    </Show>
    </>
    }
}
