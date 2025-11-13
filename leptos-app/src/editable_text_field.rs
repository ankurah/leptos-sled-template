use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use ankurah_signals::Get as AnkurahGet;

use crate::ctx;

/// Editable text field that applies operational transforms for collaborative editing.
/// Switches between display and edit modes on click/blur.
///
/// TODO: This is currently a stub that doesn't apply operational transforms.
/// The full implementation needs to call view.edit(trx) and apply YrsStringString operations.
#[component]
pub fn EditableTextField(
    /// The current value to display
    value: String,
    /// Callback when value changes
    on_change: impl Fn(String) + Clone + Send + Sync + 'static,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] class: Option<String>,
) -> impl IntoView {
    let is_editing = RwSignal::new(false);
    let local_value = RwSignal::new(String::new());
    let cursor_pos = RwSignal::new(0);
    let last_value = RwSignal::new(String::new());
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let placeholder = placeholder.unwrap_or_else(|| "Click to edit".to_string());
    let class_name = class.unwrap_or_default();

    // Focus and set cursor position when entering edit mode
    Effect::new({
        let input_ref = input_ref.clone();
        move |_| {
            if is_editing.get() {
                if let Some(input_el) = input_ref.get() {
                    let _ = input_el.focus();
                    let pos = cursor_pos.get() as u32;
                    let _ = input_el.set_selection_range(pos, pos);
                }
            }
        }
    });

    let start_edit = {
        let value = value.clone();
        move |_| {
            local_value.set(value.clone());
            last_value.set(value.clone());
            cursor_pos.set(value.len());
            is_editing.set(true);
        }
    };

    let apply_changes = {
        let on_change = on_change.clone();
        move |_old_value: String, new_value: String| {
            on_change(new_value);
        }
    };

    let handle_change = {
        let apply_changes = apply_changes.clone();
        move |ev: web_sys::Event| {
            let target = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok());
            if let Some(input) = target {
                let new_value = input.value();
                let new_cursor_pos = input.selection_start().ok().flatten().unwrap_or(0) as usize;

                apply_changes(last_value.get(), new_value.clone());

                local_value.set(new_value.clone());
                last_value.set(new_value);
                cursor_pos.set(new_cursor_pos);
            }
        }
    };

    let end_edit = move || {
        is_editing.set(false);
        local_value.set(String::new());
        last_value.set(String::new());
    };

    let handle_key_down = move |ev: KeyboardEvent| {
        if ev.key() == "Enter" || ev.key() == "Escape" {
            ev.prevent_default();
            end_edit();
        }
    };

    view! {
        <Show
            when=move || is_editing.get()
            fallback={
                let value = value.clone();
                let placeholder = placeholder.clone();
                let class_name = class_name.clone();
                let start_edit = start_edit.clone();
                move || {
                    let display = if value.is_empty() { placeholder.clone() } else { value.clone() };
                    view! {
                        <span
                            class=format!("editableText {}", class_name)
                            on:click=start_edit.clone()
                            title=placeholder.clone()
                        >
                            {display}
                        </span>
                    }
                }
            }
        >
            {
                let handle_change = handle_change.clone();
                let handle_key_down = handle_key_down.clone();
                let end_edit = end_edit.clone();
                let class_name = class_name.clone();
                move || view! {
                    <input
                        node_ref=input_ref
                        type="text"
                        class=format!("editableInput {}", class_name)
                        prop:value=move || local_value.get()
                        on:input=handle_change.clone()
                        on:keydown=handle_key_down.clone()
                        on:blur=move |_| end_edit()
                    />
                }
            }
        </Show>
    }
}

