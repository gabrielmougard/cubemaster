# Theming

`rgraph` re-uses the upstream xyflow CSS verbatim. The stylesheet
exposes around ~30 CSS variables you can override to recolor every
visual element without touching the components.

## Including the stylesheet

The CSS is bundled as a `&'static str` constant:

```rust
use dioxus::prelude::*;
use rgraph::styles::BASE_CSS;

#[component]
fn AppShell() -> Element {
    rsx! {
        style { "{BASE_CSS}" }
        App {}
    }
}
```

Alternatively serve it as a `Stylesheet` document head item or inline
it via Dioxus' asset pipeline — either path works because the CSS has
no inter-rule order dependencies.

## Built-in theme switching

The CSS file ships `light` and `dark` overrides under
`html[data-rgraph-theme="dark"]`. Toggle the attribute on `<html>` to
switch:

```rust
use_effect(move || {
    let theme = if dark { "dark" } else { "light" };
    let _ = dioxus::desktop::eval(&format!(
        "document.documentElement.dataset.rgraphTheme = '{theme}'"
    ));
});
```

Or use the [`use_color_mode_class`](../../src/hooks/use_color_mode_class.rs)
hook which writes the appropriate class on the `<RGraph>` wrapper.

## CSS variables

| Variable                                            | Default              | Affects                          |
|-----------------------------------------------------|----------------------|----------------------------------|
| `--xy-background-color`                             | `#fafafa`            | Pane background.                 |
| `--xy-background-pattern-color`                     | `#cccccc`            | Dot/line/cross pattern stroke.   |
| `--xy-node-background-color-default`                | `#fff`               | Built-in node background.        |
| `--xy-node-border-default`                          | `1px solid #1a192b`  | Built-in node border.            |
| `--xy-node-color-default`                           | `#222`               | Built-in node text color.        |
| `--xy-node-boxshadow-default`                       | `…`                  | Node drop shadow.                |
| `--xy-node-border-radius-default`                   | `3px`                | Node rounding.                   |
| `--xy-handle-background-color`                      | `#1a192b`            | Handle fill.                     |
| `--xy-handle-border-color`                          | `#fff`               | Handle outline.                  |
| `--xy-edge-stroke`                                  | `#b1b1b7`            | Edge color.                      |
| `--xy-edge-stroke-selected`                         | `#555`               | Selected edge color.             |
| `--xy-edge-stroke-width`                            | `1`                  | Edge thickness.                  |
| `--xy-connection-line-stroke`                       | `#b1b1b7`            | In-progress connection.          |
| `--xy-controls-button-background-color`             | `#fff`               | `<Controls>` button fill.        |
| `--xy-minimap-background-color`                     | `#fff`               | Minimap pane fill.               |
| `--xy-minimap-mask-background-color`                | `rgba(240,240,240,.6)` | Minimap viewport mask.         |
| `--xy-minimap-node-background-color`                | `#e2e2e2`            | Minimap node fill.               |
| `--xy-attribution-background-color`                 | `rgba(255,255,255,.5)` | Attribution chip background.   |

For the full list, search the CSS for `--xy-` — every default is on
the `:root` selector.

## Per-instance overrides

Each helper component accepts a `bg_color` / `color` / `mask_color` /
etc. prop that emits a scoped CSS custom property override. This is
the preferred pattern when you want a single component to deviate
from the page-wide theme:

```rust
Background::<BuiltInNodeData, ()> {
    bg_color:    Some("#0f172a".into()),
    color:       Some("#334155".into()),
    variant:     BackgroundVariant::Dots,
}

MiniMap::<BuiltInNodeData> {
    bg_color:    Some("#0f172a".into()),
    mask_color:  Some("rgba(255,255,255,0.1)".into()),
}
```

The implementation writes
`--xy-background-color-props: …` on the wrapping `<Panel>` style.

## Custom classes

Every helper takes a `class_name: Option<String>` that's appended after
the default class. Combine with your own CSS:

```rust
Controls::<BuiltInNodeData, ()> {
    class_name: Some("my-controls".into()),
}
```

```css
.my-controls .react-flow__controls-button {
    background: #1e293b;
    color: #f8fafc;
}
```

## Dark mode preset

A minimal dark-mode override snippet:

```css
[data-rgraph-theme="dark"] {
    --xy-background-color:           #0f172a;
    --xy-background-pattern-color:   #334155;
    --xy-node-background-color-default: #1e293b;
    --xy-node-color-default:         #f8fafc;
    --xy-node-border-default:        1px solid #475569;
    --xy-edge-stroke:                #94a3b8;
    --xy-edge-stroke-selected:       #e2e8f0;
    --xy-handle-background-color:    #f8fafc;
    --xy-handle-border-color:        #0f172a;
    --xy-controls-button-background-color: #1e293b;
}
```
