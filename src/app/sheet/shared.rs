use leptos::*;
use models::Column;
use models::ColumnValue;
use models::Row;

use crate::Non;
use chrono::Local;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use tauri_sys::{
    dialog::{FileDialogBuilder, MessageDialogBuilder, MessageDialogKind},
    path::{download_dir, home_dir},
    tauri::invoke,
};
use uuid::Uuid;

use models::RowsSort;

use client_models::{
    ColumnConfig, ColumnProps, Operation, OperationConfig, OperationKind, ValueType,
};

use std::rc::Rc;

#[inline(always)]
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

#[inline(always)]
pub async fn import_sheet_rows(
    sheetid: Uuid,
    sheettype: Rc<str>,
    filepath: String,
) -> Vec<Row<Uuid, Rc<str>>> {
    invoke::<ImportSheetArgs, Vec<Row<Uuid, Rc<str>>>>(
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

#[inline(always)]
pub async fn alert(message: &str) {
    let mut builder = MessageDialogBuilder::new();
    builder.set_title("تحذير");
    builder.set_kind(MessageDialogKind::Warning);

    builder.message(message).await.unwrap_or_default();
}

#[inline(always)]
pub async fn message(message: &str) {
    let mut builder = MessageDialogBuilder::new();
    builder.set_title("رسالة");
    builder.set_kind(MessageDialogKind::Info);

    builder.message(message).await.unwrap_or_default();
}

#[inline(always)]
pub async fn confirm(message: &str) -> bool {
    let mut builder = MessageDialogBuilder::new();
    builder.set_title("تاكيد");
    builder.confirm(message).await.unwrap_or_default()
}

#[inline(always)]
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

#[inline(always)]
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
                    let:x
                >
                    <th>{x.to_string()}</th>
                </For>
                <th class="shapeless">"  "</th>
                <For
                    each=calc_headers
                    key=move |x| x.clone()
                    let:x
                >
                    <th>{x.to_string()}</th>
                </For>
            </tr>
        </thead>
    }
}

