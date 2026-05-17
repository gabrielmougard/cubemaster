# `<RGraph>`

Top-level component. Renders the pane, viewport, node renderer, edge
renderer, attribution chip and any helper components you pass as
children. **Every** rgraph app starts with this component.

## Required props

| Prop  | Type                | Description                                                   |
|-------|---------------------|---------------------------------------------------------------|
| `id`  | `String`            | Unique DOM id. Used as the ResizeObserver selector — must be a valid CSS id. |

All other props are optional with sensible defaults.

## Generic type parameters

```rust
RGraph::<N, E> { /* … */ }
```

- `N` — your node data type. Defaults to `BuiltInNodeData`.
- `E` — your edge data type. Defaults to `()`.

## Data props

| Prop                  | Type                              | Default                  |
|-----------------------|-----------------------------------|--------------------------|
| `nodes`               | `Signal<Vec<Node<N>>>`            | empty                    |
| `edges`               | `Signal<Vec<Edge<E>>>`            | empty                    |
| `default_nodes`       | `Vec<Node<N>>`                    | `vec![]` (uncontrolled)  |
| `default_edges`       | `Vec<Edge<E>>`                    | `vec![]` (uncontrolled)  |
| `node_types`          | `HashMap<String, NodeRenderer<N>>`| built-in types only      |
| `edge_types`          | `HashMap<String, EdgeRenderer<E>>`| built-in types only      |

If you pass `nodes` *and* `on_nodes_change`, the flow is controlled.
If you only pass `default_nodes`, the flow is uncontrolled — see
[Uncontrolled flow](../advanced/uncontrolled-flow.md).

## Viewport props

| Prop                    | Type                       | Default          |
|-------------------------|----------------------------|------------------|
| `default_viewport`      | `Viewport`                 | `(0,0,1.0)`      |
| `min_zoom`              | `f64`                      | `0.5`            |
| `max_zoom`              | `f64`                      | `2.0`            |
| `translate_extent`      | `CoordinateExtent`         | `[-∞, +∞]`       |
| `node_extent`           | `CoordinateExtent`         | `[-∞, +∞]`       |
| `fit_view`              | `bool`                     | `false`          |
| `fit_view_options`      | `Option<FitViewOptionsBase>` | `None`         |
| `node_origin`           | `NodeOrigin` (`(f64, f64)`) | `(0.0, 0.0)`    |
| `pan_on_drag`           | `PanOnDrag`                | `On`             |
| `pan_on_scroll`         | `bool`                     | `false`          |
| `pan_on_scroll_mode`    | `PanOnScrollMode`          | `Free`           |
| `pan_on_scroll_speed`   | `f64`                      | `0.5`            |
| `zoom_on_scroll`        | `bool`                     | `true`           |
| `zoom_on_pinch`         | `bool`                     | `true`           |
| `zoom_on_double_click`  | `bool`                     | `true`           |
| `zoom_activation_key_code` | `Option<KeyPressMatcher>` | `Some("Meta")` |
| `pan_activation_key_code`  | `Option<KeyPressMatcher>` | `Some("Space")`|
| `prevent_scrolling`     | `bool`                     | `true`           |

## Selection props

| Prop                          | Type                       | Default          |
|-------------------------------|----------------------------|------------------|
| `nodes_draggable`             | `bool`                     | `true`           |
| `nodes_connectable`           | `bool`                     | `true`           |
| `elements_selectable`         | `bool`                     | `true`           |
| `nodes_focusable`             | `bool`                     | `true`           |
| `edges_focusable`             | `bool`                     | `true`           |
| `auto_pan_on_node_focus`      | `bool`                     | `true`           |
| `auto_pan_on_connect`         | `bool`                     | `true`           |
| `selection_key_code`          | `Option<KeyPressMatcher>`  | `Some("Shift")`  |
| `multi_selection_key_code`    | `Option<KeyPressMatcher>`  | `Some("Control")` (set to `"Meta"` on macOS) |
| `delete_key_code`             | `Option<KeyPressMatcher>`  | `Some("Backspace")` |
| `selection_mode`              | `SelectionMode`            | `Partial`        |
| `select_nodes_on_drag`        | `bool`                     | `true`           |
| `connect_on_click`            | `bool`                     | `true`           |
| `nodes_drag_handle`           | `Option<String>`           | `None`           |

