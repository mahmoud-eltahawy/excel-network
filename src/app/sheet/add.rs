use chrono::NaiveDate;
use leptos::*;
use leptos_router::*;
use models::{
    Column, ColumnConfig, ColumnProps, ColumnValue, ConfigValue, HeaderGetter, Operation,
    OperationConfig, Row, ValueType,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

use super::shared::{alert,message, new_id, SheetHead};

use crate::Id;
use tauri_sys::tauri::invoke;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct NameArg {
    name: Option<String>,
}

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
    let sheet_id = move || {
        params.with(|params| match params.get("sheet_type_id") {
            Some(id) => Uuid::from_str(id).ok(),
            None => None,
        })
    };
    let sheet_type_name_resource = create_resource(
        cx,
        || (),
        move |_| async move {
            invoke::<Id, String>("sheet_type_name", &Id { id: sheet_id() })
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
		},
                Err(err) => alert(err.to_string().as_str()).await,
            }
        });
    };

    view! { cx,
        <section>
            <A class="left-corner" href=format!("/sheet/{}", sheet_id().unwrap_or_default())>
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
                    <ShowRows
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
            <button on:click=save_sheet class="centered-button">
                "حفظ الشيت"
            </button>
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

type GetterSetter<T> = (ReadSignal<T>, WriteSignal<T>);

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnSignal {
    String(GetterSetter<String>),
    Float(GetterSetter<f64>),
    Date(GetterSetter<NaiveDate>),
}

use chrono::Local;

#[component]
fn InputRow<F, BH, CH>(
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
                    is_completable: _,
                }) => {
                    map.insert(
                        header,
                        ColumnSignal::String(create_signal(cx, String::from(""))),
                    );
                }
                ColumnConfig::Date(ColumnProps {
                    header,
                    is_completable: _,
                }) => {
                    map.insert(
                        header,
                        ColumnSignal::Date(create_signal(cx, Local::now().date_naive())),
                    );
                }
                ColumnConfig::Float(ColumnProps {
                    header,
                    is_completable: _,
                }) => {
                    map.insert(header, ColumnSignal::Float(create_signal(cx, 0.0)));
                }
            }
        }
        map
    });

    let calc_signals_map = create_memo(cx, move |_| {
        let mut map = HashMap::new();
        for OperationConfig { header, value } in calc_columns.get().into_iter() {
            map.insert(header, match_operation(value, basic_signals_map));
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
                        value: ColumnValue::String(Some(reader.get())),
                    },
                    ColumnSignal::Float((reader, _)) => Column {
                        is_basic: true,
                        value: ColumnValue::Float(reader.get()),
                    },
                    ColumnSignal::Date((reader, _)) => Column {
                        is_basic: true,
                        value: ColumnValue::Date(Some(reader.get())),
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
                    view! { cx, <>{move || make_input(cx, header.clone(), basic_signals_map)}</> }
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

fn match_operation(
    value: Operation,
    basic_signals_map: Memo<HashMap<String, ColumnSignal>>,
) -> f64 {
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
        basic_signals_map: Memo<HashMap<String, ColumnSignal>>,
        vt1: ValueType,
        vt2: ValueType,
        calc: F,
    ) -> f64
    where
        F: Fn(f64, f64) -> f64 + 'static,
    {
        match (vt1, vt2) {
            (ValueType::Const(val1), ValueType::Const(val2)) => calc(val1, val2),
            (ValueType::Variable(var), ValueType::Const(val2)) => {
                match basic_signals_map.get().get(&var) {
                    Some(ColumnSignal::Float((val1, _))) => calc(val1.get(), val2),
                    _ => 0.0,
                }
            }
            (ValueType::Const(val1), ValueType::Variable(var)) => {
                match basic_signals_map.get().get(&var) {
                    Some(ColumnSignal::Float((val2, _))) => calc(val1, val2.get()),
                    _ => 0.0,
                }
            }
            (ValueType::Variable(var1), ValueType::Variable(var2)) => {
                let bsm = basic_signals_map.get();
                match (bsm.get(&var1), bsm.get(&var2)) {
                    (
                        Some(ColumnSignal::Float((val1, _))),
                        Some(ColumnSignal::Float((val2, _))),
                    ) => calc(val1.get(), val2.get()),
                    _ => 0.0,
                }
            }
        }
    }
    fn calc_o<F>(
        basic_signals_map: Memo<HashMap<String, ColumnSignal>>,
        v: ValueType,
        bop: Box<Operation>,
        calc: F,
    ) -> f64
    where
        F: Fn(f64, f64) -> f64 + 'static,
    {
        match (v, bop) {
            (ValueType::Const(val), bop) => calc(val, match_operation(*bop, basic_signals_map)),
            (ValueType::Variable(var), bop) => match basic_signals_map.get().get(&var) {
                Some(ColumnSignal::Float((val, _))) => {
                    calc(val.get(), match_operation(*bop, basic_signals_map))
                }
                _ => 0.0,
            },
        }
    }
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

fn make_input(
    cx: Scope,
    header: String,
    basic_signals_map: Memo<HashMap<String, ColumnSignal>>,
) -> impl IntoView {
    let cmp_arg = basic_signals_map.get();
    let (i_type, value) = match cmp_arg.get(&header) {
        Some(ColumnSignal::String((read, _))) => ("text", read.get().to_string()),
        Some(ColumnSignal::Float((read, _))) => ("number", read.get().to_string()),
        Some(ColumnSignal::Date((read, _))) => ("date", read.get().to_string()),
        None => ("", "".to_string()),
    };
    view! { cx,
        <>
            <td>
                <input
                    type=i_type
                    value=move || value.clone()
                    on:change=move |ev| match cmp_arg.get(&header) {
                        Some(ColumnSignal::String((_, write))) => write.set(event_target_value(&ev)),
                        Some(ColumnSignal::Float((_, write))) => {
                            write.set(event_target_value(&ev).parse().unwrap_or_default())
                        }
                        Some(ColumnSignal::Date((_, write))) => {
                            write.set(event_target_value(&ev).parse().unwrap_or_default())
                        }
                        None => {}
                    }
                />
            </td>
        </>
    }
}
