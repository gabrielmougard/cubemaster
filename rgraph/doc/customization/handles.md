# Handles

A **handle** is the anchor point on a node where an edge starts
(`HandleType::Source`) or ends (`HandleType::Target`). The built-in
nodes draw their handles automatically; custom nodes emit them
manually with the `<Handle>` component.

## Anatomy

```rust
use rgraph::prelude::*;

Handle::<MyData, ()> {
    id:        Some("a".into()),
    r#type:    HandleType::Source,
    position:  Position::Right,
}
```

| Prop                    | Type              | Default   | Description                                  |
|-------------------------|-------------------|-----------|----------------------------------------------|
| `id`                    | `Option<String>`  | `None`    | Required when a node has multiple handles of the same type. |
| `r#type`                | `HandleType`      | required  | `Source` or `Target`.                        |
| `position`              | `Position`        | required  | `Top`, `Right`, `Bottom`, or `Left`.         |
| `is_connectable`        | `Option<bool>`    | inherit   | Override the global `nodes_connectable`.     |
| `is_connectable_start`  | `Option<bool>`    | `None`    | If `false`, dragging *from* this handle is disabled. |
| `is_connectable_end`    | `Option<bool>`    | `None`    | If `false`, dropping a connection *on* this handle is disabled. |
| `on_connect`            | `Option<EventHandler<Connection>>` | `None` | Per-handle connect callback. |
| `style`                 | `Option<String>`  | `None`    | Inline style snippet.                         |
| `class_name`            | `Option<String>`  | `None`    | Extra class names.                            |

## Multiple handles

Each handle of the same type must have a unique `id`:

```rust
Handle::<MyData, ()> {
    id:       Some("top".into()),
    r#type:   HandleType::Target,
    position: Position::Top,
}
Handle::<MyData, ()> {
    id:       Some("left".into()),
    r#type:   HandleType::Target,
    position: Position::Left,
}
Handle::<MyData, ()> {
    id:       Some("out".into()),
    r#type:   HandleType::Source,
    position: Position::Right,
}
```

Edges then target a specific handle via `source_handle` / `target_handle`:

```rust
Edge::<()>::minimal("e1", "src", "dst")
    .with_source_handle("out")
    .with_target_handle("top");
```

## Connection validation

Reject specific connections from the `<RGraph>` `is_valid_connection`
prop:

```rust
RGraph::<MyData, ()> {
    /* … */
    is_valid_connection: |conn: &Connection| -> bool {
        // disallow self-loops
        conn.source != conn.target
    },
}
```

For per-handle validation, write the same check in the handle's
`on_connect` callback and short-circuit there.

## Styling

The default CSS gives handles an 8 px circle. Override by composing
extra classes or inline styles:

```rust
Handle::<MyData, ()> {
    id: Some("a".into()),
    r#type: HandleType::Source,
    position: Position::Right,
    class_name: Some("big-handle".into()),
    style:      Some("width: 16px; height: 16px;".into()),
}
```

The handle is positioned in its parent node's coordinate system. To
nudge it relative to the side you can apply `transform: translate(…)`
in your CSS — typically when you want a handle inside the node's
padding box instead of on the border.

## Connectable callbacks per node

A node-level `connectable: false` disables every handle on that node.
The store has corresponding signals:

- `store.nodes_connectable` — global on/off (defaults `true`)
- `node.connectable` — per-node override
- `handle.is_connectable` — per-handle override

The most specific override wins.

## Pointer mechanics

When the user presses on a handle, `<Handle>` starts an
`XYHandle`-equivalent state machine that:

1. captures the pointer,
2. draws a transient `<ConnectionLine>` from the handle to the cursor,
3. tests every other handle under the cursor for compatibility (type,
   `is_connectable_end`, `is_valid_connection`),
4. fires `on_connect` on release.

Most of this happens inside [`<ZoomPane>`](../../src/container/zoom_pane.rs)
and [`<Handle>`](../../src/components/handle.rs); custom nodes only
need to emit the handle, the renderer handles the rest.
