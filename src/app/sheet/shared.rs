use leptos::*;
use models::FrontendColumn;
use models::FrontendColumnValue;
use models::FrontendRow;

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
    ColumnConfig, ColumnProps, Operation, OperationConfig, OperationKind, RowsSort, ValueType,
};

use std::rc::Rc;

pub async fn new_id() -> Uuid {
    invoke::<_, Uuid>("new_id", &Non {}).await.unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NameArg {
    pub name: Option<Rc<str>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ImportSheetArgs {
    sheettype: Rc<str>,
    sheetid: Uuid,
    filepath: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Name {
    pub id: Uuid,
    pub the_name: Rc<str>,
}

pub async fn import_sheet_rows(
    sheetid: Uuid,
    sheettype: Rc<str>,
    filepath: String,
) -> Vec<FrontendRow> {
    invoke::<ImportSheetArgs, Vec<FrontendRow>>(
        "import_sheet",
        &ImportSheetArgs {
            sheettype,
            filepath,
            sheetid,
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
    Fa: Fn() -> Vec<Rc<str>> + 'static,
    Fb: Fn() -> Vec<Rc<str>> + 'static,
{
    view! {
        <thead>
            <tr>
                <For
                    each=basic_headers
                    key=move |x| x.clone()
                    view=move | x| {
                        view! {  <th>{x.to_string()}</th> }
                    }
                />
                <th class="shapeless">"  "</th>
                <For
                    each=calc_headers
                    key=move |x| x.clone()
                    view=move | x| {
                        view! {  <th>{x.to_string()}</th> }
                    }
                />
            </tr>
        </thead>
    }
}

#[component]
fn ColumnEdit<F1, F2, F3>(
    mode: F1,
    cancel: F2,
    priorities: F3,
    rows: RwSignal<Vec<FrontendRow>>,
) -> impl IntoView
where
    F1: Fn() -> (Rc<str>, Uuid, Rc<HashMap<Rc<str>, FrontendColumn>>) + 'static,
    F2: Fn() + 'static + Clone + Copy,
    F3: Fn() -> Rc<[Rc<str>]> + 'static + Clone + Copy,
{
    let (header, id, map) = mode();
    let column_value = RwSignal::from(map.clone().get(&header).map(|x| x.value.clone()));
    let top = RwSignal::from(None::<usize>);
    let down = RwSignal::from(None::<usize>);
    let on_input = move |ev| {
        let value = event_target_value(&ev);
        let value = match column_value.get() {
            Some(FrontendColumnValue::Float(_)) => {
                FrontendColumnValue::Float(value.parse().unwrap_or_default())
            }
            Some(FrontendColumnValue::Date(_)) => {
                FrontendColumnValue::Date(Some(value.parse().unwrap_or_default()))
            }
            _ => FrontendColumnValue::String(Some(Rc::from(value))),
        };
        column_value.set(Some(value));
    };

    let save = move |_| {
        let Some(value) = column_value.get() else {
            return;
        };
        let mut the_rows = Vec::new();
        rows.update(|xs| {
            let Some(index) = xs.iter().position(|x| x.id == id) else {
                return;
            };
            the_rows = match (top.get(), down.get()) {
                (None, None) => vec![xs.remove(index)],
                (Some(up), None) => {
                    let begin = if up > index { 0 } else { index - up };
                    let mut rows = Vec::new();
                    for i in (begin..=index).rev() {
                        rows.push(xs.remove(i));
                    }
                    rows
                }
                (None, Some(down)) => {
                    let len = xs.len();
                    let end = if down >= len {
                        len - 1
                    } else {
                        index + down - 1
                    };
                    let mut rows = Vec::new();
                    for i in (index..=end).rev() {
                        rows.push(xs.remove(i));
                    }
                    rows
                }
                (Some(up), Some(down)) => {
                    let len = xs.len();
                    let begin = if up > index { 0 } else { index - up };
                    let end = if down >= len { len - 1 } else { index + down };
                    let mut rows = Vec::new();
                    for i in (begin..=end).rev() {
                        rows.push(xs.remove(i));
                    }
                    rows
                }
            };
        });
        rows.update(|xs| {
            let rows = the_rows
                .into_iter()
                .map(|row| {
                    let mut columns = row.columns;
                    columns.insert(
                        mode().0, //header
                        FrontendColumn {
                            is_basic: true,
                            value: value.clone(),
                        },
                    );
                    FrontendRow { columns, ..row }
                })
                .collect::<Vec<_>>();
            let mut rows = xs.iter().cloned().chain(rows).collect::<Vec<_>>();
            rows.sort_rows(priorities());
            *xs = rows;
        });
        down.set(None);
        top.set(None);
        cancel()
    };
    view! {
        <div class="popup">
            <input
                type=move || match column_value.get() {
                    Some(FrontendColumnValue::Float(_)) => "number",
                    Some(FrontendColumnValue::Date(_)) => "date",
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
            on:input=move |ev| top.set(Some(event_target_value(&ev).parse().unwrap_or_default()))
        />
            <input
        type="number"
            placeholder="لاسفل"
            on:input=move |ev| down.set(Some(event_target_value(&ev).parse().unwrap_or_default()))
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
pub fn ShowNewRows<BH, CH, FD, FP, FI>(
    basic_headers: BH,
    calc_headers: CH,
    delete_row: FD,
    priorities: FP,
    sheet_id: FI,
    rows: RwSignal<Vec<FrontendRow>>,
) -> impl IntoView
where
    BH: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    CH: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    FP: Fn() -> Rc<[Rc<str>]> + 'static + Clone + Copy,
    FD: Fn(Uuid) + 'static + Clone + Copy,
    FI: Fn() -> Uuid + 'static + Clone + Copy,
{
    let edit_column = RwSignal::from(None::<(Rc<str>, Uuid, Rc<HashMap<Rc<str>, FrontendColumn>>)>);
    let new_rows = Memo::new(move |_| {
        rows.get()
            .into_iter()
            .filter(|x| x.id != sheet_id())
            .collect::<Vec<_>>()
    });
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
                    cancel=move || edit_column.set(None)
                    rows=rows
                    priorities=priorities
                />
            </Show>
            <For
                each=move || new_rows.get()
                key=|row| row.id
                view=move | FrontendRow { columns, id }| {
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
                                                    on:dblclick=move |_| edit_column.set(Some((col_name1.clone(), id, columns1.clone())))
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

type GetterSetter<T> = RwSignal<(T, bool)>;

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
    F: Fn(FrontendRow) + 'static + Clone + Copy,
    BH: Fn() -> Vec<Rc<str>> + 'static + Clone,
    CH: Fn() -> Vec<Rc<str>> + 'static,
{
    let basic_signals_map = Memo::new(move |_| {
        let mut map = HashMap::<Rc<str>, _>::new();
        for x in basic_columns.get().into_iter() {
            match x {
                ColumnConfig::String(ColumnProps {
                    header,
                    is_completable,
                }) => {
                    map.insert(
                        Rc::from(header),
                        ColumnSignal::String(RwSignal::from((String::from(""), is_completable))),
                    );
                }
                ColumnConfig::Date(ColumnProps {
                    header,
                    is_completable,
                }) => {
                    map.insert(
                        Rc::from(header),
                        ColumnSignal::Date(RwSignal::from((
                            Local::now().date_naive(),
                            is_completable,
                        ))),
                    );
                }
                ColumnConfig::Float(ColumnProps {
                    header,
                    is_completable,
                }) => {
                    map.insert(
                        Rc::from(header),
                        ColumnSignal::Float(RwSignal::from((0.0, is_completable))),
                    );
                }
            }
        }
        map
    });

    let calc_signals_map = Memo::new(move |_| {
        let mut map = HashMap::<Rc<str>, _>::new();
        for OperationConfig { header, value } in calc_columns.get().into_iter() {
            let mut basic_map = HashMap::new();
            for (header, column_signal) in basic_signals_map.get() {
                let column_value = match column_signal {
                    ColumnSignal::String(reader) => {
                        FrontendColumnValue::String(Some(Rc::from(reader.get().0)))
                    }
                    ColumnSignal::Float(reader) => FrontendColumnValue::Float(reader.get().0),
                    ColumnSignal::Date(reader) => FrontendColumnValue::Date(Some(reader.get().0)),
                };
                basic_map.insert(Rc::from(header), column_value);
            }
            map.insert(
                Rc::from(header),
                resolve_operation(&value, &basic_map).unwrap_or_default(),
            );
        }
        map
    });

    let on_click = move |_| {
        let mut result = HashMap::<Rc<str>, FrontendColumn>::new();
        for (key, value) in basic_signals_map.get() {
            result.insert(
                key,
                match value {
                    ColumnSignal::String(reader) => FrontendColumn {
                        is_basic: true,
                        value: FrontendColumnValue::String(Some(Rc::from(reader.get().0))),
                    },
                    ColumnSignal::Float(reader) => FrontendColumn {
                        is_basic: true,
                        value: FrontendColumnValue::Float(reader.get().0),
                    },
                    ColumnSignal::Date(reader) => FrontendColumn {
                        is_basic: true,
                        value: FrontendColumnValue::Date(Some(reader.get().0)),
                    },
                },
            );
        }
        for (key, value) in calc_signals_map.get() {
            result.insert(
                key,
                FrontendColumn {
                    is_basic: false,
                    value: FrontendColumnValue::Float(value),
                },
            );
        }
        spawn_local(async move {
            append(FrontendRow {
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
    header: Rc<str>,
    basic_signals_map: Memo<HashMap<Rc<str>, ColumnSignal>>,
) -> impl IntoView {
    let cmp_arg = basic_signals_map.get();
    let (i_type, value) = match cmp_arg.get(&header) {
        Some(ColumnSignal::String(read)) => ("text", read.get().0.to_string()),
        Some(ColumnSignal::Float(read)) => ("number", read.get().0.to_string()),
        Some(ColumnSignal::Date(read)) => ("date", read.get().0.to_string()),
        None => ("", "".to_string()),
    };
    view! {
        <td>
            <input
                type=i_type
                value=move || value.clone()
                on:change=move |ev| match cmp_arg.get(&header) {
                    Some(ColumnSignal::String(write)) => {
                        write.update(|x| x.0 = event_target_value(&ev))
                    }
                    Some(ColumnSignal::Float(write)) => {
                        write.update(|x| x.0 = event_target_value(&ev).parse().unwrap_or_default())
                    }
                    Some(ColumnSignal::Date(write)) => {
                        write.update(|x| x.0 = event_target_value(&ev).parse().unwrap_or_default())
                    }
                    None => {}
                }
            />
        </td>
    }
}

#[derive(Clone)]
pub enum EditState {
    Primary,
    NonePrimary,
    LoadFile,
    None,
}

#[component]
pub fn PrimaryRow<FP, FN>(
    primary_headers: FP,
    non_primary_headers: FN,
    columns: Memo<HashMap<Rc<str>, FrontendColumn>>,
    new_columns: RwSignal<HashMap<Rc<str>, FrontendColumn>>,
    edit_mode: RwSignal<EditState>,
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

    let all_columns = Memo::new(move |_| {
        columns
            .get()
            .into_iter()
            .chain(new_columns.get())
            .collect::<HashMap<_, _>>()
    });

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
            <td>{let a = primary.clone();move || a.to_string()}</td>
            <td class="shapeless">" "</td>
            <td>{move || columns
                 .get()
                 .get(&primary)
                 .map(|x| x.value.to_string()
                  + &new_columns
                  .get()
                  .get(&primary)
                  .map(|x| " => ".to_string() + &x.value.to_string())
                  .unwrap_or_default()
                 )
            }
            </td>
            <td class="shapeless">" "</td>
            <td class="shapeless">" | "</td>
            <td class="shapeless">" "</td>
            <td>{let a = non_primary.clone();move || a.to_string()}</td>
            <td class="shapeless">" "</td>
            <td>{move ||all_columns.get().get(&non_primary).map(|x| x.value.to_string())}</td>
            </tr>
        }
        />
    </table>
    <Show
        when=move || matches!(edit_mode.get(),EditState::Primary)
        fallback=move|| view!{<></>}
    >
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
    </Show>
    </>
    }
}

fn get_op(op: &OperationKind) -> impl Fn(f64, f64) -> f64 {
    match op {
        OperationKind::Multiply => |v1, v2| v1 * v2,
        OperationKind::Add => |v1, v2| v1 + v2,
        OperationKind::Divide => |v1, v2| v1 / v2,
        OperationKind::Minus => |v1, v2| v1 - v2,
    }
}

fn resolve_hs(hs: &ValueType, columns_map: &HashMap<Rc<str>, FrontendColumnValue>) -> Option<f64> {
    match hs {
        ValueType::Const(hs) => Some(*hs),
        ValueType::Variable(hs) => match columns_map.get(&Rc::from(hs.as_str())) {
            Some(FrontendColumnValue::Float(hs)) => Some(*hs),
            _ => None,
        },
        ValueType::Operation(lhs) => {
            let lhs = lhs;
            resolve_operation(lhs, columns_map)
        }
    }
}

pub fn resolve_operation(
    operation: &Operation,
    columns_map: &HashMap<Rc<str>, FrontendColumnValue>,
) -> Option<f64> {
    let Operation { op, lhs, rhs } = operation;
    let op = get_op(op);
    let lhs = resolve_hs(lhs, columns_map);
    let rhs = resolve_hs(rhs, columns_map);
    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => Some(op(lhs, rhs)),
        (Some(lhs), None) => Some(lhs),
        (None, Some(rhs)) => Some(rhs),
        (None, None) => None,
    }
}
