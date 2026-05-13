# rgraph-core

Rust port of [`@xyflow/system`](https://github.com/xyflow/xyflow/tree/main/packages/system),
the framework-agnostic core that powers React Flow and Svelte Flow.

This crate is the data + logic layer for a Dioxus-based node-graph UI.
Like its sibling `rgraph-*` crates (`drag`, `zoom`, `selection`,
`interpolate`, `transition`), it deliberately omits the
DOM-listener-attaching half of its JavaScript counterpart: callers
(Dioxus components) wire raw events via `rsx!{}` handlers and feed them
into the pure state machines exposed here.

## Status

**Complete.** All 8 phases of the port have been executed. The crate
covers the entire `@xyflow/system` surface, with **244 unit tests**
passing on both the default and `serde` feature builds and **zero
warnings** under `cargo build` / `cargo test`.

| Phase | Scope                                                          | Tests | Cumulative |
| ----- | -------------------------------------------------------------- | ----- | ---------- |
| 0     | Skeleton (39 stubs, CSS assets, workspace registration)        | —     | —          |
| 1     | Pure types (`constants`, `types/*`, `promise`)                 | 38    | 38         |
| 2     | Pure math utils (`general`, `edges/*`, `marker`, toolbars, …)  | +65   | 103        |
| 3     | Graph + store + DOM (`graph`, `store`, `dom`)                  | +30   | 133        |
| 4     | `XYPanZoom` over `rgraph-zoom`                                 | +41   | 174        |
| 5     | `XYDrag` over `rgraph-drag`                                    | +22   | 196        |
| 6     | `XYHandle` connection state machine                            | +20   | 216        |
| 7     | `XYResizer` over `rgraph-drag`                                 | +18   | 234        |
| 8     | `XYMiniMap` + `styles` re-exports                              | +10*  | **244**    |

\* phase 8 adds 10 minimap tests + 3 styles smoke tests.

Each `.rs` file's header comment lists the corresponding TS source
file (with line ranges) and which other `rgraph-*` crates it depends on.

## Architecture

| Module                     | Mirrors TS path                | Purpose                                                      |
| -------------------------- | ------------------------------ | ------------------------------------------------------------ |
| `constants`                | `constants.ts`                 | Error messages, defaults                                     |
| `types::geometry`          | `types/utils.ts`               | `XYPosition`, `Rect`, `Position`, `Transform`, …             |
| `types::viewport`          | `types/general.ts`             | `Viewport`, modes, padding parser inputs                     |
| `types::nodes`             | `types/nodes.ts`               | `Node<D>`, `InternalNode<D>`, lookups, `NodeExtent`          |
| `types::edges`             | `types/edges.ts`               | `Edge<D>`, markers, line types                               |
| `types::handles`           | `types/handles.ts`             | `Handle`, `HandleType`                                       |
| `types::connection`        | `types/general.ts`             | `Connection`, `ConnectionState<N>`                           |
| `types::changes`           | `types/changes.ts`             | `NodeChange<D>`, `EdgeChange<D>`                             |
| `types::panzoom`           | `types/panzoom.ts`             | `PanZoomParams`, `PanZoomInstance` trait                     |
| `utils::general`           | `utils/general.ts`             | `clamp`, point↔renderer, `get_viewport_for_bounds`           |
| `utils::graph`             | `utils/graph.ts`               | Bounds, ancestors, descendants, `fit_viewport`               |
| `utils::edges::*`          | `utils/edges/*.ts`             | Bezier / straight / smoothstep path generation               |
| `utils::store`             | `utils/store.ts`               | `adopt_user_nodes`, `update_node_internals`, `pan_by`        |
| `utils::dom`               | `utils/dom.ts`                 | DOM-measurement-free reimagining of the upstream helpers     |
| `utils::marker`            | `utils/marker.ts`              | SVG marker id helpers (`get_marker_id`, `create_marker_ids`) |
| `utils::*_toolbar`         | `utils/{node,edge}-toolbar.ts` | Toolbar transform math                                       |
| `utils::shallow_node_data` | `utils/shallow-node-data.ts`   | Equality helpers                                             |
| `utils::connections`       | `utils/connections.ts`         | `add_edge`, `reconnect_edge`, connection-lookup helpers      |
| `xypanzoom`                | `xypanzoom/`                   | `XYPanZoom<K>` — wraps `rgraph-zoom`                         |
| `xydrag`                   | `xydrag/`                      | `XYDrag<D>` — wraps `rgraph-drag`                            |
| `xyhandle`                 | `xyhandle/`                    | `XYHandle<D>` — connection state machine                     |
| `xyresizer`                | `xyresizer/`                   | `XYResizer<D>` — wraps `rgraph-drag` for resize              |
| `xyminimap`                | `xyminimap/`                   | `XYMiniMap` — minimap viewport interaction                   |
| `styles`                   | `styles/*.css`                 | `include_str!` constants                                     |
| `promise`                  | (n/a — replaces JS `Promise`)  | Tiny std-only `Promise<T>` / `Resolver<T>`                   |

## Porting principles

1. **No DOM listeners.** Callers feed pre-measured events into pure
   state machines via `handle_pointer_*` / `handle_wheel` methods.
2. **No DOM measurement.** Container bounds and node dimensions are
   measured by the Dioxus consumer (`MountedData::get_client_rect()`)
   and passed in via `set_container_bounds` / `InternalNodeUpdate`.
3. **Generic over user data.** `Node<D: Clone>`, `Edge<D: Clone>`.
4. **Mutate `&mut HashMap` directly** for store helpers — no Zustand-
   style wrapper layer.
5. **`Promise<bool>`** in place of JS `Promise<boolean>` for animated
   calls. Resolved promises for synchronous paths, `oneshot`-channel
   promises for transition-driven paths.
6. **Literal snake_case mapping** of TS names; struct names like
   `XYPanZoom` are preserved verbatim for cross-reference convenience.
7. **CSS** shipped both as files in `assets/` and as `&'static str`
   consts in [`styles`].
8. **`!Send` callbacks.** Closures stored in option structs are
   intentionally `!Send` (no `+ Send + Sync` bound) so they can capture
   `Rc<RefCell<…>>` from single-threaded Dioxus signal contexts.

## Dependencies

```text
rgraph-core
├── rgraph-dispatch
├── rgraph-drag
├── rgraph-interpolate
├── rgraph-selection
├── rgraph-transition
└── rgraph-zoom
```

## Usage

Add a workspace dependency to your Dioxus crate's `Cargo.toml`:

```toml
[dependencies]
rgraph-core = { path = "../rgraph-core" }
# optional: enables Serialize/Deserialize on most data types
# rgraph-core = { path = "../rgraph-core", features = ["serde"] }
```

Inject the bundled styles in your root component:

```rust,ignore
use dioxus::prelude::*;
use rgraph_core::styles::ALL_CSS;

fn App() -> Element {
    rsx! {
        document::Style { {ALL_CSS} }
        // …your flow components…
    }
}
```

Build a single-pane pan/zoom manager:

```rust,ignore
use rgraph_core::{
    types::geometry::Rect,
    types::panzoom::PanZoomParams,
    types::viewport::Viewport,
    xypanzoom::XYPanZoom,
};

let panzoom = XYPanZoom::<()>::new_single(PanZoomParams {
    min_zoom: 0.25,
    max_zoom: 4.0,
    viewport: Viewport::IDENTITY,
    translate_extent: rgraph_core::constants::INFINITE_EXTENT,
    dom_bbox: Rect::new(0.0, 0.0, 1024.0, 768.0),
    on_dragging_change: Box::new(|_| {}),
    on_pan_zoom_start: None,
    on_pan_zoom: None,
    on_pan_zoom_end: None,
});

// In your onwheel handler:
panzoom.handle_wheel(rgraph_zoom::WheelInput {
    delta_y: -100.0,
    delta_mode: 0,
    ctrl: false,
    x: 50.0,
    y: 50.0,
});
```

See the per-module documentation comments for `XYDrag`, `XYHandle`,
`XYResizer`, and `XYMiniMap` for similar setup snippets.

## Testing

```sh
# Default feature set
cargo test -p rgraph-core --lib

# With serde
cargo test -p rgraph-core --features serde --lib
```

Both invocations should report `244 passed; 0 failed`.

## License

MIT OR Apache-2.0 (matches the workspace license).
