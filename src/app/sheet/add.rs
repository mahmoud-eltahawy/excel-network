use chrono::NaiveDate;
use leptos::*;
use leptos_router::*;
use models::{ConfigValue, HeaderGetter, Column, ColumnConfig, ColumnProps, OperationConfig, Operation, ValueType};
use serde::{Serialize, Deserialize};
use std::{str::FromStr, collections::HashMap};

use super::shared::SheetHead;

use uuid::Uuid;
use tauri_sys::tauri::invoke;
use crate::Id;

#[derive(Debug,Serialize,Deserialize)]
struct NameArg{
    name : Option<String>
}

#[component]
pub fn AddSheet(cx: Scope) -> impl IntoView {
    let (sheet_name, set_sheet_name) = create_signal(cx, String::from(""));
    let params = use_params_map(cx);
    let sheet_id = move || params.
	with(|params| match params.get("sheet_type_id") {
	    Some(id) => Uuid::from_str(id).ok(),
	    None => None
	});
    let sheet_type_name_resource = create_resource(
	cx,
	|| () ,
	move |_| async move {
            invoke::<Id, String>("sheet_type_name", &Id{id : sheet_id()})
            .await
	    .unwrap_or_default()
    });

    let sheet_headers_resource = create_resource(
	cx,
	move || sheet_type_name_resource.read(cx),
	move |name| async move {
            invoke::<NameArg, Vec<ConfigValue>>("sheet_headers", &NameArg{name})
            .await
	    .unwrap_or_default()
    });
    let basic_columns = create_memo(cx, move |_| sheet_headers_resource
	  .read(cx)
	  .unwrap_or_default()
	  .into_iter().flat_map(|x| match x {
		  ConfigValue::Basic(conf) => Some(conf),
		  ConfigValue::Calculated(_) => None
	  }).collect::<Vec<_>>());

    let calc_columns = create_memo(cx,move |_| sheet_headers_resource
	  .read(cx)
	  .unwrap_or_default()
	  .into_iter().flat_map(|x| match x {
		  ConfigValue::Basic(_) => None,
		  ConfigValue::Calculated(conf) => Some(conf),
	      }).collect::<Vec<_>>());

    let basic_headers =move || basic_columns
	.get()
	.into_iter()
	.map(|x| x.get_header())
	.collect::<Vec<_>>();

    let calc_headers =move || calc_columns
	.get()
	.into_iter()
	.map(|x| x.header)
	.collect::<Vec<_>>();


    let append = |map| {};


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
                    <InputRow
                        basic_headers=basic_headers
                        calc_headers=calc_headers
                        append=append
                        basic_columns=basic_columns
                        calc_columns=calc_columns
                    />
                </tbody>
            </table>
            <Outlet/>
        </section>
    }
}


type GetterSetter<T> = (ReadSignal<T>,WriteSignal<T>);

#[derive(Debug, Clone,PartialEq)]
pub enum ColumnSignal {
    String(GetterSetter<String>),
    Float(GetterSetter<f64>),
    Date(GetterSetter<NaiveDate>),
}

use chrono::Local;

