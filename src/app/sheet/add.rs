use leptos::*;
use leptos_router::*;
use models::{ConfigValue, HeaderGetter};
use serde::{Serialize, Deserialize};
use std::str::FromStr;

use super::shared::SheetHead;

use uuid::Uuid;
use tauri_sys::tauri::invoke;
use crate::Id;

#[derive(Debug,Serialize,Deserialize)]
struct NameArg{
    name : String
}

#[component]
pub fn AddSheet(cx: Scope) -> impl IntoView {
    let (sheet_name, set_sheet_name) = create_signal(cx, String::from(""));
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
	Some(type_name) => type_name,
	None => "none".to_string()
    };

    let sheet_headers_resource = create_resource(
	cx,
	sheet_type_name,
	move |name| async move {
            invoke::<NameArg, Vec<ConfigValue>>("sheet_headers", &NameArg{name})
            .await
	    .unwrap_or_default()
    });

    let headers =move || sheet_headers_resource
	  .read(cx)
	  .unwrap_or_default()
	  .into_iter().map(|x| match x {
		  ConfigValue::Basic(conf) => conf.get_header(),
		  ConfigValue::Calculated(conf) => conf.header
	      }).collect::<Vec<_>>();

    view!{cx,
	<section>
          <A class="left-corner" href=format!("/sheet/{}",sheet_id())>
                "->"
            </A>
            <br/>
            <input
                type="string"
                class="centered-input"
                placeholder= move|| format!("{} ({})","اسم الشيت",sheet_type_name())
                value=move || sheet_name.get()
                on:input=move |ev| set_sheet_name.set(event_target_value(&ev))
            />
	  <table>
	  <SheetHead headers=headers/>
	  </table>
	   <Outlet/>
	</section>
    }
}
