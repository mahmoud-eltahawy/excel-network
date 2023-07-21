use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

use tauri_sys::tauri::invoke;

#[derive(Serialize, Deserialize)]
pub struct Non;

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
                        path="/sheet/:name"
                        view=|cx| {
                            view! { cx, <Sheet/> }
                        }
                    />
                </Routes>
            </section>
        </Router>
      </main>
    }
}


#[component]
pub fn Home(cx: Scope) -> impl IntoView {
    let sheets_names = create_resource(
	cx,
	|| (),
	|_| async move {
            invoke::<Non, Vec<String>>("sheets_names", &Non{})
            .await
	    .unwrap_or_default()
    });

    view! {cx,
	 <section>
            <For
                each=move || sheets_names.read(cx).unwrap_or_default()
                key=|s| s.clone()
                view=move |cx, s| {
                    view! { cx,
                        <A class="button" href=format!("sheet/{}", s)>
			    <h1>{s}</h1>
                        </A>
                    }
                }
            />
	   <Outlet/>
	 </section>
    }
}


#[component]
pub fn Sheet(cx: Scope) -> impl IntoView {
    let params = use_params_map(cx);
    let sheet_name = move || {
	let s = params.
	with(|params| params.get("name").cloned())
	.unwrap_or_default();
	
	log!("{:#?}",s);
	s
    };
    // let sheets_names = create_resource(
    // 	cx,
    // 	|| (),
    // 	|_| async move {
    //         invoke::<Non, Vec<String>>("sheets_names", &Non{})
    //         .await
    // 	    .unwrap_or_default()
    // });

    view! {cx,
	 <section>
	   <h1>"hello world"</h1>
	   <h1>{sheet_name()}</h1>
	   <Outlet/>
	 </section>
    }
}
