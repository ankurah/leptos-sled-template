use leptos::html::Div;
use leptos::prelude::*;

use ankurah_signals::Get as AnkurahGet;
use {{crate_name}}_model::{MessageView, RoomView, UserView};

use crate::{
    chat_debug_header::ChatDebugHeader, chat_scroll_manager::ChatScrollManager, ctx, message_input::MessageInput,
    message_list::MessageList, notification_manager::NotificationManager,
};

/// Main chat component displaying messages, input, and scroll controls.
/// Manages ChatScrollManager lifecycle and coordinates all chat sub-components.
#[component]
pub fn Chat(
    room: RwSignal<Option<RoomView>>,
    current_user: RwSignal<Option<UserView>>,
    notification_manager: NotificationManager,
) -> impl IntoView {
    let show_debug = RwSignal::new(false);
    let editing_message = RwSignal::new(None::<MessageView>);

    // Create ChatScrollManager when room changes (wrapped in SendWrapper for Leptos compatibility)
    let manager = RwSignal::new(None::<ChatScrollManager>);

    // Update manager when room changes
    Effect::new({
        let manager = manager.clone();
        let notification_manager = notification_manager.clone();
        move |_| {
            if let Some(current_room) = room.get() {
                let room_id = current_room.id().to_base64();
                let new_manager = ChatScrollManager::new(room_id, notification_manager.clone());

                // Clean up old manager before setting new one (use untracked to avoid loop)
                manager.update_untracked(|old| {
                    if let Some(old_manager) = old.take() {
                        old_manager.destroy();
                    }
                    *old = Some(new_manager);
                });
            } else {
                // Clean up old manager
                manager.update_untracked(|old| {
                    if let Some(old_manager) = old.take() {
                        old_manager.destroy();
                    }
                });
            }
        }
    });

    // Query for all users
    let users = ctx().query::<UserView>("true").expect("failed to create UserView LiveQuery");

    let messages_container_ref = NodeRef::<Div>::new();

    // Bind container to scroll manager after it's rendered
    Effect::new({
        let manager = manager.clone();
        let messages_container_ref = messages_container_ref.clone();
        move |_| {
            // Track manager changes, but don't track the container ref
            if let Some(m) = manager.get() {
                // Use get_untracked to avoid creating a dependency on the NodeRef
                m.bind_container(messages_container_ref.get_untracked());
            }
        }
    });

    // Call after_layout when messages change
    // Note: We track the manager itself, and when it changes we set up a new effect for its messages
    Effect::new({
        let manager = manager.clone();
        move |prev_count: Option<usize>| {
            if let Some(m) = manager.get_untracked() {
                // Track message changes by getting the count
                let messages = m.messages();
                let count = messages.get().len();

                // Only call after_layout if count actually changed (not on first run)
                if let Some(prev) = prev_count {
                    if count != prev {
                        let m = m.clone();
                        leptos::task::spawn_local(async move {
                            leptos::task::tick().await;
                            m.after_layout();
                        });
                    }
                }

                count
            } else {
                0
            }
        }
    });

    view! {
        <Show
            when=move || room.get().is_some()
            fallback=|| {
                view! {
                    <div class="chatContainer">
                        <div class="emptyState">"Select a room to start chatting"</div>
                    </div>
                }
            }
        >
            {
                let room = room.clone();
                let manager = manager.clone();
                let current_user = current_user.clone();
                let users = users.clone();
                let editing_message = editing_message.clone();
                let messages_container_ref = messages_container_ref.clone();
                let show_debug = show_debug.clone();
                move || room.get().and_then(|current_room| {
                    manager.get().map(|mgr| {
                        let current_room_for_input = current_room.clone();
                        let current_user_id = current_user.get().map(|u| u.id().to_base64());
                        let show_jump_to_current = !mgr.should_auto_scroll();

                        // Clone manager for all usages before view! macro
                        let mgr1 = mgr.clone();
                        let mgr2 = mgr.clone();
                        let mgr3 = mgr.clone();
                        let mgr4 = mgr;

                        view! {
                            <div class="chatContainer">
                                // Debug header
                                <Show when=move || show_debug.get()>
                                    {{
                                        let mgr1 = mgr1.clone();
                                        move || view! { <ChatDebugHeader manager=mgr1.clone() /> }
                                    }}
                                </Show>

                                // Debug toggle button
                                <button
                                    class="debugToggle"
                                    on:click=move |_| show_debug.update(|v| *v = !*v)
                                    title=move || if show_debug.get() { "Hide debug info" } else { "Show debug info" }
                                    style="opacity: 0.35;"
                                >
                                    {move || if show_debug.get() { "▼" } else { "▲" }}
                                </button>

                                // Messages container
                                <div class="messagesContainer" node_ref=messages_container_ref>
                                    <MessageList
                                        messages=Signal::derive(move || mgr2.items())
                                        users=users.clone()
                                        current_user_id=current_user_id.clone()
                                        editing_message=editing_message
                                    />
                                </div>

                                // Jump to current button
                                <Show when=move || show_jump_to_current>
                                    {{
                                        let mgr3 = mgr3.clone();
                                        move || {
                                            let mgr3 = mgr3.clone();
                                            view! {
                                                <button class="jumpToCurrent" on:click=move |_| mgr3.jump_to_live()>
                                                    "Jump to Current ↓"
                                                </button>
                                            }
                                        }
                                    }}
                                </Show>

                                // Message input
                                <MessageInput
                                    room=current_room_for_input
                                    current_user=current_user.get()
                                    editing_message=editing_message
                                    manager=mgr4
                                />
                            </div>
                        }
                    })
                })
            }
        </Show>
    }
}
