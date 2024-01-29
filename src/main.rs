mod app;

mod atoms;
use app::*;
use leptonic::{root::Root, theme::LeptonicTheme};
use leptos::{logging::log, *};

fn main() {
    let id = uuid::Uuid::new_v4();
    log!("{}", id);
    mount_to_body(|| {
        view! {
            <Root default_theme=LeptonicTheme::Dark>
                <App/>
            </Root>
        }
    })
}
