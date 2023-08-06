use leptos::*;
use leptos_router::*;
use models::{
    Column, ConfigValue, HeaderGetter, Row,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::shared::{
    alert, message, NameArg, SheetHead,InputRow,ShowNewRows,import_sheet_rows,open_file
};

use crate::Id;
use tauri_sys::tauri::invoke;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct SaveSheetArgs {
    sheetname: String,
    typename: String,
    rows: Vec<Row>,
}

#[component]
pub fn AddSheet(cx: Scope) -> impl IntoView {
    let (sheet_name, set_sheet_name) = create_signal(cx, String::from(""));
    let (rows, set_rows) = create_signal(cx, Vec::new());
    let params = use_params_map(cx);
    let sheet_type_id = move || {
        params.with(|params| match params.get("sheet_type_id") {
            Some(id) => Uuid::from_str(id).ok(),
            None => None,
        })
    };
    let sheet_type_name_resource = create_resource(
        cx,
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

    let sheet_headers_resource = create_resource(
        cx,
        move || sheet_type_name_resource.read(cx),
        move |name| async move {
            invoke::<NameArg, Vec<ConfigValue>>("sheet_headers", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );
    let basic_columns = create_memo(cx, move |_| {
        sheet_headers_resource
            .read(cx)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|x| match x {
                ConfigValue::Basic(conf) => Some(conf),
                ConfigValue::Calculated(_) => None,
            })
            .collect::<Vec<_>>()
    });

    let calc_columns = create_memo(cx, move |_| {
        sheet_headers_resource
            .read(cx)
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

    let append = move |row: Row| set_rows.update(|xs| xs.push(row));

    let delete_row = move |id: Uuid| set_rows.update(|xs| xs.retain(|x| x.id != id));

    let save_sheet = move |_| {
        spawn_local(async move {
            match invoke::<_, ()>(
                "save_sheet",
                &SaveSheetArgs {
                    sheetname: sheet_name.get(),
                    typename: sheet_type_name_resource.read(cx).unwrap_or_default(),
                    rows: rows
                        .get()
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
                    set_rows.set(Vec::new());
                    message("نجح الحفظ").await
                }
                Err(err) => alert(err.to_string().as_str()).await,
            }
        });
    };

    let load_file = move |_| {
	let sheettype = sheet_type_name_resource.read(cx).unwrap_or_default();
	spawn_local(async move {
	    let Some(filepath) = open_file().await else {
		return;
	    };
	    let rows = import_sheet_rows(sheettype,filepath).await;
	    set_rows.update(|xs| xs.extend(rows));
	});
    };

    view! { cx,
        <section>
            <A class="left-corner" href=format!("/sheet/{}", sheet_type_id().unwrap_or_default())>
                "->"
            </A>
            <br/>
            <input
                type="string"
                class="centered-input"
                placeholder=move || {
                    format!(
                        "{} ({})", "اسم الشيت", sheet_type_name_resource.read(cx)
                        .unwrap_or_default()
                    )
                }
                value=move || sheet_name.get()
                on:input=move |ev| set_sheet_name.set(event_target_value(&ev))
            />
            <table>
                <SheetHead basic_headers=basic_headers calc_headers=calc_headers/>
                <tbody>
                    <ShowNewRows
                        delete_row=delete_row
                        basic_headers=basic_headers
                        calc_headers=calc_headers
                        rows=rows
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

