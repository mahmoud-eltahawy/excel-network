use leptos::*;

#[component]
pub fn SheetHead<F>(
    cx: Scope,
    headers : F,
) -> impl IntoView
    where F : Fn() -> Vec<String> + 'static 
{
    view! { cx,
        <thead>
            <tr>
	    <For
	    each=move || headers()
	    key=move |x| x.clone()
	    view=move |cx,x| view!{cx,
		<th>{x}</th>
	    }
	    />
            </tr>
        </thead>
    }
}