#[component]
fn InputRow<F,BH,CH>(
    cx: Scope,
    basic_headers : BH,
    calc_headers : CH,
    append : F,
    basic_columns : Memo<Vec<ColumnConfig>>,
    calc_columns : Memo<Vec<OperationConfig>>,
) -> impl IntoView
where
    F : Fn(HashMap<String,(Column,bool)>) + 'static,
    BH : Fn() -> Vec<String> + 'static + Clone,
    CH : Fn() -> Vec<String> + 'static,
{
    let basic_signals_map = create_memo(cx, move |_| {
	let mut map = HashMap::new();
	for x in basic_columns.get().into_iter() {
	    match x {
		ColumnConfig::String(ColumnProps{header,is_completable:_}) => {
		    map.insert(
			    header,
			    ColumnSignal::String(create_signal(cx, String::from(""))));
		},
		ColumnConfig::Date(ColumnProps{header,is_completable:_}) => {
		    map.insert(
			    header,
			    ColumnSignal::Date(create_signal(cx, Local::now().date_naive())));
		},
		ColumnConfig::Float(ColumnProps{header,is_completable:_}) => {
		    map.insert(
			    header,
			    ColumnSignal::Float(create_signal(cx, 0.0)));
		},
	    }
	};
	map
    });

    let calc_signals_map = create_memo(cx,move |_| {
	let mut map = HashMap::new();
	for OperationConfig { header, value } in calc_columns.get().into_iter() {
	    map.insert(header, match_operation(value,basic_signals_map));
	}
	map
    });

    let on_click = move |_| {
	let mut result = HashMap::new();
	for (key,value) in basic_signals_map.get(){
	    result.insert(key, match value {
		ColumnSignal::String((reader,_)) => (Column::String(Some(reader.get())),true),
		ColumnSignal::Float((reader,_)) => (Column::Float(reader.get()),true),
		ColumnSignal::Date((reader,_)) => (Column::Date(Some(reader.get())),true),
	    });
	}
	for (key ,value) in calc_signals_map.get() {
	    result.insert(key, (Column::Float(value),false));
	}
	append(result);
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
                                Some(x) => format!("{:.2}",*x),
                                None => format!("{:.2}",0.0),
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
    value : Operation,
    basic_signals_map : Memo<HashMap<String,ColumnSignal>>,
) -> f64{
    fn sum(v1 : f64,v2 : f64) -> f64 { v1 + v2}
    fn div(v1 : f64,v2 : f64) -> f64 { v1 / v2}
    fn mul(v1 : f64,v2 : f64) -> f64 { v1 * v2}
    fn sub(v1 : f64,v2 : f64) -> f64 { v1 - v2}
    fn basic_calc<F>(
	basic_signals_map : Memo<HashMap<String,ColumnSignal>>,
	vt1 : ValueType,
	vt2 : ValueType,
	calc : F,
    ) -> f64
	where F : Fn(f64,f64) -> f64 + 'static
    {
	match (vt1,vt2) {
	    (ValueType::Const(val1),ValueType::Const(val2)) => calc(val1, val2),
	    (ValueType::Variable(var),ValueType::Const(val2)) => {
		match basic_signals_map.get().get(&var) {
		    Some(val1) => match val1 {
			ColumnSignal::Float((val1,_)) => calc(val1.get(), val2),
			_ => 0.0,
		    }
		    None => 0.0,
		}
	    },
	    (ValueType::Const(val1),ValueType::Variable(var)) => {
		match basic_signals_map.get().get(&var) {
		    Some(val2) => match val2 {
			ColumnSignal::Float((val2,_)) => calc(val1,val2.get()),
			_ => 0.0,
		    }
		    None => 0.0,
		}
	    },
	    (ValueType::Variable(var1),ValueType::Variable(var2)) => {
		let bsm =basic_signals_map.get(); 
		match (bsm.get(&var1),bsm.get(&var2)) {
		    (Some(val1),Some(val2)) => match (val1,val2) {
			(ColumnSignal::Float((val1,_)),
			    ColumnSignal::Float((val2,_))) => calc(val1.get(), val2.get()),
			_ => 0.0,
		    }
		    _ => 0.0,
		}
	    },
	}
    }
    fn calc_o<F>(
	basic_signals_map : Memo<HashMap<String,ColumnSignal>>,
	v : ValueType,
	bop : Box<Operation>,
	calc : F,
    ) -> f64
	where F : Fn(f64,f64) -> f64 + 'static
    {
	match (v,bop) {
	    (ValueType::Const(val),bop) => calc(
		val,
		match_operation(*bop,basic_signals_map),
	    ),
	    (ValueType::Variable(var),bop) => {
		match basic_signals_map.get().get(&var) {
		    Some(val) => match val {
			ColumnSignal::Float((val,_)) => calc(
			    val.get(),
			    match_operation(*bop,basic_signals_map)),
			_ => 0.0,
		    }
		    None => 0.0,
		}
	    },
	}
    }
    match value {
	Operation::Multiply((v1,v2)) => basic_calc(basic_signals_map, v1, v2,mul),
	Operation::Add((v1,v2)) => basic_calc(basic_signals_map, v1, v2,sum),
	Operation::Divide((v1,v2)) => basic_calc(basic_signals_map, v1, v2,div),
	Operation::Minus((v1,v2)) => basic_calc(basic_signals_map, v1, v2,sub),
	Operation::MultiplyO((v,bop)) => calc_o(basic_signals_map, v, bop,mul),
	Operation::AddO((v,bop)) => calc_o(basic_signals_map, v, bop,sum),
	Operation::DivideO((v,bop)) => calc_o(basic_signals_map, v, bop,div),
	Operation::MinusO((v,bop)) => calc_o(basic_signals_map, v, bop,sub),
	Operation::OMultiply((bop,v)) => calc_o(basic_signals_map, v, bop,mul),
	Operation::OAdd((bop,v)) => calc_o(basic_signals_map, v, bop,sum),
	Operation::ODivide((bop,v)) => calc_o(basic_signals_map, v, bop,div),
	Operation::OMinus((bop,v)) => calc_o(basic_signals_map, v, bop,sub),
    }
}

fn make_input(
    cx: Scope,
    header : String,
    basic_signals_map : Memo<HashMap<String,ColumnSignal>>,
) -> impl IntoView {
    let cmp_arg = basic_signals_map.get();
    let (i_type,value) = match cmp_arg.get(&header) {
	Some(ColumnSignal::String((read,_))) => {
	    ("text",read.get().to_string())
	},
	Some(ColumnSignal::Float((read,_))) => {
	    ("number",read.get().to_string())
	},
	Some(ColumnSignal::Date((read,_))) => {
	    ("date",read.get().to_string())
	},
	None => {
	    ("","".to_string())
	}
    };
    view! { cx,
        <>
            <td>
            <input
	    type=i_type
	    value=move || value.clone()
	    // {
	    // 	let value = value.clone();
	    // 	if value.parse::<f64>().is_ok(){
	    // 	    value.parse::<f64>().unwrap()
	    // 	} else {
	    // 	    value
	    // 	}
	    // }
	    on:change=move |ev| match cmp_arg.get(&header) {
		Some(ColumnSignal::String((_,write))) => write.set(event_target_value(&ev)),
		Some(ColumnSignal::Float((_,write))) => write.set(event_target_value(&ev).parse().unwrap_or_default()),
		Some(ColumnSignal::Date((_,write))) => write.set(event_target_value(&ev).parse().unwrap_or_default()),
		None => ()
	    }/>
            </td>
        </>
    }
}
