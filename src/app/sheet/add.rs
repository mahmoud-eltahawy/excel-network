use leptos::*;
use leptos_router::*;
use std::str::FromStr;

use uuid::Uuid;
use tauri_sys::tauri::invoke;
use crate::Id;

#[component]
pub fn AddSheet(cx: Scope) -> impl IntoView {
    let params = use_params_map(cx);
    let sheet_id = move || params.
	with(|params| Uuid::from_str(params.get("sheet_type_id").unwrap()))
	.unwrap_or_default();
    let sheet_type_name_resource = create_resource(
	cx,
	|| () ,
	move |_| async move {
            invoke::<Id, String>("sheet_type_name", &Id{id : sheet_id()})
            .await
	    .unwrap_or_default()
    });

    let sheet_type_name = move || match sheet_type_name_resource.read(cx) {
	Some(name) => name,
	None => "none".to_string()
    };

    view!{cx,
	<section>
	  <h1>"add sheet"</h1>
	  <h1>{sheet_type_name}</h1>
	   <Outlet/>
	</section>
    }
}
