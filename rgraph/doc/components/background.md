# `<Background>`

Renders a dotted, lined, or cross-hatched grid behind the nodes. The
pattern automatically follows the viewport's pan/zoom.

```rust
Background::<BuiltInNodeData, ()> {
    variant: BackgroundVariant::Dots,
    gap:     BackgroundGap::Uniform(20.0),
    color:   Some("#cccccc".into()),
}
```

## Props

| Prop                  | Type                       | Default                                |
|-----------------------|----------------------------|----------------------------------------|
| `id`                  | `Option<String>`           | `None` (one unique pattern per flow)   |
| `variant`             | `BackgroundVariant`        | `Dots`                                 |
| `gap`                 | `BackgroundGap`            | `Uniform(20.0)`                        |
| `size`                | `Option<f64>`              | `1.0` (`Dots`/`Lines`), `6.0` (`Cross`) |
| `line_width`          | `f64`                      | `1.0`                                  |
| `offset`              | `BackgroundOffset`         | `Uniform(0.0)`                         |
| `color`               | `Option<String>`           | inherit from CSS var                   |
| `bg_color`            | `Option<String>`           | inherit                                |
| `style`               | `Option<String>`           | `None`                                 |
| `class_name`          | `Option<String>`           | `None`                                 |
| `pattern_class_name`  | `Option<String>`           | `None`                                 |

`BackgroundGap` and `BackgroundOffset` are aliases of the same enum:

```rust
pub enum BackgroundGap {
    Uniform(f64),
    Sided(f64, f64),
}
```

## Variants

```rust
BackgroundVariant::Dots   // default — single circle per cell
BackgroundVariant::Lines  // single vertical + horizontal line
BackgroundVariant::Cross  // plus-shaped marker
```

## Multiple backgrounds

If you need a layered effect (e.g. a coarse grid and a fine grid),
render two `<Background>` instances and disambiguate with the `id`
prop:

```rust
Background::<BuiltInNodeData, ()> {
    id: Some("coarse".into()),
    gap: BackgroundGap::Uniform(80.0),
    color: Some("#dddddd".into()),
}
Background::<BuiltInNodeData, ()> {
    id: Some("fine".into()),
    gap: BackgroundGap::Uniform(20.0),
    color: Some("#eeeeee".into()),
}
```

## Performance

The grid is a single `<svg>` with a `<pattern>` — pan/zoom updates
only mutate the pattern's `x`/`y`/`width`/`height` attributes via a
plain re-render. No JS, no per-cell DOM elements.
