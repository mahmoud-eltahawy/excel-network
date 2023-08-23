use crate::Id;
use leptos::*;
use leptos_router::*;
use models::{Column, ColumnValue, ConfigValue, HeaderGetter, Name, RowsSort};
use models::{Row, Sheet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tauri_sys::tauri::invoke;
use uuid::Uuid;

use super::shared::{
    alert, calculate_operation, confirm, import_sheet_rows, message, open_file, InputRow, NameArg,
    SheetHead, ShowNewRows,
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
struct RowsDeleteArg {
    sheetid: Uuid,
    rowsids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
struct RowsAddArg {
    sheetid: Uuid,
    rows: Vec<Row>,
}

#[derive(Debug,Clone)]
struct ColumnIdentity{
    row_id : Uuid,
    header: String,
    value : ColumnValue,
}

#[component]
fn ColumnEdit<F1,F2,F3>(
    mode : F1,
    cancel : F2,
    push_to_modified: F3,
) -> impl IntoView
    where
	F1 : Fn() -> ColumnIdentity + 'static,
        F2 : Fn() + 'static + Clone + Copy,
        F3 : Fn(ColumnIdentity) + 'static,
{
    let (column_value,set_column_value) = create_signal(mode().value);
    // let (top,set_top) = create_signal( None::<usize>);
    // let (down,set_down) = create_signal( None::<usize>);
    let on_input = move |ev| {
	let value = event_target_value(&ev);
	let value = match column_value.get() {
	    ColumnValue::Float(_) => ColumnValue::Float(value.parse().unwrap_or_default()),
	    ColumnValue::Date(_) => ColumnValue::Date(Some(value.parse().unwrap_or_default())),
	    _ => ColumnValue::String(Some(value)),
	};
	set_column_value.set(value);
    };

    let save = move |_| {
	let ColumnIdentity{row_id,header,value:_} = mode();
	push_to_modified(ColumnIdentity { row_id, header, value:column_value.get()});
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
            // <input
	    // 	type="number"
	    //     placeholder="لاعلي"
	    //     on:input=move |ev| set_top.set(Some(event_target_value(&ev).parse().unwrap_or_default()))
	    // />
            // <input
	    // 	type="number"
	    //     placeholder="لاسفل"
	    //     on:input=move |ev| set_down.set(Some(event_target_value(&ev).parse().unwrap_or_default()))
	    // />
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
    let (edit_mode, set_edit_mode) = create_signal(false);
    let (sheet_name, set_sheet_name) = create_signal(String::from(""));
    let (deleted_rows, set_deleted_rows) = create_signal(Vec::<Uuid>::new());
    let (added_rows, set_added_rows) = create_signal(Vec::<Row>::new());
    let (modified_columns,set_modified_columns) = create_signal(Vec::<ColumnIdentity>::new());
    create_effect(move |_| {
	log!{"{:#?}",modified_columns.get()}
    });
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
        
        move || sheet_type_name_resource.read(),
        move |name| async move {
            invoke::<NameArg, Vec<String>>("get_priorities", &NameArg { name })
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
        
        || (),
        move |_| async move {
            invoke::<Id, Sheet>("get_sheet", &Id { id: sheet_id() })
                .await
                .unwrap_or_default()
        },
    );

    let sheet_headers_resource = create_resource(
        
        move || sheet_type_name_resource.read(),
        move |name| async move {
            invoke::<NameArg, Vec<ConfigValue>>("sheet_headers", &NameArg { name })
                .await
                .unwrap_or_default()
        },
    );
    let basic_columns = create_memo( move |_| {
        sheet_headers_resource
            .read()
            .unwrap_or_default()
            .into_iter()
            .flat_map(|x| match x {
                ConfigValue::Basic(conf) => Some(conf),
                ConfigValue::Calculated(_) => None,
            })
            .collect::<Vec<_>>()
    });

    let calc_columns = create_memo( move |_| {
        sheet_headers_resource
            .read()
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

    let sheet = create_memo( move |_| {
        let c_cols = calc_columns.get();
        let mut sheet = sheet_resource.read().unwrap_or_default();
        sheet.rows = sheet
            .rows
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
                            .filter(|x| x.header == header)
                            .collect::<Vec<_>>();
                        let value = value.first().unwrap();

                        columns.insert(
                            header,
                            Column {
                                is_basic: false,
                                value: ColumnValue::Float(calculate_operation(&value.value, map)),
                            },
                        );
                    }
                    columns
                },
            })
            .collect::<Vec<_>>();
        sheet
    });

    let sheet_rows = create_memo( move |_| sheet.get().rows);

    let export = move |_| {
        spawn_local(async move {
            match invoke::<_, ()>(
                "export_sheet",
                &ExportSheetArg {
                    sheet: sheet.get(),
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
            set_deleted_rows.update(|xs| xs.retain(|x| *x != id));
        } else {
            set_deleted_rows.update(|xs| xs.push(id));
        }
    };
    let delete_new_row = move |id| set_added_rows.update(|xs| xs.retain(|x| x.id != id));
    let toggle_edit_mode = move |_| {
        if edit_mode.get() {
            spawn_local(async move {
                let reset = if
		    !deleted_rows.get().is_empty()
		    || !added_rows.get().is_empty()
		    || !modified_columns.get().is_empty()
		{
                    confirm("سيتم تجاهل كل التعديلات").await
                } else {
                    true
                };
                if reset {
                    set_edit_mode.set(false);
                    set_deleted_rows.set(Vec::new());
                    set_added_rows.set(Vec::new());
		    set_modified_columns.set(Vec::new());
                }
            })
        } else {
            set_edit_mode.set(true);
        }
    };
    let append = move |row| {
        set_added_rows.update(|xs| {
            xs.push(row);
            xs.sort_rows(sheet_priorities_resource.read().unwrap_or_default());
        })
    };
    let save_edits = move |_| {
        let Some(sheet) = sheet_resource.read() else {
	    return;
	};
        let sheet_name = sheet_name.get();
        let mut deleted_rows = deleted_rows.get();
        let mut added_rows = added_rows.get();
	let current_rows = sheet.rows;
	for c in modified_columns.get() {
	    let ColumnIdentity { row_id, header, value } = c;
	    if let [current_row] = current_rows
		.iter()
		.filter(|x| x.id == row_id)
		.collect::<Vec<_>>()[..] {
		if !deleted_rows.contains(&row_id) {
		    deleted_rows.push(row_id);
		}
			
		let mut columns = current_row.columns.clone();
		if let Some(column) = columns.get(&header)
		    .and_then(|c| Some(Column{value,..c.clone()})) {
			match added_rows
			.iter()
			.filter(|x| x.id == row_id)
			.collect::<Vec<_>>()
			.first() {
			    Some(row) => {
				let mut columns = row.columns.clone();
				columns.insert(header,column);
				let row = Row{id : row_id,columns};
				added_rows.retain(|x| x.id != row_id);
				added_rows.push(row);
			    },
			    None => {
				columns.insert(header, column);
				let row = Row{id : row_id,columns};
				added_rows.push(row);
			    },
			}
		    };
	    }
	}
        spawn_local(async move {
            if !sheet_name.is_empty() && sheet_name != sheet.sheet_name {
                match invoke::<_, ()>(
                    "update_sheet_name",
                    &SheetNameArg {
                        name: Name {
                            id: sheet.id,
                            the_name: sheet_name,
                        },
                    },
                )
                .await
                {
                    Ok(_) => {
                        set_sheet_name.set(String::from(""));
                        message("نجح تغيير الاسم").await
                    }
                    Err(err) => alert(err.to_string().as_str()).await,
                }
            }
            if !deleted_rows.is_empty() {
                match invoke::<_, ()>(
                    "delete_rows_from_sheet",
                    &RowsDeleteArg {
                        sheetid: sheet.id,
                        rowsids: deleted_rows,
                    },
                )
                .await
                {
                    Ok(_) => {
                        set_deleted_rows.set(Vec::new());
                        message("نجح الحذف").await
                    }
                    Err(err) => alert(err.to_string().as_str()).await,
                }
            }
            if !added_rows.is_empty() {
                match invoke::<_, ()>(
                    "add_rows_to_sheet",
                    &RowsAddArg {
                        sheetid: sheet.id,
                        rows: added_rows,
                    },
                )
                .await
                {
                    Ok(_) => {
                        set_added_rows.set(Vec::new());
                        message("نجحت الاضافة").await
                    }
                    Err(err) => alert(err.to_string().as_str()).await,
                }
            }
        });
        set_edit_mode.set(false);
	set_modified_columns.set(Vec::new());
        sheet_resource.refetch();
    };

    let load_file = move |_| {
        let sheettype = sheet_type_name_resource.read().unwrap_or_default();
        spawn_local(async move {
            let Some(filepath) = open_file().await else {
		return;
	    };
            let rows = import_sheet_rows(sheettype, filepath).await;
            set_added_rows.update(|xs| {
                xs.extend(rows);
                xs.sort_rows(sheet_priorities_resource.read().unwrap_or_default());
            });
        });
    };

    view! { 
        <section>
            <A class="left-corner" href=format!("/sheet/{}", sheet_type_id().unwrap_or_default())>
                "->"
            </A>
            <button class="right-corner" on:click=export>
                "🏹"
            </button>
            <br/>
            <Show
                when=move || edit_mode.get()
                fallback=move || {
                    view! {  <h1>{move || sheet_resource.read().unwrap_or_default().sheet_name}</h1> }
                }
            >
                <input
                    type="text"
                    class="centered-input"
                    placeholder=move || {
                        format!(
                            "{} ({})", "اسم الشيت", sheet_resource.read().unwrap_or_default()
                            .sheet_name
                        )
                    }
                    value=move || sheet_name.get()
                    on:input=move |ev| set_sheet_name.set(event_target_value(&ev))
                />
            </Show>
            <table>
                <SheetHead basic_headers=basic_headers calc_headers=calc_headers/>
                <tbody>
                    <ShowRows
                        delete_row=delete_row
                        basic_headers=basic_headers
                        calc_headers=calc_headers
                        rows=sheet_rows
                        edit_mode=edit_mode
                        is_deleted=is_deleted
	                modified_columns=modified_columns
	                set_modified_columns=set_modified_columns
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
                        set_rows=set_added_rows
	                priorities=move || sheet_priorities_resource.read().unwrap_or_default()
                    />
                    <Show
                        when=move || edit_mode.get()
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
            <button on:click=toggle_edit_mode class="centered-button">
                {move || if edit_mode.get() { "الغاء" } else { "تعديل" }}
            </button>
            <Show
                when=move || edit_mode.get()
                fallback=|| {
                    view! {  <></> }
                }
            >
                <button on:click=save_edits class="centered-button">
                    "تاكيد"
                </button>
                <button on:click=load_file class="centered-button">
                    "تحميل ملف"
                </button>
            </Show>
            <Outlet/>
        </section>
    }
}

#[component]
fn ShowRows<BH, CH, FD, ID>(
    basic_headers: BH,
    calc_headers: CH,
    delete_row: FD,
    is_deleted: ID,
    rows: Memo<Vec<Row>>,
    edit_mode: ReadSignal<bool>,
    modified_columns: ReadSignal<Vec<ColumnIdentity>>,
    set_modified_columns: WriteSignal<Vec<ColumnIdentity>>,
) -> impl IntoView
where
    BH: Fn() -> Vec<String> + 'static + Clone + Copy,
    CH: Fn() -> Vec<String> + 'static + Clone + Copy,
    ID: Fn(Uuid) -> bool + 'static + Clone + Copy,
    FD: Fn(Uuid) + 'static + Clone + Copy,
{
    let (edit_column,set_edit_column) = create_signal( None::<ColumnIdentity>);
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
                    cancel=move || set_edit_column.set(None)
	            push_to_modified=move |col| set_modified_columns.update(|xs| xs.push(col))
                />
            </Show>
        <For
            each=move || rows.get()
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
                                                 on:dblclick=move |_| if edit_mode.get() {
						    set_edit_column.set(Some(ColumnIdentity{
							row_id:id,
							header:header1.clone(),
							value :columns1
							    .get(&header2).clone()
							    .unwrap().value.clone()
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
                            when=move || edit_mode.get()
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
					set_modified_columns.update(|xs| xs.retain(|x| x.row_id != id))
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
