# `<Controls>`

A small panel with zoom-in/zoom-out/fit-view buttons plus an
interactivity-lock toggle.

```rust
Controls::<BuiltInNodeData, ()> {}
```

## Props

| Prop                   | Type                            | Default                       |
|------------------------|---------------------------------|-------------------------------|
| `show_zoom`            | `bool`                          | `true`                        |
| `show_fit_view`        | `bool`                          | `true`                        |
| `show_interactive`     | `bool`                          | `true`                        |
| `fit_view_options`     | `Option<Rc<FitViewOptionsBase>>`| `None`                        |
| `on_zoom_in`           | `EventHandler<()>`              | `None`                        |
| `on_zoom_out`          | `EventHandler<()>`              | `None`                        |
| `on_fit_view`          | `EventHandler<()>`              | `None`                        |
| `on_interactive_change`| `EventHandler<bool>`            | `None`                        |
| `position`             | `Option<PanelPosition>`         | `BottomLeft`                  |
| `orientation`          | `Option<ControlsOrientation>`   | `Vertical`                    |
| `class_name`           | `Option<String>`                | `None`                        |
| `style`                | `Option<String>`                | `None`                        |
| `aria_label`           | `Option<String>`                | `"React Flow controls"`       |

## Buttons

| Button       | Calls                                     |
|--------------|-------------------------------------------|
| Zoom in      | `viewport.zoom_in(None)` then `on_zoom_in`   |
| Zoom out     | `viewport.zoom_out(None)` then `on_zoom_out` |
| Fit view     | `viewport.fit_view(fit_view_options)` then `on_fit_view` |
| Interactive  | Flips `nodes_draggable | nodes_connectable | elements_selectable` together |

The fit-view button calls the real
[`ViewportHelper::fit_view`](../hooks/use-rgraph.md) helper —
which delegates to `rgraph_core::utils::graph::fit_viewport`.

## Custom buttons

Add your own buttons by passing `<ControlButton>` children:

```rust
Controls::<BuiltInNodeData, ()> {
    ControlButton {
        on_click: move |_| { /* re-layout */ },
        title:    Some("Auto-layout".into()),
        "↳"
    }
}
```

`<ControlButton>` accepts the same standard props as a `<button>`
(`title`, `aria_label`, `disabled`, `class_name`, `on_click`,
`children`). Hover/focus styles follow the same CSS the built-in
buttons use.

## Disabling buttons

`<Controls>` automatically disables zoom-in when `transform.scale() >= max_zoom`
and zoom-out when `transform.scale() <= min_zoom`. Custom buttons can
opt into the same behavior by reading the store directly:

```rust
let zoom = use_viewport().zoom;
ControlButton {
    disabled: zoom >= 2.0,
    on_click: move |_| { /* … */ },
    "x2"
}
```

## Orientation

`Controls` defaults to a vertical stack of buttons. Switch with
`orientation: Some(ControlsOrientation::Horizontal)` to lay them out
in a single row.
