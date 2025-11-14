use ankurah::LiveQuery;
use ankurah_signals::{Get as AnkurahGet, Mut, Peek, Read, Subscribe, SubscriptionGuard};
use {{crate_name}}_model::MessageView;
use send_wrapper::SendWrapper;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlDivElement, HtmlElement, window};

use crate::ctx;
use crate::notification_manager::NotificationManager;

#[derive(Debug, Clone, PartialEq)]
pub enum ScrollMode {
    Live,
    Backward,
    Forward,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoadingDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone)]
pub struct ScrollMetrics {
    pub top_gap: f64,
    pub bottom_gap: f64,
    pub min_buffer: f64,
    pub step_back: f64,
    pub result_count: usize,
}

/// ChatScrollManager handles virtual scrolling and pagination for chat messages.
/// Ported from TypeScript to be cross-framework compatible using ankurah_signals types.
/// Uses Rc wrapped in SendWrapper to work with Leptos's Send requirements in WASM.
#[derive(Clone)]
pub struct ChatScrollManager(SendWrapper<Rc<Inner>>);

struct Inner {
    // Configuration (in fractional screen height units)
    min_row_px: f64,
    min_buffer_size: f64,
    continuation_step_back: f64,
    query_size: f64,

    // Room context
    room_id: String,
    notification_manager: NotificationManager,

    // Reactive state
    mode: Mut<ScrollMode>,
    loading: Mut<Option<LoadingDirection>>,
    metrics: Mut<ScrollMetrics>,
    messages: LiveQuery<MessageView>,

    // Track query parameters (for boundary detection)
    current_limit: Mut<usize>,
    current_direction: Mut<String>, // "ASC" or "DESC"

    // Scroll state
    last_continuation_key: RefCell<Option<String>>,
    last_scroll_top: RefCell<f64>,
    user_scrolling: RefCell<bool>,
    initialized: RefCell<bool>,

    // DOM binding
    container: RefCell<Option<HtmlDivElement>>,
    scroll_closure: RefCell<Option<Closure<dyn FnMut()>>>,
    wheel_closure: RefCell<Option<Closure<dyn FnMut()>>>,
    touch_closure: RefCell<Option<Closure<dyn FnMut()>>>,

    // Subscription guard
    _guard: SubscriptionGuard,
}

impl ChatScrollManager {
    pub fn new(room_id: String, notification_manager: NotificationManager) -> Self {
        let mode = Mut::new(ScrollMode::Live);
        let loading = Mut::new(None);
        let metrics = Mut::new(ScrollMetrics { top_gap: 0.0, bottom_gap: 0.0, min_buffer: 0.0, step_back: 0.0, result_count: 0 });

        let current_limit = Mut::new(100); // Default limit, will be updated
        let current_direction = Mut::new("DESC".to_string());

        // Create initial live mode query
        let limit = 100; // Will be recomputed after container is bound
        let predicate = format!("room = '{}' AND deleted = false ORDER BY timestamp DESC LIMIT {}", room_id, limit);
        let messages = ctx().query::<MessageView>(predicate.as_str()).expect("failed to create MessageView LiveQuery");

        // Subscribe to message changes
        // TODO: Call afterLayout on message updates (requires capturing self in closure)
        let _guard = messages.subscribe(move |_| {
            // Schedule afterLayout on next tick (after DOM updates)
            // For now this is a no-op; afterLayout will be called manually after render
        });

        // Set as active room since rooms start in live mode
        notification_manager.set_active_room(Some(room_id.clone()));

        let inner = Inner {
            min_row_px: 74.0,
            min_buffer_size: 0.75,
            continuation_step_back: 0.75,
            query_size: 3.0,

            room_id,
            notification_manager,

            mode,
            loading,
            metrics,
            messages,

            current_limit,
            current_direction,

            last_continuation_key: RefCell::new(None),
            last_scroll_top: RefCell::new(0.0),
            user_scrolling: RefCell::new(false),
            initialized: RefCell::new(false),

            container: RefCell::new(None),
            scroll_closure: RefCell::new(None),
            wheel_closure: RefCell::new(None),
            touch_closure: RefCell::new(None),

            _guard,
        };

        Self(SendWrapper::new(Rc::new(inner)))
    }

