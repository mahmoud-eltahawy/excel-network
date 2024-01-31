use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

use tauri_sys::tauri::invoke;

pub mod sheet;

use sheet::{add::AddSheet, show::ShowSheet, SheetHome};

#[derive(Serialize, Deserialize)]
pub struct Non;

#[derive(Serialize, Deserialize)]
pub struct Id {
    id: Option<Uuid>,
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <main>
            <Router>
                <section>
                    <Routes>
                        <Route
                            path="/"
                            view=|| {
                                view! {  <Home/> }
                            }
                        />
                        <Route
                            path="/sheet/:sheet_type_id"
                            view=|| {
                                view! {  <SheetHome/> }
                            }
                        />
                        <Route
                            path="/sheet/:sheet_type_id/show/:sheet_id"
                            view=|| {
                                view! {  <ShowSheet/> }
                            }
                        />
                        <Route
                            path="/sheet/:sheet_type_id/add"
                            view=|| {
                                view! {  <AddSheet/> }
                            }
                        />
                    </Routes>
                </section>
            </Router>
        </main>
    }
}
use models::Name;
use thaw::{Button, ButtonSize, Space, SpaceGap};
use uuid::Uuid;

use std::rc::Rc;

#[component]
pub fn Home() -> impl IntoView {
    let sheets_types_names = Resource::once(|| async move {
        invoke::<Non, Rc<[Name<Uuid>]>>("sheets_types_names", &Non {})
            .await
            .unwrap_or(Rc::from(vec![]))
    });

    let button_style = r#"
        width: 80%;
        font-size : 2rem;
    "#
    .trim();

    let div_style = r#"
      margin: 0;
      position: absolute;
      top: 50%;
      -ms-transform: translateY(-50%);
      transform: translateY(-50%);    
      width: 100%;
    "#
    .trim();

    view! {
        <div style=div_style>
        <Space vertical=true gap=SpaceGap::WH(150,40)>
            <For
                each=move || sheets_types_names.get().unwrap_or(Rc::from(vec![])).to_vec()
                key=|s| s.id
                children=move |s| {
                    view! {
                        <Button
                            on_click=move |_| {
                                window()
                                    .location()
                                    .set_href(&format!("sheet/{}", s.id))
                                    .unwrap_or_default();
                            }
                            style={button_style}
                            size=ButtonSize::Large
                        >{s.the_name}</Button>
                        <br/>
                        <br/>
                        <br/>
                        <br/>
                    }
                }
            />
        </Space>
        <Outlet/>
        </div>
    }
}
