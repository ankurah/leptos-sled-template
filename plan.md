# Leptos Template Implementation Plan

## Current State

- ✅ Cloned react-sled-template (dev branch) into ankurah-leptos-sled-template
- ✅ Backend crates (model/, server/) already present and unchanged
- ✅ Leptos CSR scaffold running with hello-world via Trunk
- ✅ ReactiveGraphObserver bridge implemented in ankurah-signals
- ✅ RoomList component fully ported and working
- ✅ Room creation and selection working
- ⏳ react-app/ and wasm-bindings/ remain as migration checklist

## Phase 1: Minimal Signals Bridge ✅ COMPLETE

### Implementation Summary

Successfully implemented a working bridge between Ankurah signals and Leptos reactive_graph:

1. **ReactiveGraphObserver** (`ankurah/signals/src/reactive_graph.rs`)

   - Implements `ankurah_signals::Observer`
   - Maintains map of `BroadcastId -> Arc<BridgeSource>`
   - Forwards `CurrentObserver::track()` calls into reactive_graph

2. **BridgeSource** (internal to ReactiveGraphObserver)

   - Wraps an `ArcRwSignal<()>` as the Leptos-side trigger
   - Holds a `ListenerGuard` to subscribe to Ankurah broadcasts
   - Calls `notify()` on the trigger when Ankurah signal updates
   - Only tracks when `Owner::current()` exists (avoids warnings from non-reactive contexts)

3. **Key Design Decisions**
   - **Low-road approach**: Minimal changes, surrogate signals
   - **Owner check**: Only call `track()` when inside a Leptos reactive context
   - **Global singleton**: One `ReactiveGraphObserver` set at app initialization
   - **Thread-local context**: Using `lazy_static` + `OnceLock` for Node and WebsocketClient

### Learnings

1. **Leptos closure requirements are strict**

   - `view!` macro creates closures that must be `Fn`, not `FnOnce`
   - Must clone values before passing to components or nested closures
   - Helper functions that take `&T` and clone internally are more ergonomic

2. **Signal disposal and lifecycle**

   - Leptos signals can be disposed when components unmount
   - Use `try_get()` in event handlers that might fire after disposal
   - Async operations spawned from components need careful signal handling

3. **Reactive context warnings**

   - `reactive_graph` warns when signals are tracked outside reactive contexts
   - Ankurah's internal code (reactor, transaction commits) calls `track()` outside Leptos contexts
   - Solution: Check `Owner::current()` before calling `track()`

4. **Component organization**
   - Extract helper functions that return closures for Effects
   - Break complex components into smaller sub-components
   - Use Leptos prop shorthand when variable name matches prop name

## Phase 1: Minimal Signals Bridge (ORIGINAL PLAN)

### Goal

Enable Leptos components to observe Ankurah signals (LiveQuery, View fields) by bridging `CurrentObserver::track` into Leptos's reactive graph.

### Validated Design

#### Core Pattern

Looking at the React integration and test examples, the pattern is:

1. **Ankurah signals call `CurrentObserver::track(self)`** when accessed (e.g., `LiveQuery::get()`, `View::name()`)
2. **CurrentObserver maintains a thread-local stack** of `Observer` trait objects
3. **The active Observer's `observe(&dyn Signal)` method** is called, which:
   - For React, calls `signal.listen(listener)` and stores `ListenerGuard`s
   - For Leptos, we want to forward this tracking into reactive_graph so its `Observer` (a Leptos effect/memo/component) can track dependencies.

#### Leptos Integration Strategy (low road)

**Create a `ReactiveGraphObserver`** in `ankurah-signals` (behind a `reactive-graph` feature) that implements `ankurah_signals::Observer`:

- It holds a map `BroadcastId -> Arc<BridgeSource>`.
- `BridgeSource` is an Arc-backed surrogate that will eventually implement `reactive_graph::graph::Source`, `ToAnySource`, and `ReactiveNode` for a single Ankurah broadcast.
- In `ReactiveGraphObserver::observe(&dyn Signal)`, we:
  - Look up or create the `BridgeSource` for that signal's `BroadcastId`.
  - Call `bridge_source.track()`, relying on reactive_graph's `Track` impl to wire `AnySubscriber` ↔ `AnySource`.
