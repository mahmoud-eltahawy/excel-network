use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

use tauri_sys::tauri::invoke;

mod sheet;

use sheet::{SheetHome,show::ShowSheet,add::AddSheet};

#[derive(Serialize, Deserialize)]
pub struct Non;

#[derive(Serialize, Deserialize)]
pub struct Id{id : Uuid}

#[component]
pub fn App(cx: Scope) -> impl IntoView {

    view! { cx,
      <main>
        <Router>
            <section>
                <Routes>
                    <Route
                        path="/"
                        view=|cx| {
                            view! { cx, <Home/> }
                        }
                    />
                    <Route
                        path="/sheet/:sheet_type_id"
                        view=|cx| {
                            view! { cx, <SheetHome/> }
                        }
                    />
                    <Route
                        path="/sheet/:sheet_type_id/show/:sheet_id"
                        view=|cx| {
                            view! { cx, <ShowSheet/> }
                        }
                    />
                    <Route
                        path="/sheet/:sheet_type_id/add"
                        view=|cx| {
                            view! { cx, <AddSheet/> }
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

#[component]
pub fn Home(cx: Scope) -> impl IntoView {
    let sheets_types_names = create_resource(
	cx,
	|| (),
	|_| async move {
            invoke::<Non, Vec<Name>>("sheets_types_names", &Non{})
            .await
	    .unwrap_or_default()
    });

    view! {cx,
	 <section>
	   <br/>
	   <br/>
            <For
                each=move || sheets_types_names.read(cx).unwrap_or_default()
                key=|s| s.id
                view=move |cx, s| {
                    view! { cx,
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
