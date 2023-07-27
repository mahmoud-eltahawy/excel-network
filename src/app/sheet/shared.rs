use leptos::*;

use crate::Non;
use tauri_sys::{
    dialog::{MessageDialogBuilder, MessageDialogKind},
    tauri::invoke,
};
use uuid::Uuid;

pub async fn new_id() -> Uuid {
    invoke::<_, Uuid>("new_id", &Non {}).await.unwrap()
}

pub async fn alert(message: &str) {
    let mut builder = MessageDialogBuilder::new();
    builder.set_title("تحذير");
    builder.set_kind(MessageDialogKind::Warning);

    builder.message(message).await.unwrap_or_default();
}

#[component]
pub fn SheetHead<Fa, Fb>(cx: Scope, basic_headers: Fa, calc_headers: Fb) -> impl IntoView
where
    Fa: Fn() -> Vec<String> + 'static,
    Fb: Fn() -> Vec<String> + 'static,
{
    view! { cx,
        <thead>
            <tr>
                <For
                    each=basic_headers
                    key=move |x| x.clone()
                    view=move |cx, x| {
                        view! { cx, <th>{x}</th> }
                    }
                />
                <For
                    each=calc_headers
                    key=move |x| x.clone()
                    view=move |cx, x| {
                        view! { cx, <th>{x}</th> }
                    }
                />
            </tr>
        </thead>
    }
}
