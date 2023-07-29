use leptos::*;
use leptos_router::*;
use models::{Sheet, Row};
use serde::{Serialize, Deserialize};
use std::str::FromStr;
use uuid::Uuid;
use tauri_sys::tauri::invoke;
use crate::Id;
use models::{Column,ColumnValue,ConfigValue,HeaderGetter};
use std::collections::HashMap;

use super::shared::{NameArg,SheetHead,alert,message,calculate_operation};

#[derive(Serialize,Deserialize)]
struct ExportSheetArg{
    headers : Vec<String>,
    sheet : Sheet,
}

#[component]
pub fn ShowSheet(cx: Scope) -> impl IntoView {
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
            invoke::<Id, String>("sheet_type_name", &Id { id: sheet_type_id() })
                .await
                .unwrap_or_default()
        },
    );
    let sheet_id = move || {
        params.with(|params| match params.get("sheet_id") {
            Some(id) => Uuid::from_str(id).ok(),
            None => None,
        })
    };

    let sheet_resource = create_resource(
        cx,
        || (),
        move |_| async move {
            invoke::<Id, Sheet>("get_sheet", &Id { id: sheet_id() })
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

    let sheet = create_memo(cx,move |_| {
	let c_cols= calc_columns.get();
	let mut sheet = sheet_resource
	.read(cx)
	.unwrap_or_default();
	sheet.rows = sheet.rows
	.into_iter()
	.map(|Row { id, columns }| Row{
	    id,
	    columns : {
		let mut columns = columns;
		for header in calc_headers().into_iter(){
		    let mut map = HashMap::new();
		    for (col_header,Column { is_basic:_, value }) in &columns{
			map.insert(col_header.clone(), value.clone());
		    }
		    let value = &c_cols;
		    let value = value
			.iter().filter(|x| x.header == header)
			.collect::<Vec<_>>();
		    let value = value.first().unwrap();
		   
		    columns.insert(header, Column {
			is_basic: false,
			value: ColumnValue::Float(calculate_operation(&value.value, map))
		    });
		}
		columns
	    }
	}).collect::<Vec<_>>();
	sheet
    });

    let sheet_rows = create_memo(cx,move |_| sheet.get().rows);

    let export =move |_| {
	spawn_local(async move {
	    match invoke::<_, ()>("export_sheet", &ExportSheetArg {
		sheet: sheet.get(),
		headers: basic_headers().into_iter().chain(calc_headers()).collect()
	    })
	    .await {
		Ok(_) => message("نجح التصدير").await,
		Err(err) => alert(err.to_string().as_str()).await,
	    }
	    
	})
    };

    let delete_row = |_| {};

    view! { cx,
        <section>
            <A class="left-corner" href=format!("/sheet/{}", sheet_type_id().unwrap_or_default())>
                "->"
            </A>
            <button class="right-corner" on:click=export>
                "🏹"
            </button>
	    <br/>
	    <br/>
            <h1>{move || sheet_resource.read(cx).unwrap_or_default().sheet_name} </h1>
	    <table>
		<SheetHead basic_headers=basic_headers calc_headers=calc_headers/>
                <tbody>
                    <ShowRows
                        delete_row=delete_row
                        basic_headers=basic_headers
                        calc_headers=calc_headers
                        rows=sheet_rows
                    />
                </tbody>
	    </table>
            <Outlet/>
        </section>
    }
}

#[component]
fn ShowRows<BH, CH, FD>(
    cx: Scope,
    basic_headers: BH,
    calc_headers: CH,
    delete_row: FD,
    rows: Memo<Vec<Row>>,
) -> impl IntoView
where
    BH: Fn() -> Vec<String> + 'static + Clone + Copy,
    CH: Fn() -> Vec<String> + 'static + Clone + Copy,
    FD: Fn(Uuid) + 'static + Clone + Copy,
{
    view! { cx,
        <For
            each=move || rows.get()
            key=|row| row.id
            view=move |cx, Row { columns, id }| {
                view! { cx,
                    <tr>
                        <For
                            each=move || basic_headers().into_iter().chain(calc_headers())
                            key=|key| key.clone()
                            view=move |cx, column| {
                                view! { cx, <td>{columns.get(&column).map(|x| x.value.to_string())}</td> }
                            }
                        />
                        <td>
                            <button on:click=move |_| delete_row(id)>"X"</button>
                        </td>
                    </tr>
                }
            }
        />
    }
}
