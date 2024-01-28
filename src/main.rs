mod app;

mod atoms;
use app::*;
use leptonic::{root::Root, theme::LeptonicTheme};
use leptos::*;

fn main() {
    mount_to_body(|| {
        view! {
            <Root default_theme=LeptonicTheme::Dark>
                <App/>
            </Root>
        }
    })
}
