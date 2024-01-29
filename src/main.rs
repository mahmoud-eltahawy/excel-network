mod app;

mod atoms;
use app::*;
use leptos::*;
use thaw::{Theme, ThemeProvider};

fn main() {
    let theme = RwSignal::new(Theme::light());
    mount_to_body(move || {
        view! {
            <ThemeProvider theme=theme>
                <App/>
            </ThemeProvider>
        }
    })
}
