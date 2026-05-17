# `use_rgraph()`

The high-level imperative API for the flow. Returns an `RGraphHandle`
that bundles the [`RGraphStore`](../concepts/terms-and-definitions.md#the-store)
and a [`ViewportHelper`](./use-viewport.md) for convenient access.

```rust
let h = use_rgraph::<BuiltInNodeData, ()>();

h.add_node(Node::<BuiltInNodeData>::minimal("a", 0.0, 0.0));
h.set_nodes(vec![ /* … */ ]);
h.delete_elements(DeleteElementsOptions {
    nodes: Some(vec![NodeRef::Id("a".into())]),
    edges: None,
});

h.viewport.fit_view(None);
h.viewport.zoom_to(1.5, None);
```

## Construction

```rust
pub fn use_rgraph<N, E>() -> RGraphHandle<N, E>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
```

`use_rgraph` must be called **inside** a component that's nested below
`<RGraph>` (or `<RGraphProvider>`). It panics otherwise.

## Node methods

| Method                                | Description                                    |
|---------------------------------------|------------------------------------------------|
| `add_node(node: Node<N>)`             | Append a single node.                          |
| `add_nodes(nodes: Vec<Node<N>>)`      | Bulk append.                                   |
| `set_nodes(nodes: Vec<Node<N>>)`      | Replace the whole list.                        |
| `get_nodes() -> Vec<Node<N>>`         | Snapshot copy.                                 |
| `get_node(id) -> Option<Node<N>>`     | Lookup by id.                                  |
| `update_node(id, partial: NodePartial<N>)` | Patch a single field.                     |
| `update_node_data(id, data)`          | Replace just the `data` field.                 |

## Edge methods

The edge variants mirror the node variants:

| Method                                    |
|-------------------------------------------|
| `add_edge`, `add_edges`, `set_edges`, `get_edges`, `get_edge`, `update_edge` |

## Selection / deletion

```rust
h.delete_elements(DeleteElementsOptions {
    nodes: Some(vec![NodeRef::Id("n1".into())]),
    edges: Some(vec![EdgeRef::Id("e1-2".into())]),
});

h.unselect_nodes_and_edges(UnselectNodesAndEdgesParams::default());
```

## Viewport (delegated)

`h.viewport` is the same handle returned by
[`use_viewport_helper()`](./use-viewport.md). It exposes:

- `zoom_in / zoom_out / zoom_to / get_zoom`
- `set_viewport / get_viewport`
- `set_center / fit_bounds / fit_view`
- `screen_to_flow_position / flow_to_screen_position`

## Lifecycle

`RGraphHandle` is `Copy` (every field is a `Signal`), so you can pass
it around freely. Cloning is free.

```rust
let h = use_rgraph::<BuiltInNodeData, ()>();
let h2 = h; // cheap
use_effect(move || { let _ = h2.get_nodes(); });
```

## Comparison with TS

| TS                              | Rust                                |
|---------------------------------|-------------------------------------|
| `useReactFlow()`                | `use_rgraph()`                      |
| `getNodes()`                    | `h.get_nodes()`                     |
| `setNodes(fn)`                  | `h.set_nodes(new_vec)` (functional form not yet ported) |
| `updateNode(id, partial)`       | `h.update_node(id, NodePartial { … })` |
| `deleteElements({...})`         | `h.delete_elements(DeleteElementsOptions { … })` |
| `fitView()`                     | `h.viewport.fit_view(None)`         |
| `screenToFlowPosition(...)`     | `h.viewport.screen_to_flow_position(...)` |
