use leptos::*;

#[component]
pub fn SheetHead<Fa,Fb>(
    cx: Scope,
    basic_headers : Fa,
    calc_headers : Fb,
) -> impl IntoView
where
    Fa : Fn() -> Vec<String> + 'static,
    Fb : Fn() -> Vec<String> + 'static,
{
    view! { cx,
        <thead>
            <tr>
                <For
                    each=move || basic_headers()
                    key=move |x| x.clone()
                    view=move |cx, x| {
                        view! { cx, <th>{x}</th> }
                    }
                />
                <For
                    each=move || calc_headers()
                    key=move |x| x.clone()
                    view=move |cx, x| {
                        view! { cx, <th>{x}</th> }
                    }
                />
            </tr>
        </thead>
    }
}