    pub fn mode(&self) -> Read<ScrollMode> {
        self.0.mode.read()
    }

    pub fn loading(&self) -> Read<Option<LoadingDirection>> {
        self.0.loading.read()
    }

    pub fn metrics(&self) -> Read<ScrollMetrics> {
        self.0.metrics.read()
    }

    pub fn messages(&self) -> &LiveQuery<MessageView> {
        &self.0.messages
    }

    pub fn set_live_mode(&self) {
        tracing::info!("→ setLiveMode");
        self.0.mode.set(ScrollMode::Live);
        *self.0.last_continuation_key.borrow_mut() = None;
        self.0.loading.set(None);

        let limit = self.compute_limit();
        self.0.current_limit.set(limit);
        self.0.current_direction.set("DESC".to_string());

        let predicate = format!("room = '{}' AND deleted = false ORDER BY timestamp DESC LIMIT {}", self.0.room_id, limit);
        let _ = self.0.messages.update_selection(predicate.as_str());

        // Set as active room when entering live mode
        self.0.notification_manager.set_active_room(Some(self.0.room_id.clone()));
        // afterLayout() will handle scrolling on next render
    }

    pub fn jump_to_live(&self) {
        tracing::info!("jumpToLive");
        self.set_live_mode();
        self.scroll_to_bottom();
    }

    pub fn at_earliest(&self) -> bool {
        let result_count = self.0.messages.get().len();
        let current_limit = self.0.current_limit.peek();
        let current_direction = self.0.current_direction.peek();
        // DESC queries hit oldest when count < limit
        current_direction == "DESC" && result_count < current_limit
    }

    pub fn at_latest(&self) -> bool {
        let mode = self.0.mode.peek();
        let result_count = self.0.messages.get().len();
        let current_limit = self.0.current_limit.peek();
        let current_direction = self.0.current_direction.peek();
        // Live mode is always at latest, ASC queries hit newest when count < limit
        mode == ScrollMode::Live || (current_direction == "ASC" && result_count < current_limit)
    }

    pub fn should_auto_scroll(&self) -> bool {
        let mode = self.mode().get();
        let bottom_gap = self.metrics().get().bottom_gap;
        mode == ScrollMode::Live && bottom_gap < 50.0
    }

    pub fn items(&self) -> Vec<MessageView> {
        let raw = self.0.messages.get();
        // live and backward modes use DESC → reverse for display
        // forward mode uses ASC → no reverse
        if self.0.mode.peek() != ScrollMode::Forward { raw.into_iter().rev().collect() } else { raw }
    }

    pub fn bind_container(&self, container: Option<HtmlDivElement>) {
        let current = self.0.container.borrow().clone();
        // Check if we're binding the same container (compare as JsValue pointers)
        let same_container = match (&current, &container) {
            (Some(a), Some(b)) => {
                let a_val: &wasm_bindgen::JsValue = a.as_ref();
                let b_val: &wasm_bindgen::JsValue = b.as_ref();
                std::ptr::eq(a_val, b_val)
            }
            (None, None) => true,
            _ => false,
        };
        if same_container {
            return;
        }

        // Remove old event listeners
        if let Some(old_container) = current {
            if let Some(closure) = self.0.scroll_closure.borrow_mut().take() {
                let _ = old_container.remove_event_listener_with_callback("scroll", closure.as_ref().unchecked_ref());
            }
            if let Some(closure) = self.0.wheel_closure.borrow_mut().take() {
                let _ = old_container.remove_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref());
            }
            if let Some(closure) = self.0.touch_closure.borrow_mut().take() {
                let _ = old_container.remove_event_listener_with_callback("touchstart", closure.as_ref().unchecked_ref());
            }
        }

