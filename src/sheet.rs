use leptos::*;
use leptos_router::*;

use models::SheetShearchParams;

pub mod add;
pub mod shared;
pub mod show;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct SheetArgs {
    params: SheetShearchParams,
}

#[component]
pub fn SheetHome() -> impl IntoView {
    let params = use_params_map();
    let sheet_id = move || {
        params.with(|params| match params.get("sheet_type_id") {
            Some(id) => Uuid::from_str(id).ok(),
            None => None,
        })
    };
    let sheet_type_name_resource = create_resource(
        || (),
        move |_| async move {
            invoke::<Id, String>("sheet_type_name", &Id { id: sheet_id() })
                .await
                .unwrap_or_default()
        },
    );
    let sheet_type_name = move || match sheet_type_name_resource.read() {
        Some(name) => name,
        None => "none".to_string(),
    };

    let offset = RwSignal::from(0_u64);
    let begin = RwSignal::from(None::<NaiveDate>);
    let end = RwSignal::from(None::<NaiveDate>);
    let sheet_name = RwSignal(None::<String>);

    let search_args = move || SheetArgs {
        params: SheetShearchParams {
            offset: offset.get() as i64,
            sheet_type_name: sheet_type_name(),
            sheet_name: sheet_name.get(),
            begin: begin.get(),
            end: end.get(),
        },
    };

    let bills = create_resource(search_args, |value| async move {
        invoke::<_, Vec<Name>>("top_5_sheets", &value)
            .await
            .unwrap_or_default()
    });

    view! {
        <section>
            <A class="right-corner" href="add">
                "+"
            </A>
            <A class="left-corner" href="/">
                "->"
            </A>
            <input
                type="text"
                class="centered-input"
                placeholder=move || {
                    format! {
                        "{} ({})", "اسم الشيت", sheet_type_name()
                    }
                }
                value=move || sheet_name.get()
                on:input=move |ev| sheet_name.set(Some(event_target_value(&ev)))
            />
            <div class="date-input-container">
                <label for="start-date">"تاريخ البداية"</label>
                <input
                    type="date"
                    id="start-date"
                    value=move || begin.get().map(|x| x.to_string()).unwrap_or_else(|| "".to_string())
                    on:input=move |ev| begin.set(event_target_value(&ev).parse().ok())
                />
                <label for="end-date">"تاريخ النهاية"</label>
                <input
                    type="date"
                    id="end-date"
                    value=move || end.get().map(|x| x.to_string()).unwrap_or_else(|| "".to_string())
                    on:input=move |ev| end.set(event_target_value(&ev).parse().ok())
                />
            </div>
            <Show
                when=move || offset.get() != 0
                fallback=|_| {
                    view! {  <></> }
                }
            >
                <button on:click=move |_| offset.update(|x| *x -= 5) class="btn">
                    <span class="up-arrow">"↑"</span>
                </button>
            </Show>
            <br/>
            <br/>
            <For
                each=move || bills.read().unwrap_or_default()
                key=|s| s.id
                view=move | s| {
                    view! {
                        <A class="button" href=format!("show/{}", s.id)>
                            {s.the_name}
                        </A>
                    }
                }
            />
            <Show
                when=move || { bills.read().unwrap_or_default().len() >= 5 }
                fallback=|_| {
                    view! {  <></> }
                }
            >
                <button on:click=move |_| offset.update(|x| *x += 5) class="btn">
                    <span class="down-arrow">"↓"</span>
                </button>
            </Show>
            <Outlet/>
        </section>
    }
}
