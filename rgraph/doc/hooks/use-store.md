# `use_store()`

Subscribe to a derived value computed from the rgraph store. Mirrors
the TS `useStore(selector, equalityFn?)` pattern.

```rust
let count = use_store::<BuiltInNodeData, (), usize>(
    |state| state.nodes.read().len(),
    None,
);
```

## Signature

```rust
pub fn use_store<N, E, T>(
    selector:   impl Fn(&RGraphStore<N, E>) -> T + 'static,
    equality:   Option<Box<dyn Fn(&T, &T) -> bool>>,
) -> T
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
    T: PartialEq + Clone + 'static;
```

- `selector` — runs every time the underlying store signals change.
  Read whatever signals you need inside it. The result is memoized
  by `equality` (or `PartialEq` by default).
- `equality` — optional override (e.g. shallow compare a `Vec<String>`).

## When to use it

Use `use_store` instead of the raw `use_rgraph_store()` when:

- You only want a re-render on a specific derived value (e.g. node
  count, viewport zoom, selected node ids).
- The derivation is more complex than reading a single signal.

For the most common cases there are dedicated hooks:

- [`use_nodes`](../../src/hooks/use_nodes.rs) → `Vec<Node<N>>`
- [`use_edges`](../../src/hooks/use_edges.rs) → `Vec<Edge<E>>`
- [`use_viewport`](./use-viewport.md) → `Viewport`
- [`use_internal_node`](../../src/hooks/use_internal_node.rs) → one node
- [`use_visible_node_ids`](../../src/hooks/use_visible_node_ids.rs)
- [`use_visible_edge_ids`](../../src/hooks/use_visible_edge_ids.rs)

These are pre-wired with the right selector + equality function, so
prefer them when applicable.

## `use_store_api()`

If you just want the raw `RGraphStore<N, E>` handle (e.g. to read
fields imperatively in a callback without subscribing), call:

```rust
let store = use_store_api::<BuiltInNodeData, ()>();
let zoom  = store.transform.peek().scale();
```

`peek()` skips subscription, unlike `read()` which subscribes the
calling render scope.

## Comparison with TS

| TS                              | Rust                                |
|---------------------------------|-------------------------------------|
| `useStore(selector, shallow)`   | `use_store(selector, Some(shallow))`|
| `useStoreApi()`                 | `use_store_api()`                   |
| `useReactFlow()`                | `use_rgraph()`                      |
