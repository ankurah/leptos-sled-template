use leptos::prelude::*;
use web_sys::KeyboardEvent;

use ankurah::model::Mutable;
use {{crate_name}}_model::{Message, MessageView, RoomView, UserView};

use crate::{chat_scroll_manager::ChatScrollManager, ctx};

/// Message input component for sending and editing messages.
/// Handles Enter to send, Escape to cancel edit, Cmd/Ctrl+Up/Down to navigate own messages.
#[component]
pub fn MessageInput(
    room: RoomView,
    current_user: Option<UserView>,
    editing_message: RwSignal<Option<MessageView>>,
    #[prop(optional)] manager: Option<ChatScrollManager>,
) -> impl IntoView {
    let message_input = RwSignal::new(String::new());

    // TODO: Get connection state from WebSocket client
    let connection_state = move || "Connected".to_string();

    // Update input when editing message changes
    Effect::new({
        let message_input = message_input.clone();
        move |_| {
            if let Some(edit_msg) = editing_message.get() {
                message_input.set(edit_msg.text().unwrap_or_default());
            } else {
                message_input.set(String::new());
            }
        }
    });

    let handle_send_message = move || {
        let input_text = message_input.get();
        if input_text.trim().is_empty() || current_user.is_none() {
            tracing::info!("Cannot send: no input or no user");
            return;
        }

        let Some(user) = current_user.clone() else { return };

        if let Some(edit_msg) = editing_message.get() {
            // Edit existing message
            let input_text = input_text.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match (|| async {
                    let trx = ctx().begin();
                    let mutable = edit_msg.edit(&trx)?;
                    mutable.text().replace(&input_text.trim());
                    trx.commit().await?;
                    Ok::<_, Box<dyn std::error::Error>>(())
                })()
                .await
                {
                    Ok(_) => {
                        tracing::info!("Message updated");
                        editing_message.set(None);
                        message_input.set(String::new());
                    }
                    Err(e) => tracing::error!("Failed to update message: {}", e),
                }
            });
        } else {
            // Create new message
            let room_id = room.id().to_base64();
            let user_id = user.id().to_base64();
            let input_text = input_text.clone();

            wasm_bindgen_futures::spawn_local(async move {
                match (|| async {
                    let transaction = ctx().begin();
                    let timestamp = js_sys::Date::now() as i64;
                    let _msg = transaction
                        .create(&Message {
                            user: user_id.clone(),
                            room: room_id.clone(),
                            text: input_text.trim().to_string(),
                            timestamp,
                            deleted: false,
                        })
                        .await?;
                    transaction.commit().await?;
                    Ok::<_, Box<dyn std::error::Error>>(())
                })()
                .await
                {
                    Ok(_) => {
                        tracing::info!("Message sent");
                        message_input.set(String::new());
                        // TODO: Jump to live mode when manager is implemented
                        // manager?.jump_to_live().await;
                    }
                    Err(e) => tracing::error!("Failed to send message: {}", e),
                }
            });
        }
    };

    let handle_key_down = {
        let handle_send_message = handle_send_message.clone();
        move |e: KeyboardEvent| {
            if e.key() == "Enter" && !e.shift_key() {
                e.prevent_default();
                handle_send_message();
            } else if e.key() == "Escape" && editing_message.get().is_some() {
                e.prevent_default();
                editing_message.set(None);
                message_input.set(String::new());
            } else if e.key() == "ArrowUp" && (e.meta_key() || e.ctrl_key()) {
                e.prevent_default();
                // TODO: Navigate to previous own message
                tracing::info!("ArrowUp navigation - TODO");
            } else if e.key() == "ArrowDown" && (e.meta_key() || e.ctrl_key()) && editing_message.get().is_some() {
                e.prevent_default();
                // TODO: Navigate to next own message
                tracing::info!("ArrowDown navigation - TODO");
            }
        }
    };

    let is_connected = move || connection_state() == "Connected";
    let can_send = move || !message_input.get().trim().is_empty() && is_connected();

    view! {
        <div class="inputContainer">
            <input
                type="text"
                class="input"
                placeholder="Type a message..."
                prop:value=move || message_input.get()
                on:input=move |ev| message_input.set(event_target_value(&ev))
                on:keydown=handle_key_down
                prop:disabled=move || !is_connected()
            />
            <button class="button" on:click=move |_| handle_send_message() prop:disabled=move || !can_send()>
                {move || if editing_message.get().is_some() { "Update" } else { "Send" }}
            </button>
            <Show when=move || editing_message.get().is_some()>
                <button
                    class="button"
                    on:click=move |_| {
                        editing_message.set(None);
                        message_input.set(String::new());
                    }

                    style="margin-left: 8px"
                >
                    "Cancel"
                </button>
            </Show>
        </div>
    }
}
