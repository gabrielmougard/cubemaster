# Uncontrolled flow

By default rgraph runs in **controlled** mode: you pass `nodes` /
`edges` signals and your `on_nodes_change` / `on_edges_change`
callbacks fold the changes back into them. Sometimes you don't need
external observability of the data — you just want a graph the user
can edit. That's the **uncontrolled** mode.

## Switching to uncontrolled

Use `default_nodes` / `default_edges` instead of `nodes` / `edges`:

```rust
RGraph::<BuiltInNodeData, ()> {
    id: "demo",
    default_nodes: vec![
        Node::<BuiltInNodeData>::with_data("a", XYPosition::new(0.0, 0.0), BuiltInNodeData::labelled("A")),
        Node::<BuiltInNodeData>::with_data("b", XYPosition::new(200.0, 100.0), BuiltInNodeData::labelled("B")),
    ],
    default_edges: vec![
        Edge::<()>::minimal("a-b", "a", "b"),
    ],
}
```

In this mode the flow owns the data. All user gestures (drag, select,
delete, connect) update the internal `nodes` / `edges` signals on
[`RGraphStore`](../../src/store/mod.rs) directly. You don't need a
`use_signal` for the lists.

## Imperative read / write

To read the current state, use [`use_rgraph()`](../hooks/use-rgraph.md):

```rust
let h = use_rgraph::<BuiltInNodeData, ()>();
let snapshot = h.get_nodes();
```

To mutate, use the same handle:

```rust
h.add_node(Node::<BuiltInNodeData>::minimal("c", 100.0, 200.0));
h.update_node("a", NodePartial {
    position: Some(XYPosition::new(50.0, 50.0)),
    ..Default::default()
});
h.delete_elements(DeleteElementsOptions {
    nodes: Some(vec![NodeRef::Id("b".into())]),
    edges: None,
});
```

## When uncontrolled is the right choice

✅ Quick prototypes, demos, examples.

✅ Self-contained "scratchpad" flows where the lifetime of the data
matches the lifetime of the component.

✅ Read-only displays where the user can pan/zoom but not edit.

❌ Persisting the graph between sessions — you'd have to read the
snapshot via `use_rgraph()` on every save, which is awkward.

❌ Derived UI (sidebar lists, inspectors) — observability is harder
because there's no signal to subscribe from outside the flow.

❌ Validation / interception of user changes.

For anything beyond a quick demo, prefer the controlled pattern.

## Subscriptions

Even in uncontrolled mode, every component below `<RGraph>` can
subscribe to the store via `use_rgraph_store()`, `use_nodes()`,
`use_edges()`, etc. The data lives at the store layer regardless of
who owns the source-of-truth signal.

## Mixed mode

There is **no** intermediate mode — passing both `nodes` and
`default_nodes` is a programming error and the controlled signal
wins. The same applies for edges.
