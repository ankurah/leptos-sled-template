use leptos::prelude::*;

use ankurah_signals::Get as AnkurahGet;

use crate::chat_scroll_manager::{ChatScrollManager, LoadingDirection};

/// Debug header showing scroll manager state and metrics.
/// Displays mode, loading state, buffer gaps, and boundary status.
#[component]
pub fn ChatDebugHeader(manager: ChatScrollManager) -> impl IntoView {
    let mode = manager.mode();
    let mode_for_class = mode.clone();
    let loading = manager.loading();
    let loading_for_backward = loading.clone();
    let loading_for_forward = loading.clone();
    let metrics = manager.metrics();
    let metrics_for_top_class = metrics.clone();
    let metrics_for_top_text = metrics.clone();
    let metrics_for_bottom_class = metrics.clone();
    let metrics_for_bottom_text = metrics.clone();
    let metrics_for_thresholds = metrics.clone();
    let metrics_for_results = metrics.clone();

    let format_gap = |gap: f64, trigger: f64| {
        let rounded = gap.round() as i32;
        let pct = ((gap / trigger) * 100.0).round() as i32;
        format!("{}px ({}%)", rounded, pct)
    };

    let will_trigger = |gap: f64, trigger: f64| gap < trigger;

    view! {
        <div class="debugHeader">
            <div class="debugRow">
                <span class="debugLabel">"Query:"</span>
                <span class="debugValue queryText">
                    // TODO: Get current selection from manager.messages
                    "room = ? AND deleted = false ORDER BY timestamp DESC LIMIT ?"
                </span>
            </div>
            <div class="debugRow">
                <span class="debugLabel">"Mode:"</span>
                <span class=move || format!("debugValue mode-{:?}", mode_for_class.get()).to_lowercase()>
                    {move || format!("{:?}", mode.get())}
                </span>
                <span class="debugLabel">"Results:"</span>
                <span class="debugValue">{move || metrics_for_results.get().result_count}</span>
                <span class="debugLabel">"Thresholds:"</span>
                <span class="debugValue">
                    {move || {
                        let m = metrics_for_thresholds.get();
                        format!("trigger={}px, anchor={}px", m.min_buffer.round() as i32, m.step_back.round() as i32)
                    }}
                </span>
            </div>
            <div class="debugRow">
                <span class="debugLabel">"Buffer ↑:"</span>
                <span class=move || {
                    let m = metrics_for_top_class.get();
                    if will_trigger(m.top_gap, m.min_buffer) {
                        "debugValue trigger-active"
                    } else {
                        "debugValue"
                    }
                }>
                    {move || {
                        let m = metrics_for_top_text.get();
                        format_gap(m.top_gap, m.min_buffer)
                    }}
                </span>
                <span class="debugStatus">
                    {move || {
                        if loading_for_backward.get().as_ref() == Some(&LoadingDirection::Backward) {
                            Some(view! { <span style="display: inline-block; animation: spin 1s linear infinite">"◐"</span> })
                        } else {
                            None
                        }
                    }}
                </span>
                <span class="debugLabel">"Buffer ↓:"</span>
                <span class=move || {
                    let m = metrics_for_bottom_class.get();
                    if will_trigger(m.bottom_gap, m.min_buffer) {
                        "debugValue trigger-active"
                    } else {
                        "debugValue"
                    }
                }>
                    {move || {
                        let m = metrics_for_bottom_text.get();
                        format_gap(m.bottom_gap, m.min_buffer)
                    }}
                </span>
                <span class="debugStatus">
                    {move || {
                        if loading_for_forward.get().as_ref() == Some(&LoadingDirection::Forward) {
                            Some(view! { <span style="display: inline-block; animation: spin 1s linear infinite">"◐"</span> })
                        } else {
                            None
                        }
                    }}
                </span>
            </div>
            <div class="debugRow">
                <span class="debugLabel">"Boundaries:"</span>
                <span class="debugValue">
                    // TODO: Implement boundary detection with reactive signals
                    "← earliest"
                </span>
                <span class="debugValue">
                    // TODO: Implement boundary detection with reactive signals
                    "latest →"
                </span>
            </div>
        </div>
    }
}
