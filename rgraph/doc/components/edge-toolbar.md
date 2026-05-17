# `<EdgeToolbar>`

Floating toolbar attached to a specific edge. Useful for "delete edge"
buttons and inline reconnect handles.

```rust
EdgeToolbar::<BuiltInNodeData, ()> {
    edge_id: edge.id.clone(),
    x:       midpoint_x,
    y:       midpoint_y,

    button { onclick: move |_| delete_edge(&id), "×" }
}
```

The toolbar is positioned in flow-space at `(x, y)` and counter-scaled
by `1 / zoom` so it stays the same physical size regardless of zoom.

## Props

| Prop          | Type                          | Default     |
|---------------|-------------------------------|-------------|
| `edge_id`     | `String`                      | required    |
| `x`           | `f64`                         | required    |
| `y`           | `f64`                         | required    |
| `class_name`  | `Option<String>`              | `None`      |
| `style`       | `Option<String>`              | `None`      |
| `is_visible`  | `Option<bool>`                | follows `edge.selected` |
| `align_x`     | `AlignX`                      | `Center`    |
| `align_y`     | `AlignY`                      | `Center`    |
| `children`    | `Element`                     | required    |

## Typical placement

In a custom edge, compute the path midpoint from the path builder
output and pass it to `<EdgeToolbar>`:

```rust
#[component]
fn DeletableEdge(props: EdgeProps<()>) -> Element {
    let p = get_bezier_path(GetBezierPathParams {
        source_x:        props.source_x,
        source_y:        props.source_y,
        target_x:        props.target_x,
        target_y:        props.target_y,
        source_position: props.source_position,
        target_position: props.target_position,
        curvature:       0.25,
    });
    let id = props.id.clone();
    let on_delete = move |_| { /* dispatch removal */ };
    rsx! {
        BaseEdge { id: props.id.clone(), path: p.0, /* … */ }
        EdgeToolbar::<BuiltInNodeData, ()> {
            edge_id: props.id.clone(),
            x: p.1,
            y: p.2,
            button { onclick: on_delete, "×" }
        }
    }
}
```

## Internals

`<EdgeToolbar>` renders its children inside
[`<EdgeLabelRenderer>`](../customization/edge-labels.md), which places
content in a screen-aligned overlay above the SVG. The `transform`
string is computed by `get_edge_toolbar_transform` from `rgraph_core`.

## Z-index

The toolbar's z-index is `edge.z_index + 1`, so it stacks above the
edge but below any node that also overlaps it. To force on-top
rendering append `z-index: 1000;` to the inline `style` prop.
