# Custom nodes

rgraph ships four built-in node variants (`default`, `input`,
`output`, `group`), but the real power is rendering your own Dioxus
components inside a node. This page walks through writing a custom
node from scratch.

## 1. Define your node data

Custom nodes pick their own data type. It must implement
`Clone + PartialEq + 'static`:

```rust
#[derive(Clone, PartialEq)]
pub struct TaskData {
    pub title:  String,
    pub status: TaskStatus,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus { Todo, Doing, Done }
```

## 2. Write the component

A node component receives `NodeProps<TaskData>`:

```rust
use dioxus::prelude::*;
use rgraph::prelude::*;

#[component]
fn TaskNode(props: NodeProps<TaskData>) -> Element {
    let bg = match props.data.status {
        TaskStatus::Todo  => "#fde68a",
        TaskStatus::Doing => "#93c5fd",
        TaskStatus::Done  => "#86efac",
    };
    rsx! {
        Handle::<TaskData, ()> {
            id: Some("in".into()),
            r#type: HandleType::Target,
            position: Position::Top,
        }
        div { style: "padding: 10px; background: {bg}; border-radius: 6px;",
            strong { "{props.data.title}" }
        }
        Handle::<TaskData, ()> {
            id: Some("out".into()),
            r#type: HandleType::Source,
            position: Position::Bottom,
        }
    }
}
```

A few things to notice:

- **Always emit `<Handle>` components.** The renderer doesn't draw
  handles for you on custom nodes.
- **`#[component]` with `NodeProps<D>`.** The macro auto-derives
  `Props` for you; field access is `props.data`, `props.id`,
  `props.selected`, etc.

## 3. Register the type

Custom node types are dispatched by string. Compose a closure that
picks the right component based on `node.type_`:

```rust
let node_types: NodeTypes<TaskData> = HashMap::from([
    ("task".to_string(), TaskNode as NodeRenderer<TaskData>),
]);

// then pass it to <RGraph>
RGraph::<TaskData, ()> {
    id: "tasks",
    nodes,
    edges,
    node_types,
    /* … */
}
```

If `node.type_` is `Some("task")` the renderer dispatches to
`TaskNode`. Any other (or `None`) falls back to the built-in default
node and would expect `D = BuiltInNodeData`.

## 4. Emit nodes of that type

```rust
let n = Node::<TaskData>::with_data(
    "n1",
    XYPosition::new(80.0, 60.0),
    TaskData { title: "Write docs".into(), status: TaskStatus::Doing },
)
.with_type("task");
```

`Node::with_type` is a builder that sets `type_`. The matching
`node_types` entry then drives the rendering.

## Handles

A `<Handle>` is the connection anchor. Its key props:

| Prop          | Type                       | Default     |
|---------------|----------------------------|-------------|
| `id`          | `Option<String>`           | `None`      |
| `r#type`      | `HandleType` (`Source` / `Target`) | required |
| `position`    | `Position`                 | required    |
| `is_connectable` | `Option<bool>`          | inherit     |
| `is_connectable_start` | `Option<bool>`    | `None`      |
| `is_connectable_end`   | `Option<bool>`    | `None`      |

A node with **no** handles is selectable but not connectable. A node
with one `Source` handle is a "source-only" node (like the built-in
`input`).

## Multiple handles

Each handle needs a unique `id` when there's more than one of the
same type:

```rust
Handle::<TaskData, ()> {
    id: Some("left".into()),
    r#type: HandleType::Target,
    position: Position::Left,
}
Handle::<TaskData, ()> {
    id: Some("right".into()),
    r#type: HandleType::Source,
    position: Position::Right,
}
```

Edges then reference them via `source_handle` / `target_handle`:

```rust
Edge::<()>::minimal("e1", "n1", "n2")
    .with_source_handle("right")
    .with_target_handle("left");
```

## Sizing

By default the node sizes to its content via the ResizeObserver bridge.
If you need an explicit size, set `node.width` / `node.height` or apply
CSS dimensions inside your component.

## Tips

- **Memoize**. Use `use_memo`/`use_signal` inside the component to avoid
  recomputing on every render.
- **Don't subscribe to the whole store**. Read only the signals you
  need (e.g. `store.transform.read()` for screen-aligned widgets) so
  unrelated updates don't re-render your node.
- **For inline edits**, capture pointer events on the inner
  contenteditable element. The default `nodrag` CSS class can be added
  to inputs/buttons to disable drag-to-move on them.
