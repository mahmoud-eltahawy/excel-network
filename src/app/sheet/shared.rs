use leptos::*;

use crate::Non;
use tauri_sys::{
    dialog::{MessageDialogBuilder, MessageDialogKind},
    tauri::invoke,
};
use uuid::Uuid;
use serde::{Serialize,Deserialize};
use std::collections::HashMap;
use chrono::NaiveDate;

use models::{
    Operation, ValueType,
};

pub async fn new_id() -> Uuid {
    invoke::<_, Uuid>("new_id", &Non {}).await.unwrap()
}


#[derive(Debug, Serialize, Deserialize)]
pub struct NameArg {
    pub name: Option<String>,
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

type GetterSetter<T> = (ReadSignal<T>, WriteSignal<T>);

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnSignal {
    String(GetterSetter<String>),
    Float(GetterSetter<f64>),
    Date(GetterSetter<NaiveDate>),
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
    basic_signals_map: HashMap<String, ColumnSignal>,
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
	    match basic_signals_map.get(&var) {
		Some(ColumnSignal::Float((val1, _))) => calc(val1.get(), val2),
		_ => 0.0,
	    }
	}
	(ValueType::Const(val1), ValueType::Variable(var)) => {
	    match basic_signals_map.get(&var) {
		Some(ColumnSignal::Float((val2, _))) => calc(val1, val2.get()),
		_ => 0.0,
	    }
	}
	(ValueType::Variable(var1), ValueType::Variable(var2)) => {
	    match (basic_signals_map.get(&var1), basic_signals_map.get(&var2)) {
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
    basic_signals_map: HashMap<String, ColumnSignal>,
    v: ValueType,
    bop: Box<Operation>,
    calc: F,
) -> f64
where
    F: Fn(f64, f64) -> f64 + 'static,
{
    match (v, bop) {
	(ValueType::Const(val), bop) => calc(val, match_operation(*bop, basic_signals_map)),
	(ValueType::Variable(var), bop) => match basic_signals_map.get(&var) {
	    Some(ColumnSignal::Float((val, _))) => {
		calc(val.get(), match_operation(*bop, basic_signals_map))
	    }
	    _ => 0.0,
	},
    }
}

pub fn match_operation(
    value: Operation,
    basic_signals_map: HashMap<String, ColumnSignal>,
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
