# `<NodeToolbar>`

Floating toolbar attached to one or more nodes. By default the
toolbar shows only when **exactly one** node is selected (override
with `is_visible: Some(true)`).

```rust
NodeToolbar::<BuiltInNodeData, ()> {
    button { onclick: move |_| delete_selected(),     "delete" }
    button { onclick: move |_| duplicate_selected(),  "duplicate" }
}
```

The toolbar's position is computed from each target node's bounding
box and the current viewport zoom — it stays anchored as the user
pans, zooms, or drags the node.

## Props

| Prop          | Type                          | Default               |
|---------------|-------------------------------|-----------------------|
| `node_id`     | `Option<NodeToolbarTarget>`   | inherit from context  |
| `class_name`  | `Option<String>`              | `None`                |
| `style`       | `Option<String>`              | `None`                |
| `is_visible`  | `Option<bool>`                | auto (1-node select)  |
| `position`    | `Position`                    | `Top`                 |
| `offset`      | `f64`                         | `10.0`                |
| `align`       | `Align`                       | `Center`              |
| `children`    | `Element`                     | (required)            |

`NodeToolbarTarget` accepts either a single node id or a `Vec<String>`:

```rust
NodeToolbar::<BuiltInNodeData, ()> {
    node_id: Some(NodeToolbarTarget::Many(vec!["a".into(), "b".into()])),
    /* … */
}
```

## Where to render

You can render `<NodeToolbar>` two ways:

1. **Inside a custom node** — the node id is picked up from
   `NodeIdContext` automatically:

   ```rust
   #[component]
   fn TaskNode(props: NodeProps<TaskData>) -> Element {
       rsx! {
           NodeToolbar::<TaskData, ()> {
               button { "delete" }
           }
           div { "{props.data.title}" }
       }
   }
   ```

2. **At the flow root** — pass `node_id` explicitly:

   ```rust
   RGraph::<BuiltInNodeData, ()> {
       /* … */
       NodeToolbar::<BuiltInNodeData, ()> {
           node_id: Some("n1".into()),
           is_visible: Some(true),
           "Always-on toolbar for node n1"
       }
   }
   ```

## Default visibility logic

When `is_visible` is `None` the toolbar is shown iff:

1. Exactly one node in the toolbar's target list is selected, **and**
2. The total number of selected nodes equals 1.

This matches the upstream "toolbar appears on single-selection only"
behavior.

## Z-index

The toolbar's z-index is set to `max(node.internals.z) + 1` so it
always renders on top of its node. Customize via inline `style` if
you need different layering.
