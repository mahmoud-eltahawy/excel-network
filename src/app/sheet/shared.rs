use leptos::*;
use models::RowsSort;

use crate::Non;
use chrono::Local;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri_sys::{
    dialog::{FileDialogBuilder, MessageDialogBuilder, MessageDialogKind},
    path::{download_dir, home_dir},
    tauri::invoke,
};
use uuid::Uuid;

use models::{
    Column, ColumnConfig, ColumnProps, ColumnValue, Operation, OperationConfig, Row, ValueType,
};

use std::rc::Rc;

pub async fn new_id() -> Uuid {
    invoke::<_, Uuid>("new_id", &Non {}).await.unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NameArg {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ImportSheetArgs {
    sheettype: String,
    filepath: String,
}

pub async fn import_sheet_rows(sheettype: String, filepath: String) -> Vec<Row> {
    invoke::<ImportSheetArgs, Vec<Row>>(
        "import_sheet",
        &ImportSheetArgs {
            sheettype,
            filepath,
        },
    )
    .await
    .unwrap_or_default()
}

pub async fn alert(message: &str) {
    let mut builder = MessageDialogBuilder::new();
    builder.set_title("تحذير");
    builder.set_kind(MessageDialogKind::Warning);

    builder.message(message).await.unwrap_or_default();
}

pub async fn message(message: &str) {
    let mut builder = MessageDialogBuilder::new();
    builder.set_title("رسالة");
    builder.set_kind(MessageDialogKind::Info);

    builder.message(message).await.unwrap_or_default();
}

pub async fn confirm(message: &str) -> bool {
    let mut builder = MessageDialogBuilder::new();
    builder.set_title("تاكيد");
    builder.confirm(message).await.unwrap_or_default()
}

pub async fn open_file() -> Option<String> {
    let mut builder = FileDialogBuilder::new();
    builder.add_filter("Serialized", &["json"]);
    builder.set_title("اختر ملف");
    let download_dir = match download_dir().await {
        Ok(v) => Some(v),
        Err(_) => {
            let Ok(home_dir) = home_dir().await else {
		return None;
	    };
            Some(home_dir.join("Downloads"))
        }
    };
    let Some(download_dir) = download_dir else {
	return None;
    };
    builder.set_default_path(download_dir.as_path());
    let Ok(Some(path)) = builder.pick_file().await else {
	return None;
    };
    Some(path.display().to_string())
}

#[component]
pub fn SheetHead<Fa, Fb>(basic_headers: Fa, calc_headers: Fb) -> impl IntoView
where
    Fa: Fn() -> Vec<String> + 'static,
    Fb: Fn() -> Vec<String> + 'static,
{
    view! { 
        <thead>
            <tr>
                <For
                    each=basic_headers
                    key=move |x| x.clone()
                    view=move | x| {
                        view! {  <th>{x}</th> }
                    }
                />
                <th class="shapeless">"  "</th>
                <For
                    each=calc_headers
                    key=move |x| x.clone()
                    view=move | x| {
                        view! {  <th>{x}</th> }
                    }
                />
            </tr>
        </thead>
    }
}


#[component]
fn ColumnEdit<F1,F2,F3>(
    mode : F1,
    cancel : F2,
    priorities: F3,
    set_rows: WriteSignal<Vec<Row>>,
) -> impl IntoView
    where
	F1 : Fn() -> (String,Uuid,Rc<HashMap<String,Column>>) + 'static,
        F2 : Fn() + 'static + Clone + Copy,
        F3 : Fn() -> Vec<String> + 'static + Clone + Copy
{
    let (header,id ,map) = mode();
    let (column_value,set_column_value) = create_signal( map.clone().get(&header).map(|x| x.value.clone()));
    let (top,set_top) = create_signal( None::<usize>);
    let (down,set_down) = create_signal( None::<usize>);
    let header = Rc::new(header);
    let on_input = move |ev| {
	let value = event_target_value(&ev);
	let value = match column_value.get() {
	    Some(ColumnValue::Float(_)) => ColumnValue::Float(value.parse().unwrap_or_default()),
	    Some(ColumnValue::Date(_)) => ColumnValue::Date(Some(value.parse().unwrap_or_default())),
	    _ => ColumnValue::String(Some(value)),
	};
	set_column_value.set(Some(value));
    };

    let save = move |_| {
	let Some(value) = column_value.get() else {
	    return;
	};
	let mut rows = Vec::new();
	set_rows.update(|xs| {
	    let Some(index) = xs.iter().position(|x| x.id == id) else {
		return;
	    };
	    rows = match (top.get(),down.get()) {
		(None,None) => vec![xs.remove(index)],
		(Some(up),None) => {
		    let begin = if up > index {
			0
		    } else {
			index - up
		    };
		    let mut rows = Vec::new();
		    for i in (begin..=index).rev() {
			rows.push(xs.remove(i));
		    }
		    rows
		},
		(None,Some(down)) => {
		    let len = xs.len();
		    let end = if down >= len {
			len -1
		    } else {
			index + down -1
		    };
		    let mut rows = Vec::new();
		    for i in (index..=end).rev() {
			rows.push(xs.remove(i));
		    }
		    rows
		},
		(Some(up),Some(down)) => {
		    let len = xs.len();
		    let begin = if up > index {
			0
		    } else {
			index - up
		    };
		    let end = if down >= len {
			len -1
		    } else {
			index + down
		    };
		    let mut rows = Vec::new();
		    for i in (begin..=end).rev() {
			rows.push(xs.remove(i));
		    }
		    rows
		},
	    };
	});
	set_rows.update(|xs| {
	    let rows = rows.into_iter().map(|row| {
		let mut columns = row.columns;
		columns.insert(header.to_string(), Column { is_basic: true, value : value.clone() });
		Row{columns,..row}
	    }).collect::<Vec<_>>();
	    let mut rows = xs.to_owned()
		.into_iter()
		.chain(rows)
		.collect::<Vec<_>>();
	    rows.sort_rows(priorities());
	    *xs = rows;
	});
	set_down.set(None);
	set_top.set(None);
	cancel()
    };
    view! { 
        <div class="popup">
            <input
                type=move || match column_value.get() {
                    Some(ColumnValue::Float(_)) => "number",
                    Some(ColumnValue::Date(_)) => "date",
                    _ => "text",
                }
                placeholder=move || {
                    format!(
                        "{} ({})", "القيمة الحالية", column_value.get().map(| x | x
                        .to_string()).unwrap_or_default()
                    )
                }
                on:input=on_input
            />
            <input
		type="number"
	        placeholder="لاعلي"
	        on:input=move |ev| set_top.set(Some(event_target_value(&ev).parse().unwrap_or_default()))
	    />
            <input
		type="number"
	        placeholder="لاسفل"
	        on:input=move |ev| set_down.set(Some(event_target_value(&ev).parse().unwrap_or_default()))
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
pub fn ShowNewRows<BH, CH, FD,FP>(
    basic_headers: BH,
    calc_headers: CH,
    delete_row: FD,
    priorities: FP,
    rows: ReadSignal<Vec<Row>>,
    set_rows: WriteSignal<Vec<Row>>,
) -> impl IntoView
where
    BH: Fn() -> Vec<String> + 'static + Clone + Copy,
    CH: Fn() -> Vec<String> + 'static + Clone + Copy,
    FP: Fn() -> Vec<String> + 'static + Clone + Copy,
    FD: Fn(Uuid) + 'static + Clone + Copy,
{
    let (edit_column,set_edit_column) = create_signal( None::<(String,Uuid,Rc<HashMap<String,Column>>)>);
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
                    set_rows=set_rows
		    priorities=priorities
                />
            </Show>
            <For
                each=move || rows.get()
                key=|row| row.id
                view=move | Row { columns, id }| {
                    let columns = Rc::new(columns);
                    view! { 
                        <tr>
                            {
                                let columns = columns.clone();
                                view! { 
                                    <For
                                        each=basic_headers
                                        key=|key| key.clone()
                                        view=move | column| {
                                            let columns1 = columns.clone();
                                            let columns2 = columns1.clone();
                                            let col_name1 = column;
                                            let col_name2 = col_name1.clone();
                                            view! { 
                                                <td
                                                    style="cursor: pointer"
                                                    on:dblclick=move |_| set_edit_column.set(Some((col_name1.clone(), id, columns1.clone())))
                                                >
                                                    {move || columns2.get(&col_name2).map(|x| x.value.to_string())}
                                                </td>
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
                            } <td>
                                <button on:click=move |_| delete_row(id)>"X"</button>
                            </td>
                        </tr>
                    }
                }
            />
        </>
    }
}

type GetterSetter<T> = (ReadSignal<(T, bool)>, WriteSignal<(T, bool)>);

#[derive(Debug, Clone, PartialEq)]
enum ColumnSignal {
    String(GetterSetter<String>),
    Float(GetterSetter<f64>),
    Date(GetterSetter<NaiveDate>),
}

#[component]
pub fn InputRow<F, BH, CH>(
    basic_headers: BH,
    calc_headers: CH,
    append: F,
    basic_columns: Memo<Vec<ColumnConfig>>,
    calc_columns: Memo<Vec<OperationConfig>>,
) -> impl IntoView
where
    F: Fn(Row) + 'static + Clone + Copy,
    BH: Fn() -> Vec<String> + 'static + Clone,
    CH: Fn() -> Vec<String> + 'static,
{
    let basic_signals_map = create_memo( move |_| {
        let mut map = HashMap::new();
        for x in basic_columns.get().into_iter() {
            match x {
                ColumnConfig::String(ColumnProps {
                    header,
                    is_completable,
                }) => {
                    map.insert(
                        header,
                        ColumnSignal::String(create_signal( (String::from(""), is_completable))),
                    );
                }
                ColumnConfig::Date(ColumnProps {
                    header,
                    is_completable,
                }) => {
                    map.insert(
                        header,
                        ColumnSignal::Date(create_signal(
                            
                            (Local::now().date_naive(), is_completable),
                        )),
                    );
                }
                ColumnConfig::Float(ColumnProps {
                    header,
                    is_completable,
                }) => {
                    map.insert(
                        header,
                        ColumnSignal::Float(create_signal( (0.0, is_completable))),
                    );
                }
            }
        }
        map
    });

    let calc_signals_map = create_memo( move |_| {
        let mut map = HashMap::new();
        for OperationConfig { header, value } in calc_columns.get().into_iter() {
            let mut basic_map = HashMap::new();
            for (header, column_signal) in basic_signals_map.get() {
                let column_value = match column_signal {
                    ColumnSignal::String((reader, _)) => ColumnValue::String(Some(reader.get().0)),
                    ColumnSignal::Float((reader, _)) => ColumnValue::Float(reader.get().0),
                    ColumnSignal::Date((reader, _)) => ColumnValue::Date(Some(reader.get().0)),
                };
                basic_map.insert(header, column_value);
            }
            map.insert(header, calculate_operation(&value, basic_map));
        }
        map
    });

    let on_click = move |_| {
        let mut result = HashMap::new();
        for (key, value) in basic_signals_map.get() {
            result.insert(
                key,
                match value {
                    ColumnSignal::String((reader, _)) => Column {
                        is_basic: true,
                        value: ColumnValue::String(Some(reader.get().0)),
                    },
                    ColumnSignal::Float((reader, _)) => Column {
                        is_basic: true,
                        value: ColumnValue::Float(reader.get().0),
                    },
                    ColumnSignal::Date((reader, _)) => Column {
                        is_basic: true,
                        value: ColumnValue::Date(Some(reader.get().0)),
                    },
                },
            );
        }
        for (key, value) in calc_signals_map.get() {
            result.insert(
                key,
                Column {
                    is_basic: false,
                    value: ColumnValue::Float(value),
                },
            );
        }
        spawn_local(async move {
            append(Row {
                id: new_id().await,
                columns: result,
            });
        })
    };

    view! { 
        <>
            <For
                each=move || basic_headers().clone()
                key=|x| x.clone()
                view=move | header| {
                    view! {  <MyInput header=header basic_signals_map=basic_signals_map/> }
                }
            />
            <td class="shapeless">"  "</td>
            <For
                each=move || calc_headers().clone()
                key=|x| x.clone()
                view=move | header| {
                    view! { 
                        <td>
                            {move || match calc_signals_map.get().get(&header) {
                                Some(x) => format!("{:.2}",* x),
                                None => format!("{:.2}", 0.0),
                            }}
                        </td>
                    }
                }
            />
            <tr class="spanA">
                <td>
                    <button on:click=on_click class="centered-button">
                        "اضافة"
                    </button>
                </td>
            </tr>
        </>
    }
}

#[component]
fn MyInput(
    header: String,
    basic_signals_map: Memo<HashMap<String, ColumnSignal>>,
) -> impl IntoView {
    let cmp_arg = basic_signals_map.get();
    let (i_type, value) = match cmp_arg.get(&header) {
        Some(ColumnSignal::String((read, _))) => ("text", read.get().0.to_string()),
        Some(ColumnSignal::Float((read, _))) => ("number", read.get().0.to_string()),
        Some(ColumnSignal::Date((read, _))) => ("date", read.get().0.to_string()),
        None => ("", "".to_string()),
    };
    view! { 
        <td>
            <input
                type=i_type
                value=move || value.clone()
                on:change=move |ev| match cmp_arg.get(&header) {
                    Some(ColumnSignal::String((_, write))) => {
                        write.update(|x| x.0 = event_target_value(&ev))
                    }
                    Some(ColumnSignal::Float((_, write))) => {
                        write.update(|x| x.0 = event_target_value(&ev).parse().unwrap_or_default())
                    }
                    Some(ColumnSignal::Date((_, write))) => {
                        write.update(|x| x.0 = event_target_value(&ev).parse().unwrap_or_default())
                    }
                    None => {}
                }
            />
        </td>
    }
}

fn sum(v1: f64, v2: f64) -> f64 {
    v1 + v2
}
fn div(v1: f64, v2: f64) -> f64 {
    v1 / v2
}
fn mul(v1: f64, v2: f64) -> f64 {
    v1 * v2
}
fn sub(v1: f64, v2: f64) -> f64 {
    v1 - v2
}
fn basic_calc<F>(
    basic_signals_map: HashMap<String, ColumnValue>,
    vt1: &ValueType,
    vt2: &ValueType,
    calc: F,
) -> f64
where
    F: Fn(f64, f64) -> f64 + 'static,
{
    match (vt1, vt2) {
        (ValueType::Const(val1), ValueType::Const(val2)) => calc(*val1, *val2),
        (ValueType::Variable(var), ValueType::Const(val2)) => match basic_signals_map.get(var) {
            Some(ColumnValue::Float(val1)) => calc(*val1, *val2),
            _ => 0.0,
        },
        (ValueType::Const(val1), ValueType::Variable(var)) => match basic_signals_map.get(var) {
            Some(ColumnValue::Float(val2)) => calc(*val1, *val2),
            _ => 0.0,
        },
        (ValueType::Variable(var1), ValueType::Variable(var2)) => {
            match (basic_signals_map.get(var1), basic_signals_map.get(var2)) {
                (Some(ColumnValue::Float(val1)), Some(ColumnValue::Float(val2))) => {
                    calc(*val1, *val2)
                }
                _ => 0.0,
            }
        }
    }
}
fn calc_o<F>(
    basic_signals_map: HashMap<String, ColumnValue>,
    v: &ValueType,
    bop: &Operation,
    calc: F,
) -> f64
where
    F: Fn(f64, f64) -> f64 + 'static,
{
    match (v, bop) {
        (ValueType::Const(val), bop) => calc(*val, calculate_operation(bop, basic_signals_map)),
        (ValueType::Variable(var), bop) => match basic_signals_map.get(var) {
            Some(ColumnValue::Float(val)) => {
                calc(*val, calculate_operation(bop, basic_signals_map))
            }
            _ => 0.0,
        },
    }
}

pub fn calculate_operation(
    value: &Operation,
    basic_signals_map: HashMap<String, ColumnValue>,
) -> f64 {
    match value {
        Operation::Multiply((v1, v2)) => basic_calc(basic_signals_map, v1, v2, mul),
        Operation::Add((v1, v2)) => basic_calc(basic_signals_map, v1, v2, sum),
        Operation::Divide((v1, v2)) => basic_calc(basic_signals_map, v1, v2, div),
        Operation::Minus((v1, v2)) => basic_calc(basic_signals_map, v1, v2, sub),
        Operation::MultiplyO((v, bop)) => calc_o(basic_signals_map, v, bop, mul),
        Operation::AddO((v, bop)) => calc_o(basic_signals_map, v, bop, sum),
        Operation::DivideO((v, bop)) => calc_o(basic_signals_map, v, bop, div),
        Operation::MinusO((v, bop)) => calc_o(basic_signals_map, v, bop, sub),
        Operation::OMultiply((bop, v)) => calc_o(basic_signals_map, v, bop, mul),
        Operation::OAdd((bop, v)) => calc_o(basic_signals_map, v, bop, sum),
        Operation::ODivide((bop, v)) => calc_o(basic_signals_map, v, bop, div),
        Operation::OMinus((bop, v)) => calc_o(basic_signals_map, v, bop, sub),
    }
}
