# Custom edges

A custom edge is a Dioxus component that draws the SVG path between
two handle anchors. rgraph supplies five built-in edge types that
cover most cases — `default` (cubic bezier), `straight`, `step`,
`smoothstep`, `simplebezier` — but custom edges let you do anything
SVG can draw.

## 1. Anatomy of an edge component

Every edge component receives `EdgeProps<E>`:

```rust
use dioxus::prelude::*;
use rgraph::prelude::*;

#[component]
fn DashedEdge(props: EdgeProps<()>) -> Element {
    let path = get_simple_bezier_path(GetSimpleBezierPathParams {
        source_x:        props.source_x,
        source_y:        props.source_y,
        target_x:        props.target_x,
        target_y:        props.target_y,
        source_position: props.source_position,
        target_position: props.target_position,
    });
    rsx! {
        BaseEdge {
            id:       props.id.clone(),
            path:     path.0,
            style:    Some("stroke-dasharray: 6 3;".into()),
            marker_end: props.marker_end.clone(),
        }
    }
}
```

`BaseEdge` is the building block: it renders the actual `<path>`
element and wires up selection/interaction.

The four convenience path builders live under
`rgraph::components::edges`:

- `get_bezier_path(...)`
- `get_smooth_step_path(...)`
- `get_straight_path(...)`
- `get_simple_bezier_path(...)`

Each returns `(path_d, label_x, label_y, label_offset_x, label_offset_y)`
so you can use the second/third element to place an `<EdgeText>` or
custom label.

## 2. Register the type

```rust
let edge_types: EdgeTypes<()> = HashMap::from([
    ("dashed".to_string(), DashedEdge as EdgeRenderer<()>),
]);

RGraph::<BuiltInNodeData, ()> {
    id: "demo",
    nodes,
    edges,
    edge_types,
    /* … */
}
```

## 3. Emit edges of that type

```rust
Edge::<()>::minimal("e1", "n1", "n2").with_type("dashed");
```

## Edge anchors

`<EdgeAnchor>` is the optional draggable endpoint indicator shown
when an edge is selected and `edges_reconnectable` is on. It is
emitted automatically by the edge wrapper; custom edges don't need
to draw it themselves.

## Edge labels

For static labels use `<EdgeText>`:

```rust
EdgeText {
    label_x: path.1,
    label_y: path.2,
    label:   props.label.clone().unwrap_or_default(),
}
```

For richer labels (HTML, buttons, inputs) use `<EdgeLabelRenderer>` —
it renders its children in a screen-aligned, viewport-relative
container that scales/pans with the flow:

```rust
EdgeLabelRenderer {
    div {
        style: "position: absolute; transform: translate(-50%, -50%) translate({label_x}px, {label_y}px);",
        button { onclick: |_| {/* delete edge */}, "×" }
    }
}
```

See [Edge labels](./edge-labels.md) for the full guide.

## Markers

Arrows at edge endpoints come from SVG markers. Reference them by id
on `marker_start` / `marker_end`:

```rust
Edge::<()>::minimal("e1", "n1", "n2")
    .with_marker_end(EdgeMarker {
        type_: MarkerType::ArrowClosed,
        width: Some(20.0),
        height: Some(20.0),
        color: Some("#888".into()),
        ..Default::default()
    });
```

The marker is auto-registered in the SVG `<defs>` and reused for every
edge that references it.

## Tips

- **Use `BaseEdge`**. It handles selection state, focus rings,
  marker references, hover hit-area widening — much more than just
  drawing a path.
- **Don't subscribe to nodes inside an edge**. The renderer passes
  `source_x/source_y/target_x/target_y` props that already account for
  handle positions; reading the full node lookup is wasteful.
- **Animated edges**: set `edge.animated = true` on the data; the
  default CSS contains `@keyframes` driving `stroke-dashoffset`.