- In the _first iteration_, `ReactiveGraphObserver` and `BridgeSource` will be skeletons with detailed docs explaining the “low road” approach; the full `Source`/`Broadcast` wiring comes later.

This is the **low road**: we introduce a minimal observer that forwards tracking into reactive_graph via a surrogate Source, without changing Ankurah's core traits or `Broadcast` semantics. A possible **high road** later:

- Extend `Broadcast` with explicit subscribe/unsubscribe APIs tuned for reactive_graph.
- Implement `Source`/`ToAnySource` directly on a wrapper over `Broadcast`, reducing indirection.
- Potentially work with reactive_graph maintainers to support foreign signal systems more ergonomically.

**Usage in Leptos app**:

```rust
// In leptos-app/src/lib.rs, at app initialization
use ankurah_signals::CurrentObserver;

#[component]
pub fn App() -> impl IntoView {
    // Create and push the ReactiveGraphObserver once at app start
    let observer = ReactiveGraphObserver::new();
    CurrentObserver::set(observer);

    view! {
        <RoomList />
    }
}

#[component]
fn RoomList() -> impl IntoView {
    // Get the LiveQuery (from context or prop)
    let rooms: RoomLiveQuery = /* ... */;

    // Calling .get() will:
    // 1. Call CurrentObserver::track(&rooms)
    // 2. ReactiveGraphObserver::observe() gets called
    // 3. Creates a Leptos RwSignal trigger
    // 4. Subscribes to the Ankurah signal via Signal::listen
    // 5. Reads the trigger, registering it with this Leptos component
    // 6. Returns the Vec<RoomView>
    let items = move || rooms.get();

    view! {
        <For
            each=items
            key=|room| room.id().to_base64()
            children=|room| view! { <div>{room.name()}</div> }
        />
    }
}
```

### Key Insights

1. **No need to implement `reactive_graph::traits::Read`** for Ankurah signals—we're not consuming Leptos signals, we're making Ankurah signals observable by Leptos.

2. **The bridge is a single `ReactiveGraphObserver`** pushed onto `CurrentObserver`'s stack at app initialization.

3. **Leptos tracking happens automatically** when we call `.read()` on the hidden `RwSignal` trigger inside `observe()`.

4. **Lifecycle is managed by `ListenerGuard`**—when the observer is dropped, all subscriptions clean up automatically.

5. **Works with all Ankurah signals** that implement `Signal` trait: `LiveQuery`, `View` fields, `Mut<T>`, `Read<T>`, etc.

### Open Questions

1. **Observer lifecycle**: Should we create one global `ReactiveGraphObserver` or one per component?

   - **Hypothesis**: One global is simpler and matches the React pattern (one `ReactObserver` per component, but they all work the same way)
   - **Risk**: If we never clean up subscriptions, memory could grow unbounded
   - **Mitigation**: Use weak references or periodic cleanup based on `RwSignal` drop

2. **Trigger storage**: Should we use `RwSignal<()>` or is there a lighter primitive?

   - `RwSignal` is the standard Leptos primitive for triggering reactivity
   - Could explore `Trigger` or `Memo` but `RwSignal` is well-documented

3. **Thread safety**: `ReactiveGraphObserver` uses `RefCell` (single-threaded). Is this OK for WASM?

   - Yes—WASM is single-threaded, and Leptos CSR runs entirely in the browser
   - No multi-threading concerns for CSR mode

4. **Cleanup strategy**: When do we remove entries from the subscriptions map?
   - Option A: Never (rely on the fact that `BroadcastId` reuse is rare)
   - Option B: Weak references to detect when Leptos signals are dropped
   - Option C: Periodic sweep (complex, probably unnecessary)
   - **Recommendation**: Start with Option A, monitor for leaks

### Implementation Steps

1. Add `leptos` feature to `ankurah-signals/Cargo.toml`:

   ```toml
   [features]
   leptos = ["dep:leptos", "dep:reactive_graph"]

   [dependencies]
   leptos = { version = "0.7", optional = true }
   reactive_graph = { version = "0.2", optional = true }
   ```

