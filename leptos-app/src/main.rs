use leptos::prelude::*;

use ankurah::{Context, Node, policy::DEFAULT_CONTEXT as C, policy::PermissiveAgent};
use ankurah_signals::{CurrentObserver, ReactiveGraphObserver};
use ankurah_storage_indexeddb_wasm::IndexedDBStorageEngine;
use ankurah_template_model::RoomView;
use ankurah_websocket_client_wasm::WebsocketClient;
use lazy_static::lazy_static;
use send_wrapper::SendWrapper;
use std::sync::{Arc, OnceLock};
use wasm_bindgen_futures::spawn_local;
use web_sys::window;

mod debug_overlay;
mod notification_manager;
mod room_list;

use debug_overlay::DebugOverlay;
use notification_manager::NotificationManager;
use room_list::RoomList;

lazy_static! {
    static ref NODE: OnceLock<Node<IndexedDBStorageEngine, PermissiveAgent>> = OnceLock::new();
    static ref CLIENT: OnceLock<SendWrapper<WebsocketClient>> = OnceLock::new();
}

/// Get the global Ankurah context.
pub fn ctx() -> Context {
    NODE.get().expect("Node not initialized").context(C).expect("failed to create context")
}

/// Get the global WebSocket client.
pub fn ws_client() -> WebsocketClient {
    (**CLIENT.get().expect("Client not initialized")).clone()
}

fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(tracing::Level::INFO) // Only show INFO, WARN, ERROR
            .build(),
    );

    // Initialize the Ankurah node and LiveQuery asynchronously, then mount Leptos.
    spawn_local(initialize());
}

async fn initialize() {
    // Open IndexedDB-backed storage and create a Node.
    let storage = IndexedDBStorageEngine::open("ankurah_template_app").await.expect("failed to open IndexedDB storage");
    let node = Node::new(Arc::new(storage), PermissiveAgent::new());

    // Build WebSocket URL based on current window location (same pattern as wasm-bindings).
    let window = window().expect("no window available");
    let location = window.location();
    let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".into());
    let ws_url = format!("ws://{}:9797", hostname);

    let client = WebsocketClient::new(node.clone(), &ws_url).expect("failed to create WebsocketClient");

    // Wait for the client to join the remote system (metadata, collections, etc.).
    node.system.wait_system_ready().await;

    // Store node and client in global statics.
    NODE.set(node).ok().expect("NODE already initialized");
    CLIENT.set(SendWrapper::new(client)).ok().expect("CLIENT already initialized");

    // Install the ReactiveGraphObserver at the base of the Ankurah observer stack
    // so that Leptos components can observe Ankurah signals via reactive_graph.
    CurrentObserver::set(ReactiveGraphObserver::new());

    leptos::mount::mount_to_body(App);
}

#[component]
pub fn App() -> impl IntoView {
    // Build the rooms LiveQuery from the global context.
    let rooms = ctx().query::<RoomView>("true ORDER BY name ASC").expect("failed to create RoomView LiveQuery");

    // UI-local state for selected room (Leptos signal, not Ankurah).
    let selected_room = RwSignal::new(None::<RoomView>);

    // Stub notification manager for unread counts.
    let notification_manager = NotificationManager::new();

    view! {
        <DebugOverlay />

        <div class="container">
            // TODO: <Header />

            <div class="mainContent">
                <RoomList rooms selected_room notification_manager />
                // TODO: <Chat room=selected_room />
            </div>
        </div>
    }
}