        *self.0.container.borrow_mut() = container.clone();

        if let Some(new_container) = container {
            *self.0.last_scroll_top.borrow_mut() = new_container.scroll_top() as f64;

            // Create closures for event handlers
            let self_scroll = self.clone();
            let scroll_closure = Closure::wrap(Box::new(move || {
                self_scroll.on_scroll();
            }) as Box<dyn FnMut()>);

            let self_wheel = self.clone();
            let wheel_closure = Closure::wrap(Box::new(move || {
                self_wheel.on_user_scroll();
            }) as Box<dyn FnMut()>);

            let self_touch = self.clone();
            let touch_closure = Closure::wrap(Box::new(move || {
                self_touch.on_user_scroll();
            }) as Box<dyn FnMut()>);

            // Add event listeners
            let _ = new_container.add_event_listener_with_callback("scroll", scroll_closure.as_ref().unchecked_ref());
            let _ = new_container.add_event_listener_with_callback("wheel", wheel_closure.as_ref().unchecked_ref());
            let _ = new_container.add_event_listener_with_callback("touchstart", touch_closure.as_ref().unchecked_ref());

            // Store closures so they don't get dropped
            *self.0.scroll_closure.borrow_mut() = Some(scroll_closure);
            *self.0.wheel_closure.borrow_mut() = Some(wheel_closure);
            *self.0.touch_closure.borrow_mut() = Some(touch_closure);
        }
    }

    pub fn after_layout(&self) {
        if !*self.0.initialized.borrow() {
            *self.0.initialized.borrow_mut() = true;
        }
        if self.should_auto_scroll() {
            self.scroll_to_bottom();
        }
    }

    pub fn destroy(&self) {
        tracing::info!("ChatScrollManager: destroy");
        // The ListenerGuard will be dropped when Inner is dropped
        self.bind_container(None); // This will clean up event listeners
    }

    fn compute_limit(&self) -> usize {
        let container = self.0.container.borrow();
        let Some(ref container) = *container else {
            return 100; // Default
        };

        let window = window().expect("no window");
        let computed_style = window.get_computed_style(container).ok().flatten().expect("failed to get computed style");

        let padding_top =
            computed_style.get_property_value("padding-top").ok().and_then(|s| s.trim_end_matches("px").parse::<f64>().ok()).unwrap_or(0.0);
        let padding_bottom = computed_style
            .get_property_value("padding-bottom")
            .ok()
            .and_then(|s| s.trim_end_matches("px").parse::<f64>().ok())
            .unwrap_or(0.0);

        let content_height = container.client_height() as f64 - padding_top - padding_bottom;
        let query_height_px = content_height * self.0.query_size;
        (query_height_px / self.0.min_row_px).ceil() as usize
    }

    fn get_thresholds(&self) -> (f64, f64) {
        let container = self.0.container.borrow();
        let Some(ref container) = *container else {
            return (150.0, 240.0);
        };

        let window_px = container.client_height() as f64;
        (self.0.min_buffer_size * window_px, self.0.continuation_step_back * window_px)
    }

    fn update_metrics(&self) {
        let container = self.0.container.borrow();
        let Some(ref container) = *container else {
            return;
        };

        let scroll_top = container.scroll_top() as f64;
        let scroll_height = container.scroll_height() as f64;
        let client_height = container.client_height() as f64;
        let (min_buffer, step_back) = self.get_thresholds();

        self.0.metrics.set(ScrollMetrics {
            top_gap: scroll_top,
            bottom_gap: scroll_height - scroll_top - client_height,
            min_buffer,
            step_back,
            result_count: self.0.messages.get().len(),
        });
    }

    fn get_continuation_anchor(&self, direction: LoadingDirection, message_list: &[MessageView]) -> Option<(HtmlElement, MessageView)> {
        let container = self.0.container.borrow();
        let container = container.as_ref()?;

        if message_list.is_empty() {
            return None;
        }

        let (_, step_back) = self.get_thresholds();
        let is_backward = direction == LoadingDirection::Backward;

        if is_backward {
            // Step back from bottom of newest message
            let last_msg = message_list.last()?;
            let start_el = container
                .query_selector(&format!("[data-msg-id=\"{}\"]", last_msg.id().to_base64()))
                .ok()??
                .dyn_into::<HtmlElement>()
                .ok()?;

            let target_pos = start_el.offset_top() as f64 + start_el.offset_height() as f64 - step_back;

            for msg in message_list.iter().rev() {
                let el = container
                    .query_selector(&format!("[data-msg-id=\"{}\"]", msg.id().to_base64()))
                    .ok()??
                    .dyn_into::<HtmlElement>()
                    .ok()?;

                if (el.offset_top() as f64 + el.offset_height() as f64) <= target_pos {
                    tracing::info!("getContinuationAnchor backward: timestamp={}", msg.timestamp().unwrap_or(0));
                    return Some((el, msg.clone()));
                }
            }

            // Fallback: return oldest message
            let msg = message_list.first()?;
            let el =
                container.query_selector(&format!("[data-msg-id=\"{}\"]", msg.id().to_base64())).ok()??.dyn_into::<HtmlElement>().ok()?;
            tracing::info!("getContinuationAnchor backward (fallback to oldest)");
            Some((el, msg.clone()))
        } else {
            // Step forward from top of oldest message
            let first_msg = message_list.first()?;
            let start_el = container
                .query_selector(&format!("[data-msg-id=\"{}\"]", first_msg.id().to_base64()))
                .ok()??
                .dyn_into::<HtmlElement>()
                .ok()?;

            let target_pos = start_el.offset_top() as f64 + step_back;

            for msg in message_list.iter() {
                let el = container
                    .query_selector(&format!("[data-msg-id=\"{}\"]", msg.id().to_base64()))
                    .ok()??
                    .dyn_into::<HtmlElement>()
                    .ok()?;

                if el.offset_top() as f64 >= target_pos {
                    tracing::info!("getContinuationAnchor forward: timestamp={}", msg.timestamp().unwrap_or(0));
                    return Some((el, msg.clone()));
                }
            }

            // Fallback: return newest message
            let msg = message_list.last()?;
            let el =
                container.query_selector(&format!("[data-msg-id=\"{}\"]", msg.id().to_base64())).ok()??.dyn_into::<HtmlElement>().ok()?;
            tracing::info!("getContinuationAnchor forward (fallback to newest)");
            Some((el, msg.clone()))
        }
    }

    pub fn load_more(&self, direction: LoadingDirection) {
        let is_backward = direction == LoadingDirection::Backward;
        let message_list = self.items();

        let Some((el, msg)) = self.get_continuation_anchor(direction.clone(), &message_list) else {
            return;
        };

        let timestamp = msg.timestamp().unwrap_or(0);
        let key = format!("{:?}-{}", direction, timestamp);
        if self.0.last_continuation_key.borrow().as_ref() == Some(&key) {
            return;
        }

        // Begin load
        self.0.loading.set(Some(direction.clone()));
        let mode = if is_backward { ScrollMode::Backward } else { ScrollMode::Forward };
        self.0.mode.set(mode);

        // Clear active room when leaving live mode
        if self.0.mode.peek() != ScrollMode::Live {
            self.0.notification_manager.set_active_room(None);
        }

        *self.0.last_continuation_key.borrow_mut() = Some(key);

        let limit = self.compute_limit();
        let y_before = offset_to_parent(&el).map(|(_, y)| y).unwrap_or(0.0);

        // Log timestamp range before load
        let earliest_before = message_list.first().and_then(|m| m.timestamp().ok());
        let latest_before = message_list.last().and_then(|m| m.timestamp().ok());

        let op = if is_backward { "<=" } else { ">=" };
        let order = if is_backward { "DESC" } else { "ASC" };

        let room_id = self.0.room_id.clone();
        let messages = self.0.messages.clone();
        let self_clone = self.clone();
        let el_clone = el.clone();

        spawn_local(async move {
            let predicate = format!(
                "room = '{}' AND deleted = false AND timestamp {} {} ORDER BY timestamp {} LIMIT {}",
                room_id, op, timestamp, order, limit
            );
            let _ = messages.update_selection(predicate.as_str());

            self_clone.0.current_limit.set(limit);
            self_clone.0.current_direction.set(order.to_string());

            // Log timestamp range after load
            let after_list = self_clone.items();
            let earliest_after = after_list.first().and_then(|m| m.timestamp().ok());
            let latest_after = after_list.last().and_then(|m| m.timestamp().ok());

            tracing::info!(
                "loadMore timestamps: direction={:?}, before=(earliest={:?}, latest={:?}, count={}), after=(earliest={:?}, latest={:?}, count={})",
                direction,
                earliest_before,
                latest_before,
                message_list.len(),
                earliest_after,
                latest_after,
                after_list.len()
            );

            // If we hit the newest boundary - switch to live
            if self_clone.at_latest() {
                self_clone.set_live_mode();
                return;
            }

            let y_after = offset_to_parent(&el_clone).map(|(_, y)| y).unwrap_or(0.0);
            let delta = y_after - y_before;
            tracing::info!("loadMore: {:?} delta={}", direction, delta);

            if let Some(ref container) = *self_clone.0.container.borrow() {
                self_clone.scroll_to(container.scroll_top() as f64 + delta);
            }
            self_clone.0.loading.set(None);
        });
    }

    fn on_user_scroll(&self) {
        *self.0.user_scrolling.borrow_mut() = true;
    }

    fn on_scroll(&self) {
        let container = self.0.container.borrow();
        let Some(ref container) = *container else {
            return;
        };

        let scroll_top = container.scroll_top() as f64;
        let scroll_delta = scroll_top - *self.0.last_scroll_top.borrow();
        *self.0.last_scroll_top.borrow_mut() = scroll_top;

        // Always update metrics (for debug display)
        self.update_metrics();

        // Only trigger loads on user-initiated scrolls
        if *self.0.user_scrolling.borrow() {
            *self.0.user_scrolling.borrow_mut() = false;

            let message_list = self.items();
            if message_list.is_empty() {
                return;
            }

            let (min_buffer, _) = self.get_thresholds();
            let scroll_height = container.scroll_height() as f64;
            let client_height = container.client_height() as f64;
            let bottom_gap = scroll_height - scroll_top - client_height;

            // Scrolled up - try to load older messages
            if scroll_delta < 0.0 && scroll_top < min_buffer && !self.at_earliest() && self.0.loading.peek().is_none() {
                self.load_more(LoadingDirection::Backward);
            }
            // Scrolled down - try to load newer messages
            else if scroll_delta > 0.0 && bottom_gap < min_buffer && !self.at_latest() && self.0.loading.peek().is_none() {
                self.load_more(LoadingDirection::Forward);
            }
        }
    }

    fn scroll_to(&self, scroll_top: f64) {
        let container = self.0.container.borrow();
        let Some(ref container) = *container else {
            return;
        };

        let current = container.scroll_top() as f64;
        if (scroll_top - current).abs() > 0.1 {
            container.set_scroll_top(scroll_top as i32);
            let self_clone = self.clone();
            let window = window().expect("no window");
            let closure = Closure::once(move || {
                self_clone.update_metrics();
            });
            let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
            closure.forget();
        }
    }

    fn scroll_to_bottom(&self) {
        let container = self.0.container.borrow();
        let Some(ref container) = *container else {
            return;
        };
        self.scroll_to(container.scroll_height() as f64);
    }
}

fn offset_to_parent(el: &HtmlElement) -> Option<(f64, f64)> {
    let a = el.get_bounding_client_rect();
    let parent = el.parent_element()?;
    let b = parent.get_bounding_client_rect();
    Some((a.left() - b.left(), a.top() - b.top()))
}
