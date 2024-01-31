use chrono::NaiveDate;
use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thaw::{Button, ButtonSize, DatePicker, Input, Space};
use uuid::Uuid;

use crate::{
    atoms::{AddIcon, BackArrow, DownIcon, UpIcon},
    Id,
};
use tauri_sys::tauri::invoke;

use models::{Name, SearchSheetParams};

use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct SheetArgs {
    params: SearchSheetParams,
}

pub mod add;
pub mod shared;
pub mod show;

#[component]
pub fn SheetHome() -> impl IntoView {
    let params = use_params_map();
    let sheet_id = move || {
        params.with(|params| {
            params
                .get("sheet_type_id")
                .and_then(|id| Uuid::from_str(id).ok())
        })
    };
    let sheet_type_name_resource = Resource::once(move || async move {
        invoke::<Id, Rc<str>>("sheet_type_name", &Id { id: sheet_id() })
            .await
            .unwrap_or(Rc::from(""))
    });

    let sheet_type_name = move || match sheet_type_name_resource.get() {
        Some(name) => name,
        None => Rc::from("none"),
    };

    let offset = RwSignal::from(0_u64);
    let begin = RwSignal::from(None::<NaiveDate>);
    let end = RwSignal::from(None::<NaiveDate>);
    let sheet_name = RwSignal::from(String::from(""));

    let search_args = move || SheetArgs {
        params: SearchSheetParams {
            offset: offset.get() as i64,
            sheet_type_name: sheet_type_name().to_string(),
            sheet_name: {
                let name = sheet_name.get();
                if name.is_empty() {
                    None
                } else {
                    Some(name)
                }
            },
            begin: begin.get(),
            end: end.get(),
        },
    };

    let bills = Resource::new(search_args, |value| async move {
        invoke::<_, Rc<[Name<Uuid>]>>("top_5_sheets", &value)
            .await
            .unwrap_or(Rc::from(vec![]))
    });

    view! {
        <Space vertical=true>
            <Space>
                <BackArrow n=2/>
                <AddIcon/>
            </Space>
            <Input value=sheet_name placeholder=format!{"{}", "اسم الشيت"}/>
            <h3>"تاريخ البداية"</h3>
            <DatePicker value=begin/>
            <h3>"تاريخ النهاية"</h3>
            <DatePicker value=end/>
            <br/><br/><br/>
            <Show
                when=move || offset.get() != 0
            >
                <UpIcon scroll=move |_| offset.update(|x| *x -= 5)/>
            </Show>
            <br/>
            <For
                each=move || bills.get().unwrap_or(Rc::from(vec![])).to_vec()
                key=|s| s.id
                children=move |s| {
                    view! {
                        <Button on_click=move |_| {
                            let href = window().location().href().unwrap_or_default();
                            window()
                                .location()
                                .set_href(&format!("{}/show/{}", href,s.id))
                                .unwrap_or_default();
                            }
                            style="width: 70%; font-size : 1.2rem;"
                            size=ButtonSize::Large
                        >{s.the_name}</Button>
                        <br/><br/><br/><br/>
                    }
                }
            />
            <Show
                when=move || { bills.get().unwrap_or(Rc::from(vec![])).len() >= 5 }
            >
                <DownIcon scroll=move |_| offset.update(|x| *x += 5)/>
            </Show>
            <Outlet/>
        </Space>
    }
}
