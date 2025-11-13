use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
pub fn App() -> impl IntoView {
    view! { <p>"Hello, world!"</p> }
}
