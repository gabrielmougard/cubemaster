# Drag-and-drop nodes from a sidebar

A common pattern: a sidebar with draggable node templates that the
user can drop onto the canvas to instantiate a new node.

## 1. Sidebar

Use plain Dioxus drag-and-drop on the source elements:

```rust
#[component]
fn Sidebar() -> Element {
    rsx! {
        aside { style: "width: 180px; padding: 8px;",
            for kind in ["input", "default", "output"] {
                div {
                    draggable: true,
                    style: "padding: 6px 10px; border: 1px solid #ccc; margin: 4px; cursor: grab;",
                    ondragstart: move |evt| {
                        evt.data_transfer().set_data("application/rgraph", kind);
                        evt.data_transfer().set_effect_allowed(DragEffect::Move);
                    },
                    "{kind} node"
                }
            }
        }
    }
}
```

## 2. The drop zone

The `<RGraph>` wrapper handles the `ondrop` event. Convert the screen
coordinates to flow-space via the viewport helper:

```rust
#[component]
fn Flow() -> Element {
    let h = use_rgraph::<BuiltInNodeData, ()>();
    let mut nodes = use_signal(Vec::<Node<BuiltInNodeData>>::new);
    let mut edges = use_signal(Vec::<Edge<()>>::new);

    let on_drop = move |evt: DragEvent| {
        let kind = evt.data_transfer().get_data("application/rgraph");
        if kind.is_empty() { return; }

        let coords = evt.client_coordinates();
        let flow_pos = h.viewport.screen_to_flow_position(
            XYPosition::new(coords.x, coords.y),
            ScreenToFlowOptions::default(),
        );

        let id = format!("n{}", nodes.peek().len() + 1);
        let new_node = Node::<BuiltInNodeData>::with_data(
            &id, flow_pos, BuiltInNodeData::labelled(&kind),
        )
        .with_type(&kind);
        let mut next = nodes.peek().clone();
        next.push(new_node);
        nodes.set(next);
    };

    rsx! {
        div { style: "display: flex; width: 100vw; height: 100vh;",
            Sidebar {}
            div { style: "flex: 1;",
                ondragover: |evt| evt.prevent_default(),
                ondrop:     on_drop,
                RGraph::<BuiltInNodeData, ()> {
                    id: "demo",
                    nodes,
                    edges,
                    on_nodes_change: move |c| {
                        let next = apply_node_changes(c, nodes.peek().clone());
                        nodes.set(next);
                    },
                    on_edges_change: move |c| {
                        let next = apply_edge_changes(c, edges.peek().clone());
                        edges.set(next);
                    },
                    on_connect: move |conn| {
                        let mut next = edges.peek().clone();
                        next.push(Edge::<()>::from_connection(&conn));
                        edges.set(next);
                    },
                    Background::<BuiltInNodeData, ()> {}
                    Controls::<BuiltInNodeData, ()> {}
                }
            }
        }
    }
}
```

## Notes

- `evt.prevent_default()` on `ondragover` is **required** for `ondrop`
  to fire (HTML spec).
- `screen_to_flow_position` accounts for the pane's offset relative
  to the document and the current viewport. If you skip it, the node
  drops far from the cursor when the user has scrolled or zoomed.
- The `kind` string round-trips through the `DataTransfer` API, which
  is the same API web apps use — works seamlessly in Dioxus desktop's
  Chromium webview.

## Variations

- **Drop from outside the flow**: register the `<div>` wrapping
  `<RGraph>` instead of the canvas itself, so dropping anywhere over
  the flow region creates a node.
- **Snap to grid**: pass `snap_to_grid: true` + `snap_grid: (20.0, 20.0)`
  to `<RGraph>` so dropped nodes align automatically.
- **Initial connection**: after creating the node, immediately push an
  edge to it (e.g. from the currently-selected node).
