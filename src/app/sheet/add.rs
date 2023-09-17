use leptos::*;
use leptos_router::*;
use models::{Column, ConfigValue, HeaderGetter, Row, RowsSort};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::shared::{
    alert, import_sheet_rows, message, new_id, open_file, InputRow, NameArg, SheetHead, ShowNewRows,
};

use std::collections::HashMap;

use crate::{
    app::sheet::shared::{merge_primary_row_headers, PrimaryRowContent, PrimaryRowEditor},
    Id,
};
use tauri_sys::tauri::invoke;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct SaveSheetArgs {
    sheetid: Uuid,
    sheetname: Rc<str>,
    typename: Rc<str>,
    rows: Vec<Row<Rc<str>>>,
}

use std::rc::Rc;

#[component]
pub fn AddSheet() -> impl IntoView {
    let sheet_name = RwSignal::from(Rc::from(""));
    let rows = RwSignal::from(Vec::<Row<Rc<str>>>::new());
    let modified_primary_columns = RwSignal::from(HashMap::<Rc<str>, Column<Rc<str>>>::new());
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

    let append = move |row: Row<Rc<str>>| {
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
                    typename: sheet_type_name_resource.get().unwrap_or(Rc::from("")),
                    rows: the_rows
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
                    message("👍").await;
                    rows.set(Vec::new());
                    sheet_id_resource.refetch();
                    sheet_name.set(Rc::from(""));
                }
                Err(err) => alert(err.to_string().as_str()).await,
            }
        });
    };

    let sheet_primary_headers_resource = Resource::new(
        move || sheet_type_name_resource.get(),
        move |name| async move {
            invoke::<NameArg, Rc<[Rc<str>]>>("sheet_primary_headers", &NameArg { name })
                .await
                .unwrap_or(Rc::from([]))
        },
    );

    let primary_non_primary_headers = move || {
        let primary_headers = sheet_primary_headers_resource.get().unwrap_or(Rc::from([]));

        modified_primary_columns
            .get()
            .keys()
            .cloned()
            .filter(|x| !primary_headers.contains(x))
            .collect::<Rc<[_]>>()
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
                on:input=move |ev| sheet_name.set(Rc::from(event_target_value(&ev).trim()))
            />
            <PrimaryRow
              primary_headers=move || sheet_primary_headers_resource.get().unwrap_or(Rc::from([]))
              non_primary_headers=primary_non_primary_headers
              new_columns=modified_primary_columns
            />
            <br/>
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
    new_columns: RwSignal<HashMap<Rc<str>, Column<Rc<str>>>>,
) -> impl IntoView
where
    FP: Fn() -> Rc<[Rc<str>]> + 'static + Clone + Copy,
    FN: Fn() -> Rc<[Rc<str>]> + 'static + Clone + Copy,
{
    let headers = move || merge_primary_row_headers(primary_headers(), non_primary_headers());

    view! {
    <>
    <PrimaryRowContent
        headers=headers
        delete_fun=move |header| new_columns.update(|xs| xs.retain(|x,_| x.clone() != header))
        new_columns=new_columns
        columns=Memo::new(move |_| HashMap::new())
        is_in_edit_mode=move || true
        is_deleted=move |_| false
    />
        <PrimaryRowEditor new_columns=new_columns/>
    </>
    }
}
