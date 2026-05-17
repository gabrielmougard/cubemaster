# State management

rgraph supports three state-management patterns. Pick the one that
matches your data flow.

## 1. Controlled with explicit signal

You own the source of truth, rgraph mutates it through callbacks:

```rust
let mut nodes = use_signal(|| /* … */);
let mut edges = use_signal(|| /* … */);

RGraph::<BuiltInNodeData, ()> {
    id: "demo",
    nodes,
    edges,
    on_nodes_change: move |changes| {
        nodes.set(apply_node_changes(changes, nodes.peek().clone()));
    },
    on_edges_change: move |changes| {
        edges.set(apply_edge_changes(changes, edges.peek().clone()));
    },
    on_connect: move |conn| {
        let mut next = edges.peek().clone();
        next.push(Edge::<()>::from_connection(&conn));
        edges.set(next);
    },
}
```

**Use this when:**

- You want to derive state from the nodes (selection count, layout
  validation, persistence to disk).
- You need to intercept changes (deny certain moves, normalize
  positions to a grid, etc.).

## 2. Controlled with `use_nodes_state` helper

The same as above, but the boilerplate is hidden in the hook:

```rust
let (nodes, on_nodes_change) = use_nodes_state::<BuiltInNodeData>(initial_nodes);
let (edges, on_edges_change) = use_edges_state::<()>(initial_edges);

RGraph { nodes, edges, on_nodes_change, on_edges_change, /* … */ }
```

See [`use_nodes_state`](../hooks/use-nodes-state.md).

## 3. Uncontrolled

Pass `default_nodes` / `default_edges` instead of `nodes` / `edges`:

```rust
RGraph::<BuiltInNodeData, ()> {
    id: "demo",
    default_nodes: vec![ /* … */ ],
    default_edges: vec![ /* … */ ],
}
```

rgraph manages the internal copy. To read or mutate later:

```rust
let h = use_rgraph::<BuiltInNodeData, ()>();
h.add_node(/* … */);
let snapshot = h.get_nodes();
```

See [Uncontrolled flow](./uncontrolled-flow.md) for the trade-offs.

## When to lift state higher

If multiple components need the node list (e.g. a sidebar listing
nodes, an inspector showing the selected node), keep it in the parent
component as a signal and pass it down through props or context:

```rust
let nodes = use_context_provider(|| Signal::new(Vec::<Node<BuiltInNodeData>>::new()));

// later, in any descendant:
let nodes = use_context::<Signal<Vec<Node<BuiltInNodeData>>>>();
```

## State persistence

The store's nodes/edges are plain `Vec<Node<N>> / Vec<Edge<E>>`. With
serde enabled (`features = ["serde-derive"]`), serialize them
directly:

```rust
#[derive(Serialize, Deserialize)]
struct SavedGraph {
    nodes:   Vec<Node<BuiltInNodeData>>,
    edges:   Vec<Edge<()>>,
    viewport: Viewport,
}
```

Reload by re-rendering `<RGraph>` with the new signal values. The
ResizeObserver auto-resyncs the pane bounding box on mount.
