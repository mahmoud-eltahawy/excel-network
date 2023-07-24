use leptos::*;
use leptos_router::*;

#[component]
pub fn ShowSheet(cx: Scope) -> impl IntoView {

    view! { cx,
        <section>
            <h1>"show sheet"</h1>
            <Outlet/>
        </section>
    }
}
