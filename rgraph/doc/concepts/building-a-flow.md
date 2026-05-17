# Building a flow

This guide walks through creating your first interactive flow with
rgraph: two nodes, an edge between them, and basic pan/zoom/drag.

## 1. Project setup

In your Dioxus desktop app's `Cargo.toml`:

```toml
[dependencies]
dioxus      = { version = "0.7", features = ["desktop"] }
rgraph      = { path = "../rgraph" }
rgraph-core = { path = "../rgraph-core" }
```

## 2. The minimal app

```rust
use dioxus::prelude::*;
use rgraph::prelude::*;
use rgraph::styles::BASE_CSS;
use rgraph_core::types::geometry::XYPosition;

#[component]
fn App() -> Element {
    rsx! {
        style { "{BASE_CSS}" }
        Flow {}
    }
}

#[component]
fn Flow() -> Element {
    rsx! {
        div { style: "width: 100vw; height: 100vh;",
            RGraph::<BuiltInNodeData, ()> {
                id: "demo",
            }
        }
    }
}

fn main() {
    dioxus::desktop::launch(App);
}
```

Running this displays an empty pane with a transparent background. The
`id` prop is required — it's used as the ResizeObserver selector that
keeps the pane's bounding box in sync with the window.

## 3. Add nodes

Nodes are passed in via the `nodes` signal:

```rust
let nodes = use_signal(|| vec![
    Node::<BuiltInNodeData>::with_data(
        "1",
        XYPosition::new(100.0, 100.0),
        BuiltInNodeData::labelled("Node A"),
    )
    .with_type("input"),

    Node::<BuiltInNodeData>::with_data(
        "2",
        XYPosition::new(400.0, 100.0),
        BuiltInNodeData::labelled("Node B"),
    ),

    Node::<BuiltInNodeData>::with_data(
        "3",
        XYPosition::new(250.0, 280.0),
        BuiltInNodeData::labelled("Node C"),
    )
    .with_type("output"),
]);
```

The `with_type("input" | "output" | "group")` builder selects the
[built-in node variants](../customization/custom-nodes.md). Omitting
it falls back to `default`.

## 4. Add edges

```rust
let edges = use_signal(|| vec![
    Edge::<()>::minimal("e1-2", "1", "2"),
    Edge::<()>::minimal("e2-3", "2", "3").with_animated(true),
]);
```

## 5. Wire it up

```rust
rsx! {
    div { style: "width: 100vw; height: 100vh;",
        RGraph::<BuiltInNodeData, ()> {
            id: "demo",
            nodes,
            edges,
        }
    }
}
```

You now have:

- Two source-only "input" nodes and one target-only "output" node,
- An animated edge between the second and third node,
- Pan with click-and-drag, zoom with mouse wheel,
- Drag any node to reposition it.

## 6. Add UI helpers

The optional helper components live under `rgraph::additional_components`
and are re-exported from `rgraph::prelude`:

```rust
RGraph::<BuiltInNodeData, ()> {
    id: "demo",
    nodes,
    edges,
    Background::<BuiltInNodeData, ()> {
        variant: BackgroundVariant::Dots,
        gap: BackgroundGap::Uniform(20.0),
    }
    Controls::<BuiltInNodeData, ()> {}
    MiniMap::<BuiltInNodeData> {
        pannable: true,
    }
}
```

## 7. React to changes

Use the `on_nodes_change` / `on_edges_change` props to keep your own
state in sync:

```rust
let mut nodes = use_signal(|| vec![ /* … */ ]);
let mut edges = use_signal(|| vec![ /* … */ ]);

rsx! {
    RGraph::<BuiltInNodeData, ()> {
        id: "demo",
        nodes,
        edges,
        on_nodes_change: move |changes| {
            let next = apply_node_changes(changes, nodes.peek().clone());
            nodes.set(next);
        },
        on_edges_change: move |changes| {
            let next = apply_edge_changes(changes, edges.peek().clone());
            edges.set(next);
        },
        on_connect: move |conn| {
            let mut next = edges.peek().clone();
            next.push(Edge::<()>::from_connection(&conn));
            edges.set(next);
        },
    }
}
```

`apply_node_changes` / `apply_edge_changes` are pure helpers in
[`rgraph::utils::changes`](../../src/utils/changes.rs) that mirror the
React-Flow utilities of the same name.

## What's next?

- [Adding interactivity](./adding-interactivity.md) — selection,
  deletion, keyboard shortcuts.
- [Custom nodes](../customization/custom-nodes.md) — render your own
  Dioxus components inside nodes.
- [The viewport](./the-viewport.md) — programmatic pan/zoom, fit-view,
  coordinate conversion.
