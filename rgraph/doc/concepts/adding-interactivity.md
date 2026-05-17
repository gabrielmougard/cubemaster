# Adding interactivity

By default rgraph nodes are draggable, edges are selectable, and the
viewport pans/zooms with mouse input. This page covers the toggles and
event hooks you'll wire up for a typical interactive flow.

## Pan, zoom, drag

The defaults are all `true`. To disable them, pass the corresponding
boolean on `<RGraph>`:

```rust
RGraph::<BuiltInNodeData, ()> {
    id: "view-only",
    nodes_draggable:      false,
    nodes_connectable:    false,
    elements_selectable:  false,
    pan_on_drag:          PanOnDrag::Off,
    zoom_on_scroll:       false,
    zoom_on_pinch:        false,
}
```

The "interactivity lock" inside [`<Controls>`](../components/controls.md)
toggles `nodes_draggable | nodes_connectable | elements_selectable` as
a single group, mirroring upstream behavior.

## Selection

Click-to-select toggles the `selected` flag on a node or edge. Hold
the `multi_selection_key_code` (default `"Control"`, set to `"Meta"`
on macOS) to add to the selection instead of replacing it. Drawing a
marquee with `selection_key_code` (default `"Shift"`) selects multiple
elements.

To deselect everything, call:

```rust
let h = use_rgraph::<BuiltInNodeData, ()>();
h.unselect_nodes_and_edges(Default::default());
```

## Reacting to user actions

`<RGraph>` exposes a callback for every meaningful user action. All
callbacks take `EventHandler<T>` so any closure that implements
`FnMut(T) + 'static` works:

```rust
RGraph::<BuiltInNodeData, ()> {
    id: "demo",
    nodes,
    edges,

    on_node_click:   |args: NodeMouseHandlerArgs<_>| { /* … */ },
    on_node_drag:    |args: OnNodeDragArgs<_>|       { /* … */ },
    on_node_drag_stop: |args: OnNodeDragArgs<_>|     { /* … */ },
    on_edge_click:   |args: EdgeMouseHandlerArgs<_>| { /* … */ },
    on_pane_click:   |evt: MouseData|                { /* … */ },
    on_move:         |args: OnMoveCallbackArgs|      { /* … */ },
    on_selection_change: |params: OnSelectionChangeParams<_, _>| { /* … */ },
}
```

The full list lives in
[`rgraph::types::component_props`](../../src/types/component_props.rs).

## Connecting nodes

When the user finishes dragging from a source handle onto a target
handle, the `on_connect` callback fires with the proposed
`Connection`. Append it to your edges signal to commit it:

```rust
on_connect: move |conn: Connection| {
    let mut next = edges.peek().clone();
    next.push(Edge::<()>::from_connection(&conn));
    edges.set(next);
},
```

To validate connections before committing, supply `is_valid_connection`:

```rust
is_valid_connection: |conn: &Connection| -> bool {
    // reject self-loops
    conn.source != conn.target
},
```

## Deletion

The default delete key is `"Backspace"`. The flow listens for it on
the window (via [`use_global_key_handler`](../hooks/use-store.md)).
Override with:

```rust
RGraph::<BuiltInNodeData, ()> {
    delete_key_code: Some(KeyPressMatcher::Key("Delete".into())),
    on_delete: move |params| {
        // remove from your own state
    },
    on_before_delete: |params| -> bool {
        // return false to veto
        true
    },
}
```

## Keyboard navigation

- `Tab` cycles focus through nodes/edges.
- Arrow keys nudge the selected nodes (default 4 px, hold `Shift` for
  larger steps).
- `Enter` triggers `on_node_click` on the focused node.

These behaviors live in
[`hooks::use_global_key_handler`](../../src/hooks/use_global_key_handler.rs)
and can be customized through the `pan_activation_key_code`,
`selection_key_code`, `delete_key_code` etc. props on `<RGraph>`.

## Controlled vs uncontrolled

If you pass both `nodes` and `on_nodes_change`, the flow is
**controlled**: your signal is the source of truth, and you must apply
the incoming changes via `apply_node_changes`. If you omit
`on_nodes_change`, the flow is **uncontrolled**: rgraph updates its
internal copy automatically.

See [Uncontrolled flow](../advanced/uncontrolled-flow.md) for details.