## Edge props

| Prop                       | Type                | Default          |
|----------------------------|---------------------|------------------|
| `default_edge_options`     | `DefaultEdgeOptions`| `Default::default()` |
| `edges_reconnectable`      | `bool`              | `true`           |
| `reconnect_radius`         | `f64`               | `10.0`           |
| `only_render_visible_elements` | `bool`          | `false`          |
| `elevate_edges_on_select`  | `bool`              | `false`          |
| `elevate_nodes_on_select`  | `bool`              | `true`           |
| `default_marker_color`     | `String`            | `"#b1b1b7"`      |

## Callback props

The full list lives in
[`rgraph::types::component_props`](../../src/types/component_props.rs);
the most commonly wired:

| Prop                     | Argument type                          |
|--------------------------|----------------------------------------|
| `on_nodes_change`        | `Vec<NodeChange<N>>`                    |
| `on_edges_change`        | `Vec<EdgeChange<E>>`                    |
| `on_node_click`          | `NodeMouseHandlerArgs<N>`               |
| `on_node_drag_start`     | `OnNodeDragArgs<N>`                     |
| `on_node_drag`           | `OnNodeDragArgs<N>`                     |
| `on_node_drag_stop`      | `OnNodeDragArgs<N>`                     |
| `on_edge_click`          | `EdgeMouseHandlerArgs<E>`               |
| `on_pane_click`          | `MouseData`                             |
| `on_pane_context_menu`   | `MouseData`                             |
| `on_connect`             | `Connection`                            |
| `on_connect_start`       | `OnConnectStartCallbackArgs`            |
| `on_connect_end`         | `OnConnectEndCallbackArgs`              |
| `on_reconnect`           | `OnReconnectArgs<E>`                    |
| `on_reconnect_start`     | `OnReconnectStartArgs<E>`               |
| `on_reconnect_end`       | `OnReconnectEndArgs<E>`                 |
| `on_move`                | `OnMoveCallbackArgs`                    |
| `on_move_start`          | `OnMoveCallbackArgs`                    |
| `on_move_end`            | `OnMoveCallbackArgs`                    |
| `on_init`                | `RGraphInstance<N, E>`                  |
| `on_selection_change`    | `OnSelectionChangeParams<N, E>`         |
| `on_delete`              | `DeletedElements<N, E>`                 |
| `on_before_delete`       | returns `bool` (veto)                   |
| `is_valid_connection`    | `&Connection` → `bool`                  |

## Class / style

| Prop              | Default                       |
|-------------------|-------------------------------|
| `class_name`      | `None`                        |
| `style`           | `None`                        |
| `proOptions`      | `None`                        |
| `nodes_drag_handle` | `None`                      |

## Example

```rust
use dioxus::prelude::*;
use rgraph::prelude::*;
use rgraph_core::types::geometry::XYPosition;

#[component]
fn Demo() -> Element {
    let mut nodes = use_signal(|| vec![
        Node::<BuiltInNodeData>::with_data("a", XYPosition::new(0.0, 0.0), BuiltInNodeData::labelled("A")),
        Node::<BuiltInNodeData>::with_data("b", XYPosition::new(200.0, 100.0), BuiltInNodeData::labelled("B")),
    ]);
    let mut edges = use_signal(|| vec![
        Edge::<()>::minimal("a-b", "a", "b"),
    ]);

    rsx! {
        div { style: "width: 100vw; height: 100vh;",
            RGraph::<BuiltInNodeData, ()> {
                id: "demo",
                nodes,
                edges,
                fit_view: true,
                min_zoom: 0.25,
                max_zoom: 4.0,
                on_nodes_change: move |changes| {
                    let next = apply_node_changes(changes, nodes.peek().clone());
                    nodes.set(next);
                },
                on_edges_change: move |changes| {
                    let next = apply_edge_changes(changes, edges.peek().clone());
                    edges.set(next);
                },
                on_connect: move |conn| {
                    let mut next = edges.peek().clone();
                    next.push(Edge::<()>::from_connection(&conn));
                    edges.set(next);
                },
                Background::<BuiltInNodeData, ()> {}
                Controls::<BuiltInNodeData, ()> {}
                MiniMap::<BuiltInNodeData> { pannable: true }
            }
        }
    }
}
```
