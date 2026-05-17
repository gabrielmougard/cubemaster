# `use_viewport()`

A reactive read of the current viewport. Subscribes the calling
component so it re-renders whenever pan/zoom changes.

```rust
let v: Viewport = use_viewport();
println!("zoom = {}", v.zoom);
```

## Type

```rust
pub fn use_viewport() -> Viewport;

pub struct Viewport {
    pub x:    f64,
    pub y:    f64,
    pub zoom: f64,
}
```

## When to use it

- You want a viewport-dependent piece of UI (zoom indicator, counter-
  scaled label, screen-coords HUD).
- You need a re-render whenever the user pans or zooms.

For one-shot reads without a subscription, use `use_rgraph().viewport.get_viewport()`
instead — it reads via `peek()` and never triggers a re-render.

## `use_viewport_helper()`

The hook above returns a snapshot value. For the imperative API (zoom
in/out, fit, screen↔flow conversion) call
[`use_viewport_helper()`](../components/rgraph.md#example) or read
`use_rgraph().viewport`:

```rust
let v = use_viewport_helper::<BuiltInNodeData, ()>();
v.zoom_to(2.0, None);
v.fit_view(None);
v.set_viewport(PartialViewport {
    x: Some(0.0), y: Some(0.0), zoom: Some(1.5),
}, None);
```

`use_viewport_helper` and `use_rgraph().viewport` return the same
`ViewportHelper` struct.

## `use_on_viewport_change()`

Subscribe to viewport changes with start/change/end callbacks:

```rust
use_on_viewport_change(UseOnViewportChangeOptions {
    on_start:  Some(|v| println!("start at {:?}", v)),
    on_change: Some(|v| println!("change at {:?}", v)),
    on_end:    Some(|v| println!("end at {:?}", v)),
});
```

This is the same surface React Flow exposes through its hook of the
same name; it does **not** trigger re-renders for the caller (the
callbacks fire as side effects).
