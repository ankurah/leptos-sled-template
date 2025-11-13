use leptos::prelude::*;
use web_sys::window;

use ankurah_signals::Get as AnkurahGet;
use ankurah_template_model::UserView;

use crate::{editable_text_field::EditableTextField, qr_code_modal::QRCodeModal, ws_client};

/// Header component displaying app title, user info, connection status, and QR code button.
#[component]
pub fn Header(current_user: RwSignal<Option<UserView>>) -> impl IntoView {
    let show_qr_code = RwSignal::new(false);

    // Get connection state from WebSocket client
    // TODO: Properly observe connection state changes
    let connection_status = move || "Connected".to_string();

    let current_url = window().and_then(|w| w.location().href().ok()).unwrap_or_default();

    view! {
        <>
            <div class="header">
                <h1 class="title">"ankurah-template Chat"</h1>
                <div class="headerRight">
                    <button
                        class="qrButton"
                        on:click=move |_| show_qr_code.set(true)
                        title="Show QR Code"
                    >
                        "📱"
                    </button>
                    <div class="userInfo">
                        <span>"👤"</span>
                        <Show
                            when=move || current_user.get().is_some()
                            fallback=|| view! { <span class="userName">"Loading..."</span> }
                        >
                            {move || {
                                current_user.get().map(|user| {
                                    let display_name = user.display_name().unwrap_or_default();
                                    view! {
                                        <EditableTextField
                                            value=display_name.clone()
                                            on_change=move |new_name: String| {
                                                // TODO: Update user display_name via transaction
                                                tracing::info!("Would update display_name to: {}", new_name);
                                            }
                                            class="userName".to_string()
                                        />
                                    }
                                })
                            }}
                        </Show>
                    </div>
                    <div class=move || {
                        let status = connection_status();
                        if status == "Connected" {
                            "connectionStatus connected"
                        } else {
                            "connectionStatus disconnected"
                        }
                    }>
                        {move || {
                            let status = connection_status();
                            if status.is_empty() { "Disconnected".to_string() } else { status }
                        }}
                    </div>
                </div>
            </div>
            <Show when=move || show_qr_code.get()>
                <QRCodeModal url=current_url.clone() on_close=move || show_qr_code.set(false) />
            </Show>
        </>
    }
}
