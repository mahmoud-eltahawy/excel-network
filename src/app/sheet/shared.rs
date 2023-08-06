use leptos::*;

use crate::Non;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri_sys::{
    dialog::{MessageDialogBuilder, MessageDialogKind},
    tauri::invoke,
};
use uuid::Uuid;
use chrono::Local;

use models::{ColumnValue, Operation, ValueType,Row,ColumnConfig,OperationConfig,ColumnProps,Column};

pub async fn new_id() -> Uuid {
    invoke::<_, Uuid>("new_id", &Non {}).await.unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NameArg {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ImportSheetArgs{
    sheettype: String,
    name: String,
}

pub async fn import_sheet_rows(
    sheettype :String,
    name: String,
) -> Vec<Row>{
    invoke::<ImportSheetArgs, Vec<Row>>(
	"import_sheet",
	&ImportSheetArgs {
	    sheettype,
	    name,
	})
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

#[component]
pub fn SheetHead<Fa, Fb>(cx: Scope, basic_headers: Fa, calc_headers: Fb) -> impl IntoView
where
    Fa: Fn() -> Vec<String> + 'static,
    Fb: Fn() -> Vec<String> + 'static,
{
    view! { cx,
        <thead>
            <tr>
                <For
                    each=basic_headers
                    key=move |x| x.clone()
                    view=move |cx, x| {
                        view! { cx, <th>{x}</th> }
                    }
                />
                <For
                    each=calc_headers
                    key=move |x| x.clone()
                    view=move |cx, x| {
                        view! { cx, <th>{x}</th> }
                    }
                />
            </tr>
        </thead>
    }
}

#[component]
pub fn ShowNewRows<BH, CH, FD>(
    cx: Scope,
    basic_headers: BH,
    calc_headers: CH,
    delete_row: FD,
    rows: ReadSignal<Vec<Row>>,
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

type GetterSetter<T> = (ReadSignal<(T,bool)>, WriteSignal<(T,bool)>);

#[derive(Debug, Clone, PartialEq)]
enum ColumnSignal {
    String(GetterSetter<String>),
    Float(GetterSetter<f64>),
    Date(GetterSetter<NaiveDate>),
}

#[component]
pub fn InputRow<F, BH, CH>(
    cx: Scope,
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
    let basic_signals_map = create_memo(cx, move |_| {
        let mut map = HashMap::new();
        for x in basic_columns.get().into_iter() {
            match x {
                ColumnConfig::String(ColumnProps {
                    header,
                    is_completable,
                }) => {
                    map.insert(
                        header,
                        ColumnSignal::String(create_signal(cx, (String::from(""),is_completable))),
                    );
                }
                ColumnConfig::Date(ColumnProps {
                    header,
                    is_completable,
                }) => {
                    map.insert(
                        header,
                        ColumnSignal::Date(create_signal(cx, (Local::now().date_naive(),is_completable))),
                    );
                }
                ColumnConfig::Float(ColumnProps {
                    header,
                    is_completable,
                }) => {
                    map.insert(header, ColumnSignal::Float(create_signal(cx,(0.0,is_completable))));
                }
            }
        }
        map
    });

    let calc_signals_map = create_memo(cx, move |_| {
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

    view! { cx,
        <>
            <For
                each=move || basic_headers().clone()
                key=|x| x.clone()
                view=move |cx, header| {
                    view! { cx,
			    <MyInput
				header=header
				basic_signals_map=basic_signals_map
			    />
		    }
                }
            />
            <For
                each=move || calc_headers().clone()
                key=|x| x.clone()
                view=move |cx, header| {
                    view! { cx,
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
                    <button on:click=on_click>"اضافة"</button>
                </td>
            </tr>
        </>
    }
}

#[component]
fn MyInput(
    cx: Scope,
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
    view! { cx,
	<td>
	    <input
		type=i_type
		value=move || value.clone()
		on:change=move |ev| match cmp_arg.get(&header) {
		    Some(ColumnSignal::String((_, write))) =>
			write.update(|x| x.0 = event_target_value(&ev)),
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
    bop: &Box<Operation>,
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
