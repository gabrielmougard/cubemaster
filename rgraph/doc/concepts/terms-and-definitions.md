# Terms & definitions

`rgraph` is built around three concepts: **nodes**, **edges**, and the
**viewport**. This page explains the vocabulary used throughout the
documentation.

## Nodes

A node is a single rectangle inside the flow. Nodes carry:

| Field           | Type                  | Description                                   |
|-----------------|-----------------------|-----------------------------------------------|
| `id`            | `String`              | Unique node identifier.                       |
| `position`      | `XYPosition`          | Top-left corner in flow-space.                |
| `data`          | `D` (generic)         | Caller-owned payload (e.g. label, color, …).  |
| `type_`         | `Option<String>`      | Selects the React component used to render.   |
| `width`/`height`| `Option<f64>`         | Optional explicit dimensions.                 |
| `hidden`        | `Option<bool>`        | Skip rendering when `true`.                   |
| `selected`      | `Option<bool>`        | Whether the user has clicked the node.        |
| `draggable`     | `Option<bool>`        | Override the global `nodes_draggable` flag.   |
| `connectable`   | `Option<bool>`        | Override the global `nodes_connectable`.      |
| `parent_id`     | `Option<String>`      | Nest this node inside another node.           |

The four built-in node variants (`default`, `input`, `output`, `group`)
all use the alias [`BuiltInNodeData`](../../src/types/nodes.rs) for
`D`. Custom nodes pick their own `D`.

```rust
use rgraph::prelude::*;
use rgraph_core::types::geometry::XYPosition;

let label_node = Node::<BuiltInNodeData>::with_data(
    "n1",
    XYPosition::new(40.0, 60.0),
    BuiltInNodeData::labelled("Hello"),
);
```

## Edges

An edge is a line connecting a **source** handle on one node to a
**target** handle on another node.

| Field             | Type            | Description                                          |
|-------------------|-----------------|------------------------------------------------------|
| `id`              | `String`        | Unique edge identifier.                              |
| `source`/`target` | `String`        | Source / target node id.                             |
| `source_handle`   | `Option<String>`| Selects a specific handle on the source node.        |
| `target_handle`   | `Option<String>`| Same for the target.                                 |
| `type_`           | `Option<String>`| `"default"`, `"straight"`, `"step"`, `"smoothstep"`, `"simplebezier"`, or a custom name. |
| `label`           | `Option<String>`| Optional inline label.                               |
| `animated`        | `bool`          | Draws an animated stroke when `true`.                |
| `hidden`/`selected`/`deletable`/`focusable` | `Option<bool>` | Same semantics as on `Node`. |

```rust
let e = Edge::<()>::minimal("e1-2", "n1", "n2");
```

## Handles

A **handle** is an anchor point on a node where edges can start
(`source`) or end (`target`). The built-in nodes draw their handles
automatically; custom nodes use the [`<Handle>`](../customization/handles.md)
component.

```rust
Handle::<MyData, ()> {
    id: Some("a".into()),
    r#type: HandleType::Source,
    position: Position::Right,
}
```

## Viewport

The **viewport** is the visible portion of the flow expressed as a
3-tuple `(x, y, zoom)`. Helpers:

- `transform.scale()` → `zoom`
- `transform.tx()` / `transform.ty()` → pan offsets in screen pixels

Read it from the store with [`use_viewport()`](../hooks/use-viewport.md)
and mutate it through the [`use_rgraph()`](../hooks/use-rgraph.md)
handle:

```rust
let h = use_rgraph::<BuiltInNodeData, ()>();
h.viewport.zoom_in(None);
h.viewport.set_center(0.0, 0.0, None);
h.viewport.fit_view(None);
```

## Connection

A **connection** is the transient edge drawn while the user drags from
one handle. When the cursor lands on a valid target handle the
connection is committed as a real edge via the `on_connect` callback.

## The store

Internally rgraph uses a single [`RGraphStore<N, E>`](../../src/store/mod.rs)
struct whose fields are [`Signal<…>`](https://dioxuslabs.com/learn/0.7/reference/signals).
Each component subscribes only to the signals it reads, which gives
React-Flow-like fine-grained reactivity without a global zustand store.

Access patterns:

- [`use_rgraph_store::<N, E>()`](../../src/context/mod.rs) — raw store handle
- [`use_rgraph()`](../hooks/use-rgraph.md) — the high-level imperative API
- [`use_store(selector, equality)`](../hooks/use-store.md) — subscribe to a derived value
