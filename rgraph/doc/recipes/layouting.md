# Auto-layout

rgraph deliberately ships with no layout algorithm — flows often have
unique semantics (timelines, mind maps, dataflow graphs) and a
one-size-fits-all auto-layout makes those harder to express. Instead,
plug in any graph-layout library and run it on a snapshot of the
nodes/edges.

## The pattern

1. Read the current nodes/edges (snapshot).
2. Compute new `XYPosition`s with your layout library of choice.
3. Write the positions back via `apply_node_changes` or
   `update_node`.

## Example: simple grid layout

```rust
fn grid_layout(
    nodes: &[Node<BuiltInNodeData>],
    cols:  usize,
    cell:  f64,
) -> Vec<NodeChange<BuiltInNodeData>> {
    nodes.iter().enumerate().map(|(i, n)| {
        let row = i / cols;
        let col = i % cols;
        NodeChange::Position {
            id: n.id.clone(),
            position: Some(XYPosition::new(col as f64 * cell, row as f64 * cell)),
            position_absolute: None,
            dragging: None,
        }
    }).collect()
}
```

Apply it:

```rust
let h = use_rgraph::<BuiltInNodeData, ()>();
let changes = grid_layout(&h.get_nodes(), 4, 180.0);
let next = apply_node_changes(changes, h.get_nodes());
h.set_nodes(next);
h.viewport.fit_view(None);
```

## Plugging in a real layout library

Any Rust crate that takes a node list + edge list and returns
positions works. Popular choices:

- **`petgraph` + custom force-directed layout** — most flexible.
- **`layout-rs`** — Sugiyama hierarchical layout.
- **WASM bridges** for `dagre.js` / `elkjs` if you want to reuse the
  same algorithm the React Flow ecosystem uses (run it inside the
  Dioxus desktop webview via `eval`).

### Example with dagre.js (via `dom::eval`)

```rust
use rgraph::dom::eval::eval_with_result;
use serde_json::json;

let payload = json!({
    "nodes": h.get_nodes().iter().map(|n| json!({
        "id": n.id,
        "width": n.width.unwrap_or(150.0),
        "height": n.height.unwrap_or(40.0),
    })).collect::<Vec<_>>(),
    "edges": h.get_edges().iter().map(|e| json!({
        "source": e.source, "target": e.target,
    })).collect::<Vec<_>>(),
});

let positions: serde_json::Value = eval_with_result(&format!(
    r#"
    const g = new dagre.graphlib.Graph();
    g.setGraph({{ rankdir: 'LR' }});
    g.setDefaultEdgeLabel(() => ({{}}));
    const payload = {};
    payload.nodes.forEach(n => g.setNode(n.id, {{ width: n.width, height: n.height }}));
    payload.edges.forEach(e => g.setEdge(e.source, e.target));
    dagre.layout(g);
    return Object.fromEntries(g.nodes().map(id => [id, {{ x: g.node(id).x, y: g.node(id).y }}]));
    "#,
    payload
));
// `positions` is a `{ id: { x, y } }` JSON object.
```

You'd then translate `positions` to `NodeChange::Position`s and feed
them into `apply_node_changes`.

## When to run layout

| Trigger                              | Where                              |
|--------------------------------------|------------------------------------|
| On mount                             | `use_effect` once.                 |
| When a node is added                 | After `add_node` in your handler.  |
| When the user clicks "Auto-layout"   | Custom `<ControlButton>` callback. |
| When edges change                    | Inside `on_edges_change`.          |

## Caveats

- Avoid running layout on every render — wrap it in `use_effect` or
  a button click.
- Layout libraries assume nodes have **measured** dimensions. The
  first render of a new node has `measured: None` until the
  ResizeObserver fires; consider running layout from
  `use_nodes_initialized` to wait for measurements.

```rust
let initialized = use_nodes_initialized(UseNodesInitializedOptions::default());
use_effect(move || {
    if initialized.read().clone() {
        // run layout
    }
});
```

## Sub-flows / nested layouts

Each "group" node (`type_ = "group"`, `parent_id` referenced by
children) is its own sub-coordinate system. Run layout per group by
filtering nodes on `parent_id`. The
[`evaluate_absolute_position`](../../../rgraph-core/src/utils/general.rs)
helper converts back to absolute coords when you need to compare
across groups.
