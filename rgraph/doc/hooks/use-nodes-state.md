# `use_nodes_state` / `use_edges_state`

Convenience hooks that create the `(signal, on_change_callback)` pair
needed for a **controlled** flow with one line each. They mirror the
TS hooks of the same name.

```rust
let (nodes, on_nodes_change) = use_nodes_state::<BuiltInNodeData>(vec![
    Node::<BuiltInNodeData>::minimal("a", 0.0, 0.0),
    Node::<BuiltInNodeData>::minimal("b", 100.0, 100.0),
]);
let (edges, on_edges_change) = use_edges_state::<()>(vec![]);

rsx! {
    RGraph::<BuiltInNodeData, ()> {
        id: "demo",
        nodes,
        edges,
        on_nodes_change,
        on_edges_change,
    }
}
```

## Returned tuple

```rust
pub fn use_nodes_state<N>(initial: Vec<Node<N>>) -> UseNodesState<N>;
pub fn use_edges_state<E>(initial: Vec<Edge<E>>) -> UseEdgesState<E>;
```

`UseNodesState<N>` is destructurable into:

- `nodes: Signal<Vec<Node<N>>>`
- `on_nodes_change: Callback<Vec<NodeChange<N>>>`

The callback internally calls `apply_node_changes` and writes the
result back to the signal, so the flow is controlled with zero
boilerplate.

## When to use it

Use these hooks when:

- You want a controlled flow but don't need custom logic in the
  change handler.
- You're prototyping and want to focus on layout / styling first.

When you outgrow them, switch to the manual pattern:

```rust
let mut nodes = use_signal(|| /* … */);

rsx! {
    RGraph {
        nodes,
        on_nodes_change: move |changes| {
            // do whatever you want
            let next = apply_node_changes(changes, nodes.peek().clone());
            nodes.set(next);
        }
    }
}
```

## See also

- [`apply_node_changes`](../../src/utils/changes.rs) and
  `apply_edge_changes` — the pure helpers driving the callback.
- [Uncontrolled flow](../advanced/uncontrolled-flow.md) — the
  alternative pattern that needs no signal at all.
