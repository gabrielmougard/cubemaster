# Performance

rgraph is designed to handle graphs with thousands of nodes/edges
without dropping frames, but a few patterns make a noticeable
difference. This page collects them.

## 1. Subscribe narrowly

Every `signal.read()` inside a render scope subscribes that scope.
The fewer signals you read, the less the component re-renders.

```rust
// ❌ subscribes to *all* node mutations
let nodes = use_rgraph_store::<BuiltInNodeData, ()>().nodes.read();
let count = nodes.len();

// ✅ subscribes only when the count itself changes
let count = use_store::<BuiltInNodeData, (), usize>(
    |state| state.nodes.read().len(),
    None,
);
```

Use [`use_store(selector, equality)`](../hooks/use-store.md) for
derived values.

## 2. Only render visible elements

If your flow has thousands of off-screen nodes, set:

```rust
RGraph::<MyData, ()> {
    only_render_visible_elements: true,
}
```

This switches the renderer to a viewport-clipping mode that skips
nodes whose bounding box doesn't intersect the visible pane. The
trade-off is one extra hit-test per node per pan/zoom — fine for
≥1k nodes, negligible for ≥10k.

## 3. Avoid full-list rebuilds

`apply_node_changes` runs in O(n + k) where n = nodes and k = changes.
If you find yourself rebuilding the entire vec on every change, you
might be using `set_nodes(vec![…])` when you should be using
`update_node(id, partial)`.

```rust
// ❌ replaces the whole vec — O(n)
let mut next = h.get_nodes();
next.iter_mut().find(|n| n.id == "a").unwrap().position.x += 10.0;
h.set_nodes(next);

// ✅ surgical
h.update_node("a", NodePartial {
    position: Some(XYPosition::new(new_x, new_y)),
    ..Default::default()
});
```

## 4. Memoize node components

Built-in nodes are memoized by `props.id + props.selected + props.data`
via `PartialEq`. Custom nodes get the same benefit if you derive
`PartialEq` for `D` and use `#[component]` — the macro emits a memo
wrapper automatically.

If your `D` contains a `Vec` or `HashMap` that's expensive to compare,
consider wrapping it in `Rc<…>` so `PartialEq` is pointer-comparison.

## 5. Throttle expensive callbacks

Drag callbacks (`on_node_drag`) fire on every pointer move. If you
want to persist position only on drag end, use `on_node_drag_stop`
instead:

```rust
RGraph::<BuiltInNodeData, ()> {
    on_node_drag_stop: move |args| {
        persist(&args.node);
    },
}
```

## 6. Defer fit-view

`fit_view: true` runs on every render. After the initial render you
usually want it only on demand:

```rust
RGraph::<BuiltInNodeData, ()> {
    fit_view: initial_render.read().clone(),
}
```

…or just trigger it once via `use_effect`:

```rust
let h = use_rgraph::<BuiltInNodeData, ()>();
use_effect(move || { h.viewport.fit_view(None); });
```

## 7. Bulk operations

For large bulk updates (e.g. loading a saved graph), batch the
mutations under a single render frame by using `set_nodes` /
`set_edges` once instead of many `add_node` calls.

The store has a per-frame batcher
([`BatchProvider`](../../src/components/batch_provider.rs)) that
coalesces signal writes; explicit batching helps when you also need
to defer the dependent render.

## 8. Profiling tips

- `tracing` is wired through every store action. Enable a
  `tracing_subscriber` and filter `rgraph` to inspect the change
  pipeline.
- `cargo flamegraph` works on rgraph builds with `--release`. Look
  for hot calls inside `apply_node_changes` and the layout passes.
- The Dioxus desktop DevTools (Inspect Element → Performance) lets
  you see paint timings and animation frames — useful for chasing
  CSS-induced jank.
