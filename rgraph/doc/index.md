# rgraph — Node-based graphs for Dioxus

`rgraph` is a Rust/Dioxus port of [React Flow](https://reactflow.dev/)
(the `@xyflow/react` package), built on top of the framework-agnostic
[`rgraph-core`](../../rgraph-core/) port of `@xyflow/system`.

It provides everything you need to build node-based UIs in a Dioxus
desktop application: an `RGraph` component, draggable nodes,
connectable handles, customizable edges, plus optional `Background`,
`Controls`, `MiniMap`, `NodeToolbar`, `EdgeToolbar` and `NodeResizer`
components.

## Status

| Phase | Scope                                         | Status      |
|-------|-----------------------------------------------|-------------|
| 0     | Crate skeleton, module stubs                  | ✅ done     |
| 1     | Pure types, change utilities                  | ✅ done     |
| 2     | Reactive store + provider                     | ✅ done     |
| 3     | Hooks (`use_rgraph`, `use_viewport_helper`, …)| ✅ done     |
| 4     | Pane / ZoomPane / Viewport rendering          | ✅ done     |
| 5     | Node renderer + built-in nodes + drag/select  | ✅ done     |
| 6     | Edge renderer, handles, connections           | ✅ done     |
| 7     | Top-level `<RGraph>` component                | ✅ done     |
| 8     | `Background`, `Controls`, `MiniMap`, etc.     | ✅ done     |

Tests: 107/107 passing. Clippy: 0 warnings.

## Documentation map

- **Concepts** — start here if you're new
  - [Terms & definitions](./concepts/terms-and-definitions.md)
  - [Building a flow](./concepts/building-a-flow.md)
  - [Adding interactivity](./concepts/adding-interactivity.md)
  - [The viewport](./concepts/the-viewport.md)
  - [Built-in components](./concepts/built-in-components.md)
- **Customization**
  - [Custom nodes](./customization/custom-nodes.md)
  - [Custom edges](./customization/custom-edges.md)
  - [Handles](./customization/handles.md)
  - [Edge labels](./customization/edge-labels.md)
  - [Theming](./customization/theming.md)
- **Component reference**
  - [`<RGraph>`](./components/rgraph.md)
  - [`<Background>`](./components/background.md)
  - [`<Controls>`](./components/controls.md)
  - [`<MiniMap>`](./components/minimap.md)
  - [`<NodeToolbar>`](./components/node-toolbar.md)
  - [`<EdgeToolbar>`](./components/edge-toolbar.md)
  - [`<NodeResizer>`](./components/node-resizer.md)
- **Hooks**
  - [`use_rgraph()`](./hooks/use-rgraph.md)
  - [`use_nodes_state` / `use_edges_state`](./hooks/use-nodes-state.md)
  - [`use_viewport()`](./hooks/use-viewport.md)
  - [`use_store()`](./hooks/use-store.md)
- **Advanced**
  - [State management](./advanced/state-management.md)
  - [Uncontrolled flow](./advanced/uncontrolled-flow.md)
  - [Performance](./advanced/performance.md)
- **Recipes**
  - [Drag-and-drop from a sidebar](./recipes/dragndrop.md)
  - [Auto-layout](./recipes/layouting.md)

## Quickstart

### 1. Add the crate

In your app's `Cargo.toml`:

```toml
[dependencies]
dioxus     = { version = "0.7", features = ["desktop"] }
rgraph     = { path = "../rgraph" } # or a published version
```

### 2. Render an `<RGraph>`

```rust
use dioxus::prelude::*;
use rgraph::prelude::*;
use rgraph_core::types::geometry::XYPosition;

#[component]
fn App() -> Element {
    let nodes = use_signal(|| vec![
        Node::<BuiltInNodeData>::with_data(
            "1",
            XYPosition::new(100.0, 100.0),
            BuiltInNodeData::labelled("Hello"),
        ),
        Node::<BuiltInNodeData>::with_data(
            "2",
            XYPosition::new(300.0, 200.0),
            BuiltInNodeData::labelled("World"),
        ),
    ]);
    let edges = use_signal(|| vec![
        Edge::<()>::minimal("e1-2", "1", "2"),
    ]);

    rsx! {
        div { style: "width: 100vw; height: 100vh;",
            RGraph::<BuiltInNodeData, ()> {
                id: "demo",
                nodes,
                edges,
                Background::<BuiltInNodeData, ()> {}
                Controls::<BuiltInNodeData, ()> {}
                MiniMap::<BuiltInNodeData> {}
            }
        }
    }
}

fn main() {
    dioxus::desktop::launch(App);
}
```

### 3. Include the stylesheet

`rgraph` re-uses the upstream xyflow CSS verbatim. Mount it via
`document::Stylesheet` or include it in your app's bundled assets:

```rust
use dioxus::prelude::*;
use rgraph::styles::BASE_CSS;

#[component]
fn AppShell() -> Element {
    rsx! {
        style { "{BASE_CSS}" }
        App {}
    }
}
```

## What's different from React Flow

The Rust port is intentionally close to the TS source, but a few
differences are unavoidable:

- **Generic types everywhere.** Where React Flow uses
  `Node<DataType>`, rgraph uses `Node<D>` and propagates the type
  parameter through every component. The shorthand alias
  `BuiltInNodeData` covers the four built-in node variants (default,
  input, output, group).
- **Signals instead of zustand.** State is stored as
  `Signal<RGraphStore>` and exposed through `use_rgraph_store()` /
  `use_rgraph()`. There is no global "store API".
- **Pointer events.** All pointer interactions go through Dioxus
  desktop's webview pointer events. Per-element capture is achieved
  with `dom::eval` calls (`set_pointer_capture` / `release_pointer_capture`).
- **No portals.** Dioxus 0.7 does not implement `createPortal`, so
  `<EdgeLabelRenderer>`, `<NodeToolbarPortal>` and `<ViewportPortal>`
  render inline. The visual outcome is the same.

## Where to go next

If you have built React Flow apps before, jump straight to the
[component reference](./components/rgraph.md) — the prop surface
matches the TS API closely.

If you're new to node-based UIs, follow the
[Building a flow](./concepts/building-a-flow.md) tutorial.
