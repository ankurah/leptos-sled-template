use leptos::prelude::*;

use ankurah::LiveQuery;
use ankurah_signals::Get as AnkurahGet;
use {{crate_name}}_model::{MessageView, RoomView, UserView};

use crate::{
    chat_debug_header::ChatDebugHeader, chat_scroll_manager::ChatScrollManager, ctx, message_input::MessageInput, message_row::MessageRow,
    notification_manager::NotificationManager,
};

/// Main chat component displaying messages, input, and scroll controls.
/// Manages ChatScrollManager lifecycle and coordinates all chat sub-components.
#[component]
pub fn Chat(room: RwSignal<Option<RoomView>>, current_user: RwSignal<Option<UserView>>, notification_manager: NotificationManager) -> impl IntoView {
    let show_debug = RwSignal::new(false);
    let editing_message = RwSignal::new(None::<MessageView>);

    // TODO: Create ChatScrollManager when room changes
    // For now, we'll create a dummy LiveQuery
    let manager: Option<ChatScrollManager> = None;

    // Query for all users
    let users = ctx().query::<UserView>("").expect("failed to create UserView LiveQuery");

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
            {move || {
                room.get().map(|current_room| {
                    let current_user_id = current_user.get().map(|u| u.id().to_base64());

                    view! {
                        <div class="chatContainer">
                            // Debug header (shown when show_debug is true)
                            // TODO: Show ChatDebugHeader when manager is implemented
                            // <Show when=move || show_debug.get() && manager.is_some()>
                            //     {move || manager.as_ref().map(|m| view! { <ChatDebugHeader manager=m.clone() /> })}
                            // </Show>

                            // Debug toggle button
                            // TODO: Show when manager is implemented
                            // <Show when=move || manager.is_some()>
                            //     <button
                            //         class="debugToggle"
                            //         on:click=move |_| show_debug.update(|v| *v = !*v)
                            //         title=move || if show_debug.get() { "Hide debug info" } else { "Show debug info" }
                            //         style="opacity: 0.35"
                            //     >
                            //         {move || if show_debug.get() { "▼" } else { "▲" }}
                            //     </button>
                            // </Show>

                            // Messages container
                            <div class="messagesContainer">
                                // TODO: Bind container to manager
                                // TODO: Get message list from manager.items()
                                // For now, show empty state
                                <div class="emptyState">"No messages yet. Be the first to say hello!"</div>

                                // TODO: Map over messageList
                                // <For
                                //     each=move || message_list
                                //     key=|message: &MessageView| message.id()
                                //     children=move |message: MessageView| {
                                //         view! {
                                //             <MessageRow
                                //                 message=message
                                //                 users=users.clone()
                                //                 current_user_id=current_user_id.clone()
                                //                 editing_message=editing_message
                                //             />
                                //         }
                                //     }
                                // />
                            </div>

                            // Jump to current button (shown when not at bottom)
                            // TODO: Show when manager.should_auto_scroll is false
                            // <Show when=move || manager.as_ref().map(|m| !m.should_auto_scroll()).unwrap_or(false)>
                            //     <button class="jumpToCurrent" on:click=move |_| {
                            //         // TODO: manager.jump_to_live()
                            //     }>
                            //         "Jump to Current ↓"
                            //     </button>
                            // </Show>

                            // Message input
                            <MessageInput room=current_room current_user=current_user.get() editing_message=editing_message />
                        </div>
                    }
                })
            }}
        </Show>
    }
}


