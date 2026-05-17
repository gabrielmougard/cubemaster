# `<MiniMap>`

Overview panel showing every node and a viewport indicator. Optionally
pannable: drag inside the minimap to pan the parent viewport.

```rust
MiniMap::<BuiltInNodeData> {
    pannable: true,
}
```

## Props

| Prop                   | Type                                    | Default                |
|------------------------|-----------------------------------------|------------------------|
| `width`                | `f64`                                   | `200.0`                |
| `height`               | `f64`                                   | `150.0`                |
| `class_name`           | `Option<String>`                        | `None`                 |
| `style`                | `Option<String>`                        | `None`                 |
| `node_color`           | `Option<MiniMapNodeAttr<N>>`            | inherit                |
| `node_stroke_color`    | `Option<MiniMapNodeAttr<N>>`            | inherit                |
| `node_class_name`      | `Option<MiniMapNodeAttr<N>>`            | `None`                 |
| `node_border_radius`   | `f64`                                   | `5.0`                  |
| `node_stroke_width`    | `Option<f64>`                           | `None`                 |
| `bg_color`             | `Option<String>`                        | inherit                |
| `mask_color`           | `Option<String>`                        | inherit                |
| `mask_stroke_color`    | `Option<String>`                        | inherit                |
| `mask_stroke_width`    | `Option<f64>`                           | `None`                 |
| `position`             | `Option<PanelPosition>`                 | `BottomRight`          |
| `on_click`             | `Option<EventHandler<(MouseEvent, (f64,f64))>>` | `None`         |
| `on_node_click`        | `Option<EventHandler<(MouseEvent, String)>>`    | `None`         |
| `pannable`             | `bool`                                  | `false`                |
| `zoomable`             | `bool`                                  | `false` (deferred)     |
| `aria_label`           | `Option<String>`                        | `"Mini Map"`           |
| `inverse_pan`          | `bool`                                  | `false`                |
| `zoom_step`            | `f64`                                   | `1.0`                  |
| `offset_scale`         | `f64`                                   | `5.0`                  |

> **Status note.** Pan-on-drag inside the minimap is wired through
> [`XYMiniMap`](../../../rgraph-core/src/xyminimap/mod.rs). Wheel-zoom
> inside the minimap (`zoomable`) currently has no effect — it
> requires the same wheel-event plumbing as `<ZoomPane>` and will
> land in a follow-up phase.

## Color attributes

`MiniMapNodeAttr<N>` accepts a static string or a closure that
computes the value per-node:

```rust
MiniMap::<TaskData> {
    node_color: Some(MiniMapNodeAttr::Dynamic(Rc::new(|node| {
        match node.user.data.status {
            TaskStatus::Done  => "#16a34a".to_string(),
            TaskStatus::Doing => "#2563eb".to_string(),
            TaskStatus::Todo  => "#a3a3a3".to_string(),
        }
    }))),
}
```

The closure receives an [`InternalNode<N>`](../../../rgraph-core/src/types/nodes.rs)
so you have access to the full data + measured size + selection
state.

## Custom node rendering

`MiniMap` renders each node as a `<rect>` by default. To render
something else (a circle, a path, a sub-icon), provide a custom
component as `node_component`. *(API surface; rendering hook to be
exposed when there's a real need — for the MVP it ships rectangles
only.)*

## Click-to-pan

The standard `onclick` handler on the minimap receives the click
position in screen coordinates:

```rust
MiniMap::<BuiltInNodeData> {
    on_click: |(evt, (x, y))| {
        println!("clicked minimap at screen ({x},{y})");
    },
}
```

Use `viewport.screen_to_flow_position` from
[`use_rgraph()`](../hooks/use-rgraph.md) if you need flow-space
coordinates instead.

## Performance

The minimap subscribes only to:

- the node lookup signal (re-renders when nodes are added/removed/moved),
- the viewport transform signal (re-renders on pan/zoom),
- the pane size signal.

It does *not* read individual node selection state, so toggling node
selections does not cause minimap re-renders.