2. Create `ankurah-signals/src/leptos.rs`:

   - Implement `ReactiveGraphObserver`
   - Export helper to create and register it

3. Add test in `ankurah-signals/tests/leptos_integration.rs`:

   ```rust
   #[test]
   fn test_leptos_observes_ankurah_signal() {
       let runtime = create_runtime();
       let observer = ReactiveGraphObserver::new();
       CurrentObserver::set(observer);

       let signal = Mut::new(42);
       let effect_ran = Arc::new(AtomicBool::new(false));

       create_effect({
           let signal = signal.clone();
           let flag = effect_ran.clone();
           move |_| {
               let value = signal.get(); // This should track via CurrentObserver
               flag.store(true, Ordering::Relaxed);
           }
       });

       assert!(effect_ran.load(Ordering::Relaxed));

       effect_ran.store(false, Ordering::Relaxed);
       signal.set(43);

       // Effect should have re-run
       assert!(effect_ran.load(Ordering::Relaxed));
   }
   ```

4. Wire into `leptos-app`:
   - Add `ankurah-signals = { version = "...", features = ["leptos"] }` to Cargo.toml
   - Initialize observer in `App` component
   - Test with simple `Mut<i32>` before moving to `LiveQuery`

## Phase 2: RoomList Component (Post-Implementation)

### Prerequisites

- ✅ `ReactiveGraphObserver` implemented and tested
- ✅ Simple test case verified (Leptos component observing `Mut<i32>`)

### Steps

1. Add ankurah dependencies to leptos-app/Cargo.toml:

   - ankurah (with leptos feature once available)
   - ankurah-websocket-client-wasm
   - ankurah-storage-indexeddb-wasm
   - ankurah-template-model (with wasm feature)
   - ankurah-signals (with leptos feature)

2. Port wasm-bindings initialization logic into leptos-app/src/lib.rs:

   - Create IndexedDB storage
   - Initialize Node with PermissiveAgent
   - Connect WebSocket client
   - Expose context via Leptos context API

3. Implement minimal RoomList component:

   - Query rooms using `Room::query(ctx(), "deleted = false")`
   - Use `move || rooms.get()` in `<For>` to observe the LiveQuery
   - Display room names
   - Verify reactivity: creating rooms updates the UI

4. Copy relevant CSS from react-app/src/components/RoomList.css

5. Pause for design review before continuing

## Phase 3: Remaining Components (Post-Review)

### Migration Strategy

- Use react-app/ and wasm-bindings/ as checklist
- Port components one-by-one:
  - RoomList ✓ (Phase 2)
  - MessageList
  - MessageInput
  - UserProfile
  - etc.
- Delete React source files as we complete each port
- Move CSS files into leptos-app/styles/ and delete originals

### ChatScrollManager

- Translate TypeScript logic to Rust
- Use web_sys for DOM queries instead of querySelector
- Leverage Leptos's NodeRef for element references
- Adapt scroll event handlers to Leptos's event system
- Consider making this a reusable utility that both templates can share (via wasm-bindgen export back to React)

## Phase 4: Testing & Documentation

### Tests

- Backend tests already exist (model/, server/)
- Add focused tests for ankurah-signals Leptos integration
- Manual testing of full app functionality

### Documentation

- Update README with Leptos-specific instructions
- Document the signals bridge design
- Add examples of using Ankurah signals in Leptos components

## Critical Path Items

### Blocking Phase 2

1. ✅ Validate `ReactiveGraphObserver` design
2. Implement in ankurah-signals behind `leptos` feature
3. Write focused test case
4. Verify with simple Leptos component before building RoomList

### Risk Areas

- **Subscription cleanup**: Need to monitor for memory leaks if subscriptions accumulate
- **Leptos version compatibility**: Currently targeting 0.7/0.8; API may change
- **Performance**: Verify that the RwSignal indirection doesn't cause excessive re-renders
- **Debugging**: May need additional logging to trace signal updates through the bridge

## Next Steps

1. Implement `ReactiveGraphObserver` in ankurah-signals
2. Add focused test case
3. Wire into leptos-app and verify with simple counter example
4. Proceed to RoomList if successful