#[inline(always)]
#[component]
fn ColumnEdit<F1, F2, F3, F4>(
    mode: F1,
    cancel: F2,
    priorities: F3,
    get_column_type: F4,
    rows: RwSignal<Vec<Row<Uuid, Rc<str>>>>,
) -> impl IntoView
where
    F1: Fn() -> (Rc<str>, Uuid, Rc<HashMap<Rc<str>, Column<Rc<str>>>>) + 'static,
    F2: Fn() + 'static + Clone + Copy,
    F3: Fn() -> Rc<[Rc<str>]> + 'static + Clone + Copy,
    F4: Fn(String) -> Option<ColumnConfig> + 'static + Clone + Copy,
{
    let (header, id, map) = mode();

    let column_value = RwSignal::from(map.clone().get(&header).map(|x| x.value.clone()));
    let top = RwSignal::from(None::<usize>);
    let down = RwSignal::from(None::<usize>);

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
                        Column {
                            is_basic: true,
                            value: value.clone(),
                        },
                    );
                    Row { columns, ..row }
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
    #[component]
    fn MainInput<F4>(
        column_value: RwSignal<Option<ColumnValue<Rc<str>>>>,
        get_column_type: F4,
        header: Rc<str>,
    ) -> impl IntoView
    where
        F4: Fn(String) -> Option<ColumnConfig> + 'static + Clone + Copy,
    {
        let on_input = move |ev| {
            let value = event_target_value(&ev).trim().to_string();
            let value = match column_value.get() {
                Some(ColumnValue::Float(_)) => {
                    ColumnValue::Float(value.parse().unwrap_or_default())
                }
                Some(ColumnValue::Date(_)) => ColumnValue::Date(value.parse().unwrap_or_default()),
                _ => ColumnValue::String(Rc::from(value)),
            };
            column_value.set(Some(value));
        };

        let input_type = match get_column_type(header.to_string()) {
            Some(ColumnConfig::Float(_)) => "number",
            Some(ColumnConfig::Date(_)) => "date",
            _ => "text",
        };
        let placeholder = move || {
            format!(
                "{} ({})",
                "القيمة الحالية",
                column_value
                    .get()
                    .map(|x| x.to_string())
                    .unwrap_or_default()
            )
        };
        view! {
            <input
                type=input_type
                placeholder=placeholder
                on:input=on_input
            />
        }
    }

    view! {
        <div class="popup">
            <MainInput
                get_column_type=get_column_type
                header=header
                column_value=column_value
            />
            <input
            type="number"
            placeholder="لاعلي"
            on:input=move |ev| top.set(Some(event_target_value(&ev).trim().parse().unwrap_or_default()))
            />
            <input
            type="number"
            placeholder="لاسفل"
            on:input=move |ev| down.set(Some(event_target_value(&ev).trim().parse().unwrap_or_default()))
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

#[inline(always)]
#[component]
pub fn ShowNewRows<BH, CH, FD, FP, FI, BT>(
    basic_headers: BH,
    calc_headers: CH,
    delete_row: FD,
    priorities: FP,
    sheet_id: FI,
    get_column_type: BT,
    rows: RwSignal<Vec<Row<Uuid, Rc<str>>>>,
) -> impl IntoView
where
    BH: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    CH: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    FP: Fn() -> Rc<[Rc<str>]> + 'static + Clone + Copy,
    FD: Fn(Uuid) + 'static + Clone + Copy,
    FI: Fn() -> Uuid + 'static + Clone + Copy,
    BT: Fn(String) -> Option<ColumnConfig> + 'static + Clone + Copy,
{
    type EditColumn = (Rc<str>, Uuid, Rc<HashMap<Rc<str>, Column<Rc<str>>>>);
    let edit_column = RwSignal::from(None::<EditColumn>);
    let new_rows = Memo::new(move |_| {
        rows.get()
            .into_iter()
            .filter(|x| x.id != sheet_id())
            .collect::<Vec<_>>()
    });

    #[inline(always)]
    #[component]
    fn ShowColumnEdit<FP, BT>(
        edit_column: RwSignal<Option<EditColumn>>,
        rows: RwSignal<Vec<Row<Uuid, Rc<str>>>>,
        priorities: FP,
        get_column_type: BT,
    ) -> impl IntoView
    where
        FP: Fn() -> Rc<[Rc<str>]> + 'static + Clone + Copy,
        BT: Fn(String) -> Option<ColumnConfig> + 'static + Clone + Copy,
    {
        view! {
            <Show
                when=move || edit_column.get().is_some()
            >
                <ColumnEdit
                    mode=move || edit_column.get().unwrap()
                    cancel=move || edit_column.set(None)
                    rows=rows
                    priorities=priorities
                    get_column_type=get_column_type
                />
            </Show>
        }
    }

    #[inline(always)]
    #[component]
    fn BasicColumns<BH>(
        basic_headers: BH,
        columns: Rc<HashMap<Rc<str>, Column<Rc<str>>>>,
        edit_column: RwSignal<Option<EditColumn>>,
        id: Uuid,
    ) -> impl IntoView
    where
        BH: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    {
        let children = move |header: Rc<str>| {
            let on_dblclick = {
                let columns = columns.clone();
                let header = header.clone();
                move |_| edit_column.set(Some((header.clone(), id, columns.clone())))
            };
            let content = {
                let columns = columns.clone();
                let header = header.clone();
                move || columns.get(&header).map(|x| x.value.to_string())
            };
            view! {
                <td
                    style="cursor: pointer"
                    on:dblclick=on_dblclick
                >
                    {content}
                </td>
            }
        };

        view! {
            <For
                each=basic_headers
                key=|key| key.clone()
                children=children
            />
        }
    }

    #[inline(always)]
    #[component]
    fn CalcColumn<CH>(
        calc_headers: CH,
        columns: Rc<HashMap<Rc<str>, Column<Rc<str>>>>,
    ) -> impl IntoView
    where
        CH: Fn() -> Vec<Rc<str>> + 'static + Clone + Copy,
    {
        let children = move |column| {
            let columns = columns.clone();
            let content = move || columns.get(&column).map(|x| x.value.to_string());
            view! {  <td>{content}</td> }
        };
        view! {
            <For
                each=calc_headers
                key=|key| key.clone()
                children=children
            />
        }
    }
    let children = move |Row { columns, id }| {
        let columns = Rc::new(columns);
        view! {
            <tr>
                <BasicColumns
                    basic_headers=basic_headers
                    columns=columns.clone()
                    edit_column=edit_column
                    id=id
                />
                <td class="shapeless">"  "</td>
                <CalcColumn
                    calc_headers=calc_headers
                    columns=columns
                />
                <td>
                    <button on:click=move |_| delete_row(id)>"X"</button>
                </td>
            </tr>
        }
    };

    view! {
        <>
            <ShowColumnEdit
                edit_column=edit_column
                rows=rows
                priorities=priorities
                get_column_type=get_column_type
            />
            <For
                each=move || new_rows.get()
                key=|row| row.id
                children=children
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

#[inline(always)]
#[component]
pub fn InputRow<F, BH, CH>(
    basic_headers: BH,
    calc_headers: CH,
    append: F,
    basic_columns: Memo<Vec<ColumnConfig>>,
    calc_columns: Memo<Vec<OperationConfig>>,
) -> impl IntoView
where
    F: Fn(Row<Uuid, Rc<str>>) + 'static + Clone + Copy,
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
                    ColumnSignal::String(reader) => ColumnValue::String(Rc::from(reader.get().0)),
                    ColumnSignal::Float(reader) => ColumnValue::Float(reader.get().0),
                    ColumnSignal::Date(reader) => ColumnValue::Date(reader.get().0),
                };
                basic_map.insert(header, column_value);
            }
            map.insert(
                Rc::from(header),
                resolve_operation(&value, &basic_map).unwrap_or_default(),
            );
        }
        map
    });

    let on_click = move |_| {
        let mut result = HashMap::<Rc<str>, Column<Rc<str>>>::new();
        for (key, value) in basic_signals_map.get() {
            result.insert(
                key,
                match value {
                    ColumnSignal::String(reader) => Column {
                        is_basic: true,
                        value: ColumnValue::String(Rc::from(reader.get().0)),
                    },
                    ColumnSignal::Float(reader) => Column {
                        is_basic: true,
                        value: ColumnValue::Float(reader.get().0),
                    },
                    ColumnSignal::Date(reader) => Column {
                        is_basic: true,
                        value: ColumnValue::Date(reader.get().0),
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
        <tr>
            <For
                each=move || basic_headers().clone()
                key=|x| x.clone()
                let:header
            >
                <MyInput header=header basic_signals_map=basic_signals_map/>
            </For>
            <td class="shapeless">"  "</td>
            <For
                each=move || calc_headers().clone()
                key=|x| x.clone()
                let:header
            >
                <td>
                    {move || calc_signals_map
                        .get()
                        .get(&header)
                        .map(|x| format!("{:.2}",* x))
                    }
                </td>
            </For>
        </tr>
        <tr>
            <td class="shapeless"></td>
            <td class="shapeless"></td>
            <td class="shapeless"></td>
            <td class="shapeless">
                <button on:click=on_click class="centered-button">
                    "_+_"
                </button>
            </td>
        </tr>
        </>
    }
}

#[inline(always)]
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
                        write.update(|x| x.0 = event_target_value(&ev).trim().to_string())
                    }
                    Some(ColumnSignal::Float(write)) => {
                        write.update(|x| x.0 = event_target_value(&ev).trim().parse().unwrap_or_default())
                    }
                    Some(ColumnSignal::Date(write)) => {
                        write.update(|x| x.0 = event_target_value(&ev).trim().parse().unwrap_or_default())
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

#[inline(always)]
pub fn resolve_operation(
    operation: &Operation,
    columns_map: &HashMap<Rc<str>, ColumnValue<Rc<str>>>,
) -> Option<f64> {
    fn get_op(op: &OperationKind) -> impl Fn(f64, f64) -> f64 {
        match op {
            OperationKind::Multiply => |v1, v2| v1 * v2,
            OperationKind::Add => |v1, v2| v1 + v2,
            OperationKind::Divide => |v1, v2| v1 / v2,
            OperationKind::Minus => |v1, v2| v1 - v2,
        }
    }

    #[inline(always)]
    fn resolve_hs(
        hs: &ValueType,
        columns_map: &HashMap<Rc<str>, ColumnValue<Rc<str>>>,
    ) -> Option<f64> {
        match hs {
            ValueType::Const(hs) => Some(*hs),
            ValueType::Variable(hs) => match columns_map.get(&Rc::from(hs.as_str())) {
                Some(ColumnValue::Float(hs)) => Some(*hs),
                _ => None,
            },
            ValueType::Operation(lhs) => resolve_operation(lhs, columns_map),
        }
    }

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

#[inline(always)]
pub fn merge_primary_row_headers(
    primary_headers: Rc<[Rc<str>]>,
    non_primary_headers: Rc<[Rc<str>]>,
) -> Rc<[(Rc<str>, Rc<str>)]> {
    let space = non_primary_headers.len() as i32 - primary_headers.len() as i32;

    let mut primary_headers = primary_headers;
    let mut non_primary_headers = non_primary_headers;

    let vaccume = |space| (0..space).map(|_| Rc::from("")).collect::<Rc<[Rc<str>]>>();
    match space.cmp(&0_i32) {
        Ordering::Greater => {
            primary_headers = primary_headers
                .iter()
                .chain(vaccume(space).iter())
                .cloned()
                .collect::<Rc<[Rc<str>]>>();
        }
        Ordering::Less => {
            let space = -space;
            non_primary_headers = non_primary_headers
                .iter()
                .chain(vaccume(space).iter())
                .cloned()
                .collect::<Rc<[Rc<str>]>>();
        }
        Ordering::Equal => (),
    }

    primary_headers
        .iter()
        .cloned()
        .zip(non_primary_headers.iter().cloned())
        .collect::<Rc<[_]>>()
}

#[inline(always)]
#[component]
pub fn PrimaryRowContent<F1, F2, F3, F4>(
    headers: F1,
    is_in_edit_mode: F2,
    is_deleted: F3,
    delete_fun: F4,
    columns: Memo<HashMap<Rc<str>, Column<Rc<str>>>>,
    new_columns: RwSignal<HashMap<Rc<str>, Column<Rc<str>>>>,
) -> impl IntoView
where
    F1: Fn() -> Rc<[(Rc<str>, Rc<str>)]> + 'static + Clone + Copy,
    F2: Fn() -> bool + 'static + Clone + Copy,
    F3: Fn(Rc<str>) -> bool + 'static + Clone + Copy,
    F4: Fn(Rc<str>) + 'static + Clone + Copy,
{
    let all_columns = Memo::new(move |_| {
        columns
            .get()
            .into_iter()
            .chain(new_columns.get())
            .collect::<HashMap<_, _>>()
    });

    #[component]
    fn RightPrimaryColumns<F2, F3, F4>(
        primary: Rc<str>,
        columns: Memo<HashMap<Rc<str>, Column<Rc<str>>>>,
        new_columns: RwSignal<HashMap<Rc<str>, Column<Rc<str>>>>,
        is_in_edit_mode: F2,
        is_deleted: F3,
        delete_fun: F4,
    ) -> impl IntoView
    where
        F2: Fn() -> bool + 'static + Clone + Copy,
        F3: Fn(Rc<str>) -> bool + 'static + Clone + Copy,
        F4: Fn(Rc<str>) + 'static + Clone + Copy,
    {
        let get_old_value = move |primary: Rc<str>| {
            columns
                .get()
                .get(&primary)
                .map(|x| x.value.to_string())
                .unwrap_or_default()
        };

        let get_new_value = move |primary: Rc<str>| {
            new_columns
                .get()
                .get(&primary)
                .map(|x| " => ".to_string() + &x.value.to_string())
                .unwrap_or_default()
        };

        let primary_value_plus_transition =
            move |primary: Rc<str>| get_old_value(primary.clone()) + &get_new_value(primary);
        #[component]
        fn Editor<F2, F3, F4>(
            primary: Rc<str>,
            is_in_edit_mode: F2,
            is_deleted: F3,
            delete_fun: F4,
        ) -> impl IntoView
        where
            F2: Fn() -> bool + 'static + Clone + Copy,
            F3: Fn(Rc<str>) -> bool + 'static + Clone + Copy,
            F4: Fn(Rc<str>) + 'static + Clone + Copy,
        {
            view! {
                <Show
                    when=is_in_edit_mode
                    fallback=|| view!{<td class="shapeless"></td>}
                >
                    <td><button
                        on:click={let a = primary.clone();move |_| delete_fun(a.clone())}>
                            {
                            let p = primary.clone();
                            move || if is_deleted(p.clone()) {"P"} else {"X"}
                        }</button></td>
                </Show>
            }
        }
        view! {
            <Show
                when={let a = primary.clone();move || !get_old_value(a.clone()).is_empty()}
                fallback=|| view! {
                    <>
                    <td class="shapeless"></td>
                    <td class="shapeless"></td>
                    <td class="shapeless"></td>
                    <td class="shapeless"></td>
                    <td class="shapeless"></td>
                    <td class="shapeless"></td>
                    <td class="shapeless"></td>
                    </>
                }
            >
            <>
                <td>{let a = primary.clone();move || a.to_string()}</td>
                <td class="shapeless">" "</td>
                <td>{let p = primary.clone();move || primary_value_plus_transition(p.clone())}</td>
                <Editor
                    primary=primary.clone()
                    is_in_edit_mode=is_in_edit_mode
                    is_deleted=is_deleted
                    delete_fun=delete_fun
                />
                <td class="shapeless">" "</td>
                <td class="shapeless">" "</td>
                <td class="shapeless">" "</td>
            </>
            </Show>
        }
    }

    #[component]
    fn LeftNonPrimaryColumns<F2, F3, F4>(
        non_primary: Rc<str>,
        is_in_edit_mode: F2,
        is_deleted: F3,
        delete_fun: F4,
        all_columns: Memo<HashMap<Rc<str>, Column<Rc<str>>>>,
    ) -> impl IntoView
    where
        F2: Fn() -> bool + 'static + Clone + Copy,
        F3: Fn(Rc<str>) -> bool + 'static + Clone + Copy,
        F4: Fn(Rc<str>) + 'static + Clone + Copy,
    {
        #[component]
        fn TitleValue(
            non_primary: Rc<str>,
            all_columns: Memo<HashMap<Rc<str>, Column<Rc<str>>>>,
        ) -> impl IntoView {
            let title = {
                let a = non_primary.clone();
                move || a.to_string()
            };
            let value = {
                let np = non_primary.clone();
                move || {
                    all_columns
                        .get()
                        .get(&np)
                        .map(|x| x.value.to_string())
                        .unwrap_or_default()
                }
            };
            let con = {
                let value = value.clone();
                let title = title.clone();
                move || !title().is_empty() && !value().is_empty()
            };

            view! {
                <Show
                    when=con
                    fallback=|| view! {
                      <>
                        <td class="shapeless"></td>
                        <td class="shapeless"></td>
                        <td class="shapeless"></td>
                      </>
                    }
                >
                  <>
                    <td>{title.clone()}</td>
                    <td class="shapeless">" "</td>
                    <td>{value.clone()}</td>
                  </>
                </Show>
            }
        }
        let deleted_style = {
            let a = non_primary.clone();
            move || match is_deleted(a.clone()) {
                true => "fineb",
                false => "dangerb",
            }
        };
        view! {
            <>
            <TitleValue
                non_primary=non_primary.clone()
                all_columns=all_columns
            />
            <Show
                when=is_in_edit_mode
                fallback=|| view! {<td class="shapeless"></td>}
            >
                <td><button
                     class=deleted_style.clone()
                     on:click={let a =non_primary.clone(); move |_| delete_fun(a.clone())}>"X"</button></td>
            </Show>
            </>
        }
    }

    let each_head = move || headers().iter().cloned().collect::<Vec<_>>();

    let children = move |(primary, non_primary)| {
        view! {
            <tr>
            <RightPrimaryColumns
                primary=primary
                columns=columns
                new_columns=new_columns
                is_in_edit_mode=is_in_edit_mode
                is_deleted=is_deleted
                delete_fun=delete_fun
            />
            <LeftNonPrimaryColumns
                non_primary=non_primary
                is_in_edit_mode=is_in_edit_mode
                is_deleted=is_deleted
                delete_fun=delete_fun
                all_columns=all_columns

            />
            </tr>
        }
    };

    view! {
    <table>
        <For
        each=each_head
        key=|x| x.0.to_string() + x.1.as_ref()
        children=children
    />
    </table>
    }
}

#[inline(always)]
#[component]
pub fn PrimaryRowEditor(new_columns: RwSignal<HashMap<Rc<str>, Column<Rc<str>>>>) -> impl IntoView {
    let add_what = RwSignal::from(None::<&str>);
    let header = RwSignal::from(Rc::from(""));
    let column_value = RwSignal::from(ColumnValue::Float(0.0));

    let on_value_input = move |ev| {
        column_value.update(|x| match x {
            ColumnValue::String(_) => {
                *x = ColumnValue::String(Rc::from(event_target_value(&ev).trim()))
            }
            ColumnValue::Date(_) => {
                *x = ColumnValue::Date(event_target_value(&ev).trim().parse().unwrap_or_default())
            }
            ColumnValue::Float(_) => {
                *x = ColumnValue::Float(event_target_value(&ev).trim().parse().unwrap_or_default())
            }
        })
    };

    let append_column = move |_| {
        new_columns.update(|map| {
            map.insert(
                header.get(),
                Column {
                    is_basic: true,
                    value: column_value.get(),
                },
            );
        });
        add_what.set(None);
    };

    view! {
        <Show
        when=move || add_what.get().is_some()
        fallback=move|| view!{
        <>
            <button
            class="centered-button"
            on:click=move |_| {
            add_what.set(Some("date"));
            column_value.set(ColumnValue::Date(Local::now().date_naive()))
            }
            >"+ تاريخ"</button>
            <button
            class="centered-button"
            on:click=move |_| {
            add_what.set(Some("number"));
            column_value.set(ColumnValue::Float(0.0));
            }
            >"+ رقم"</button>
            <button
            class="centered-button"
            on:click=move |_| {
            add_what.set(Some("text"));
            column_value.set(ColumnValue::String(Rc::from("")));
            }
            >"+ نص"</button>
        </>
        }
        >
            <div>
            <input
            style="width:40%; height:30px;"
            type="text"
            placeholder="العنوان"
                on:input=move |ev| header.set(Rc::from(event_target_value(&ev).trim()))
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
            on:click=append_column
        class="centered-button"
        >"تاكيد"</button>
        <button
        class="centered-button"
           on:click=move |_| add_what.set(None)
        >"الغاء"</button>
        </Show>
    }
}
