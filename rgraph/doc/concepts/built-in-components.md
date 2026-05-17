# Built-in components

`rgraph` ships seven optional helper components alongside the core
`<RGraph>` renderer. They mirror the upstream React Flow set 1:1 and
are re-exported from `rgraph::prelude`.

| Component                                         | Purpose                                          |
|---------------------------------------------------|--------------------------------------------------|
| [`<Background>`](../components/background.md)     | Dotted/lined/cross grid backdrop.                |
| [`<Controls>`](../components/controls.md)         | Zoom in/out, fit view, interactivity lock panel. |
| [`<MiniMap>`](../components/minimap.md)           | Overview panel with a viewport indicator.        |
| [`<NodeToolbar>`](../components/node-toolbar.md)  | Floating toolbar attached to a node.             |
| [`<EdgeToolbar>`](../components/edge-toolbar.md)  | Floating toolbar attached to an edge.            |
| [`<NodeResizer>`](../components/node-resizer.md)  | Drag handles around a node to resize it.         |
| `<Panel>`                                         | Generic position-aware floating container.       |

## Placement

All seven components must be rendered **as children of `<RGraph>`**
because they read the rgraph store via context:

```rust
RGraph::<BuiltInNodeData, ()> {
    id: "demo",
    nodes,
    edges,

    Background::<BuiltInNodeData, ()> {}
    Controls::<BuiltInNodeData, ()> {}
    MiniMap::<BuiltInNodeData> {}
}
```

The order doesn't matter — `<Background>` always renders behind the
nodes, `<MiniMap>` / `<Controls>` always overlay them.

## `<Panel>`

`<Panel>` is the generic container the other helpers build on top of.
It supports the eight standard positions and arbitrary HTML attribute
passthrough:

```rust
Panel {
    position:   Some(PanelPosition::TopRight),
    class_name: Some("my-panel".into()),
    style:      Some("background: white;".into()),

    "data-testid": "info-panel",
    "aria-label":  "Layout controls",

    button { "Re-layout" }
}
```

Attribute passthrough uses the `#[props(extends = GlobalAttributes, extends = div)]`
mechanism from `dioxus-core` — any global HTML attribute works.

## Default positions

| Component       | Default position         |
|-----------------|--------------------------|
| `<Background>`  | full-pane (no position)  |
| `<Controls>`    | `PanelPosition::BottomLeft` |
| `<MiniMap>`     | `PanelPosition::BottomRight` |
| `<NodeToolbar>` | `Position::Top`, `Align::Center` |
| `<EdgeToolbar>` | centered on the supplied `(x, y)` |

## Generic type parameters

Because rgraph stores are doubly-generic over node data `N` and edge
data `E`, some helpers carry those parameters too:

- `Background::<N, E>`
- `Controls::<N, E>`
- `MiniMap::<N>` (edges are not read)
- `NodeToolbar::<N, E>`
- `EdgeToolbar::<N, E>`
- `NodeResizer::<N, E>`

The defaults are `BuiltInNodeData` for `N` and `()` for `E`, so the
following also works in the common case:

```rust
Controls {}
MiniMap {}
```

Specify the parameters explicitly when your custom node data isn't
`BuiltInNodeData`:

```rust
Controls::<MyNodeData, MyEdgeData> {}
```
