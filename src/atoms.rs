use std::rc::Rc;

use leptos::*;
use leptos_router::A;
use models::{Row, Sheet};
use serde::Serialize;
use tauri_sys::tauri::invoke;
use thaw::{Button, Icon};
use uuid::Uuid;
use web_sys::MouseEvent;

const ICON_STYLE: &str = "font-size: 2.5rem;";

#[component]
pub fn BackArrow(n: usize) -> impl IntoView {
    let href = "../".repeat(n);
    view! {
        <A href=href>
            <Icon style=ICON_STYLE icon=icondata::AiArrowLeftOutlined/>
        </A>
    }
}

#[derive(Clone)]
pub enum RenderMode {
    Accumalate,
    Collapse,
    None,
}

#[component]
pub fn CollapseIcon(
    render_mode: RwSignal<RenderMode>,
    is_collapsble: impl Fn() -> bool + 'static,
) -> impl IntoView {
    let toggle = move |_| match render_mode.get() {
        RenderMode::None | RenderMode::Accumalate => render_mode.set(RenderMode::Collapse),
        RenderMode::Collapse => render_mode.set(RenderMode::Accumalate),
    };
    let content = move || match render_mode.get() {
        RenderMode::None => icondata::AiStarTwotone,
        RenderMode::Accumalate => icondata::AiMoreOutlined,
        RenderMode::Collapse => icondata::AiNodeCollapseOutlined,
    };
    view! {
        <Show when=is_collapsble>
            <button on:click=toggle><Icon style=ICON_STYLE  icon=content()/></button>
        </Show>
    }
}

#[component]
pub fn AddIcon() -> impl IntoView {
    view! {
        <A href="add">
            <Icon style=ICON_STYLE icon=icondata::TiDocumentAdd/>
        </A>
    }
}

#[component]
pub fn SaveIcon(
    save_edits: impl Fn(MouseEvent) + Copy + 'static,
    has_anything_changed: impl Fn() -> bool + Copy + 'static,
) -> impl IntoView {
    view! {
        <Show
            when=has_anything_changed
        >
            <button on:click=save_edits>
                <Icon style=ICON_STYLE icon=icondata::AiSaveFilled/>
            </button>
        </Show>
    }
}
#[component]
pub fn DownIcon(scroll: impl Fn(MouseEvent) + Copy + 'static) -> impl IntoView {
    view! {
        <Button on_click=scroll>
            <Icon style=ICON_STYLE icon=icondata::AiArrowDownOutlined/>
        </Button>
    }
}
#[component]
pub fn UpIcon(scroll: impl Fn(MouseEvent) + Copy + 'static) -> impl IntoView {
    view! {
        <Button on_click=scroll>
            <Icon style=ICON_STYLE icon=icondata::AiArrowUpOutlined/>
        </Button>
    }
}

#[component]
pub fn EditIcon(
    on_edit: RwSignal<bool>,
    has_anything_changed: impl Fn() -> bool + Copy + 'static,
    revert_all_edits: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let cancel_edit = move || {
        spawn_local(async move {
            let reset = if has_anything_changed() {
                confirm("سيتم تجاهل كل التعديلات").await
            } else {
                true
            };
            if reset {
                revert_all_edits();
            }
        });
    };

    let toggle_edit_mode = move |_| {
        if on_edit.get() {
            on_edit.set(false);
        } else {
            on_edit.set(true);
        }
        cancel_edit()
    };

    view! {
        <button on:click=toggle_edit_mode>
            {
                move || if on_edit.get() {
                    view! {<Icon style=ICON_STYLE icon=icondata::AiCloseCircleFilled/>}
                } else {
                    view!{<Icon style=ICON_STYLE icon=icondata::AiEditFilled/>}
                }
            }
        </button>
    }
}

use crate::{
    app::sheet::shared::message,
    sheet::shared::{alert, confirm},
};

#[component]
pub fn ExcelExport(
    sheet: impl Fn() -> Sheet<Uuid, Rc<str>> + Copy + 'static,
    all_rows: impl Fn() -> Vec<Row<Uuid, Rc<str>>> + Copy + 'static,
    headers: impl Fn() -> Vec<Rc<str>> + Copy + 'static,
) -> impl IntoView {
    fn export(
        mut sheet: Sheet<Uuid, Rc<str>>,
        headers: Vec<Rc<str>>,
        all_rows: Vec<Row<Uuid, Rc<str>>>,
    ) {
        #[derive(Serialize)]
        struct Args {
            headers: Vec<Rc<str>>,
            sheet: Sheet<Uuid, Rc<str>>,
        }
        sheet.rows = all_rows;
        spawn_local(async move {
            match invoke::<_, ()>("export_sheet", &Args { sheet, headers }).await {
                Ok(_) => message("👍").await,
                Err(err) => alert(err.to_string().as_str()).await,
            }
        })
    }

    view! {
        <button on:click=move|_|export(sheet(),headers(),all_rows())>
            <Icon style=ICON_STYLE icon=icondata::AiFileExcelFilled/>
        </button>
    }
}
