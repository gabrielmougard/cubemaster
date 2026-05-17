# `<NodeResizer>`

Draws four corner handles + four side lines around a node, letting the
user drag them to resize the node. The resulting `NodeChange`s flow
through `trigger_node_changes` exactly like a drag-to-move, so your
controlled `on_nodes_change` callback (or `apply_node_changes`) picks
them up automatically.

```rust
#[component]
fn ResizableTaskNode(props: NodeProps<TaskData>) -> Element {
    rsx! {
        NodeResizer::<TaskData, ()> {
            min_width:  120.0,
            min_height: 60.0,
            color:      Some("#3b82f6".into()),
        }
        Handle::<TaskData, ()> { id: Some("t".into()), r#type: HandleType::Target,  position: Position::Top    }
        div { /* node body */ }
        Handle::<TaskData, ()> { id: Some("s".into()), r#type: HandleType::Source, position: Position::Bottom }
    }
}
```

## Props

| Prop                | Type            | Default     |
|---------------------|-----------------|-------------|
| `node_id`           | `Option<String>`| inherit     |
| `color`             | `Option<String>`| `None`      |
| `handle_class_name` | `Option<String>`| `None`      |
| `handle_style`      | `Option<String>`| `None`      |
| `line_class_name`   | `Option<String>`| `None`      |
| `line_style`        | `Option<String>`| `None`      |
| `is_visible`        | `bool`          | `true`      |
| `min_width`         | `f64`           | `10.0`      |
| `min_height`        | `f64`           | `10.0`      |
| `max_width`         | `f64`           | `f64::MAX`  |
| `max_height`        | `f64`           | `f64::MAX`  |
| `keep_aspect_ratio` | `bool`          | `false`     |
| `auto_scale`        | `bool`          | `true`      |

## How it works

Each of the eight controls (`NodeResizeControl`) instantiates an
[`XYResizer`](../../../rgraph-core/src/xyresizer/mod.rs) state
machine on mount. Pointer events translated through
`pointer_event_like` drive `handle_pointer_down/move/up/cancel`. The
state machine's `on_change` callback is wired to dispatch
`NodeChange::Position` + `NodeChange::Dimensions` entries to the
store's `trigger_node_changes`.

When the parent node has `expand_parent: true`, the resizer also
calls `handle_expand_parent` to push position+dimension updates onto
the parent group so it grows to enclose the child.

## Custom single-handle controls

For more bespoke UIs (resizable card, draggable header), use
`<NodeResizeControl>` directly:

```rust
NodeResizeControl::<TaskData, ()> {
    position: Some(ControlPosition::BottomRight),
    variant:  ResizeControlVariant::Handle,
    color:    Some("#ef4444".into()),
}
```

`position` accepts any [`ControlPosition`](../../../rgraph-core/src/xyresizer/types.rs)
(`TopLeft`, `Top`, `TopRight`, `Right`, `BottomRight`, `Bottom`,
`BottomLeft`, `Left`).

`variant` is either `Handle` (square dot drawn at the position) or
`Line` (full edge of the node).

## Resize direction

For unidirectional resize, pass `resize_direction`:

```rust
NodeResizeControl::<TaskData, ()> {
    position: Some(ControlPosition::Right),
    variant:  ResizeControlVariant::Line,
    resize_direction: Some(ResizeControlDirection::Horizontal),
}
```

`Horizontal` updates only `width`, `Vertical` only `height`. This is
useful when you want to expose only width adjustment (e.g. a Kanban
column).

## Boundaries

`min_width` / `min_height` / `max_width` / `max_height` are enforced
by the resizer state machine. If you also want to constrain position
(e.g. clamp the node inside a parent), set `parent_id` on the node
and `expand_parent: false` — the standard drag clamping then applies
during resize too.

## Aspect ratio

Set `keep_aspect_ratio: true` to lock width:height to the node's
current ratio at the start of the gesture. Useful for images or
square cards.
