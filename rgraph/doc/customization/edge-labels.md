# Edge labels

The simplest label is the `edge.label` field — a static string the
default edge renderers render with `<EdgeText>`. For interactive
labels (buttons, inputs, badges) use `<EdgeLabelRenderer>`.

## Static labels

```rust
Edge::<()>::minimal("e1", "n1", "n2")
    .with_label("hello")
    .with_label_style("fill:#444;font-size:11px;");
```

The default edge places the label at the midpoint computed by the
matching path builder. To draw the label yourself in a custom edge:

```rust
let path = get_bezier_path(GetBezierPathParams {
    source_x:        props.source_x,
    source_y:        props.source_y,
    target_x:        props.target_x,
    target_y:        props.target_y,
    source_position: props.source_position,
    target_position: props.target_position,
    curvature:       0.25,
});

rsx! {
    BaseEdge { id: props.id.clone(), path: path.0, /* … */ }
    if let Some(label) = &props.label {
        EdgeText {
            label_x: path.1,
            label_y: path.2,
            label:   label.clone(),
        }
    }
}
```

## Interactive labels

`<EdgeText>` draws a `<text>` SVG element — fine for plain text but
not for buttons, inputs, dropdowns, or anything that should respond
to clicks. For those, render HTML inside `<EdgeLabelRenderer>`:

```rust
EdgeLabelRenderer {
    div {
        style: "position: absolute;
                transform: translate(-50%, -50%) translate({path.1}px, {path.2}px);
                background: white; padding: 4px; border-radius: 4px;",
        button { onclick: move |_| remove_edge(&id), "×" }
    }
}
```

Under the hood, `<EdgeLabelRenderer>` is an inline div whose
`position: absolute` parent is the `<RGraph>` container — meaning the
content pans/zooms with the viewport because the surrounding
`viewport` group applies the transform.

> **Note.** React Flow uses `createPortal` here to escape the SVG. In
> Dioxus 0.7 the same effect is achieved with inline rendering at the
> correct DOM depth.

## Multiple labels per edge

You can render any number of `<EdgeLabelRenderer>` children inside one
edge component — for example a forward arrow label and a reverse arrow
label:

```rust
EdgeLabelRenderer {
    div {
        style: "position: absolute;
                transform: translate(-50%, -50%) translate({mid_x}px, {mid_y}px);",
        "→"
    }
}
EdgeLabelRenderer {
    div {
        style: "position: absolute;
                transform: translate(-50%, -50%) translate({mid_x}px, {mid_y - 14.0}px);",
        "fork"
    }
}
```

## Counter-zoom

To keep the label a constant pixel size regardless of zoom, divide its
content by the current zoom:

```rust
let zoom  = use_viewport().zoom;
let scale = 1.0 / zoom;

EdgeLabelRenderer {
    div {
        style: "position: absolute;
                transform: translate(-50%, -50%) translate({x}px, {y}px) scale({scale});",
        "constant-size label"
    }
}
```

[`EdgeToolbar`](../components/edge-toolbar.md) does exactly this
internally via `get_edge_toolbar_transform`.
