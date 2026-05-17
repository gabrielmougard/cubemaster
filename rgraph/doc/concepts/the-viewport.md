# The viewport

The viewport is the visible portion of the flow expressed as the
3-tuple `(x, y, zoom)`:

- `x` / `y` — pan offset in CSS pixels.
- `zoom` — multiplier (`1.0` is identity, `2.0` doubles every length).

The current viewport lives at `store.transform: Signal<Transform>`. A
`Transform` is a `(tx, ty, scale)` struct with `tx() / ty() / scale()`
accessors.

## Reading the viewport

```rust
let viewport = use_viewport(); // a reactive copy
println!("zoom = {}", viewport.zoom);
```

For one-shot reads (no subscription), use the imperative handle:

```rust
let h = use_rgraph::<BuiltInNodeData, ()>();
let v: Viewport = h.viewport.get_viewport();
println!("zoom = {}", v.zoom);
```

## Mutating the viewport

All viewport mutations go through the
[`ViewportHelper`](../../src/hooks/use_viewport_helper.rs) returned by
`use_rgraph().viewport`:

```rust
let v = use_rgraph::<BuiltInNodeData, ()>().viewport;

v.zoom_in(None);                  // ×1.2
v.zoom_out(None);                 // ×(1/1.2)
v.zoom_to(2.0, None);             // absolute zoom level

v.set_viewport(PartialViewport {
    x:    Some(120.0),
    y:    Some(80.0),
    zoom: Some(1.5),
}, None);

v.set_center(0.0, 0.0, None);     // center the flow on (0,0)
v.fit_bounds(rect, None);          // fit a specific rectangle
v.fit_view(None);                  // fit all visible nodes
```

`fit_view` accepts an optional [`FitViewOptionsBase`](../../../rgraph-core/src/types/viewport.rs)
to filter by node id, override padding, etc.

## Coordinate conversion

Two helpers convert between screen-space and flow-space:

```rust
let flow_pos:   XYPosition = v.screen_to_flow_position(
    XYPosition::new(evt.client_x, evt.client_y),
    ScreenToFlowOptions::default(),
);

let screen_pos: XYPosition = v.flow_to_screen_position(flow_node_pos);
```

`screen_to_flow_position` reads the pane's cached bounding box
(populated by `<ZoomPane>`/`use_resize_handler` on mount and on
window resize) so you don't have to read `getBoundingClientRect()`
yourself.

## Pan/zoom configuration

The behavior is configured through `<RGraph>` props:

| Prop                 | Type             | Default     |
|----------------------|------------------|-------------|
| `min_zoom`           | `f64`            | `0.5`       |
| `max_zoom`           | `f64`            | `2.0`       |
| `translate_extent`   | `CoordinateExtent` | `[-∞, ∞]` |
| `node_extent`        | `CoordinateExtent` | `[-∞, ∞]` |
| `default_viewport`   | `Viewport`       | `(0,0,1.0)` |
| `pan_on_drag`        | `PanOnDrag`      | `On`        |
| `pan_on_scroll`      | `bool`           | `false`     |
| `pan_on_scroll_mode` | `PanOnScrollMode`| `Free`      |
| `pan_on_scroll_speed`| `f64`            | `0.5`       |
| `zoom_on_scroll`     | `bool`           | `true`      |
| `zoom_on_pinch`      | `bool`           | `true`      |
| `zoom_on_double_click` | `bool`         | `true`      |
| `zoom_activation_key_code` | `Option<KeyPressMatcher>` | `Some("Meta")` |
| `pan_activation_key_code`  | `Option<KeyPressMatcher>` | `Some("Space")` |

See [`<RGraph>` reference](../components/rgraph.md) for the full list.

## ViewportPortal

If you want to render arbitrary content **inside** the flow-space
(so it pans/zooms with the nodes), wrap it in `<ViewportPortal>`:

```rust
ViewportPortal::<BuiltInNodeData, ()> {
    div { style: "position: absolute; left: 100px; top: 100px;",
        "I live in flow-space!"
    }
}
```

`<ViewportPortal>` renders into the same SVG-aligned container the
node renderer uses, so transforms cascade automatically.
