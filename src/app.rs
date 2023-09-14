use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

use tauri_sys::tauri::invoke;

mod sheet;

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
use uuid::Uuid;

use std::rc::Rc;

#[component]
pub fn Home() -> impl IntoView {
    let sheets_types_names = Resource::once(|| async move {
        invoke::<Non, Rc<[Name]>>("sheets_types_names", &Non {})
            .await
            .unwrap_or(Rc::from(vec![]))
    });

    view! {
        <section>
            <br/>
            <br/>
            <For
                each=move || sheets_types_names.get().unwrap_or(Rc::from(vec![])).to_vec()
                key=|s| s.id
                view=move | s| {
                    view! {
                        <A class="button" href=format!("sheet/{}", s.id)>
                            <h1>{s.the_name}</h1>
                        </A>
                    }
                }
            />
            <Outlet/>
        </section>
    }
}
