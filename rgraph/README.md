# rgraph

Dioxus port of [`@xyflow/react`](https://github.com/xyflow/xyflow/tree/main/packages/react) (a.k.a. React Flow) built on top of [`rgraph-core`](../rgraph-core/) (framework-agnostic state-machines and types ported from `@xyflow/system`).

## Documentation

End-user docs live under [`doc/`](./doc/index.md):

- [Overview & quickstart](./doc/index.md)
- Concepts: [terms](./doc/concepts/terms-and-definitions.md), [building a flow](./doc/concepts/building-a-flow.md), [the viewport](./doc/concepts/the-viewport.md)
- Customization: [custom nodes](./doc/customization/custom-nodes.md), [custom edges](./doc/customization/custom-edges.md), [theming](./doc/customization/theming.md)
- Components: [`<RGraph>`](./doc/components/rgraph.md), [`<Background>`](./doc/components/background.md), [`<Controls>`](./doc/components/controls.md), [`<MiniMap>`](./doc/components/minimap.md), [`<NodeToolbar>`](./doc/components/node-toolbar.md), [`<EdgeToolbar>`](./doc/components/edge-toolbar.md), [`<NodeResizer>`](./doc/components/node-resizer.md)
- Hooks: [`use_rgraph()`](./doc/hooks/use-rgraph.md), [`use_nodes_state`](./doc/hooks/use-nodes-state.md), [`use_viewport`](./doc/hooks/use-viewport.md), [`use_store`](./doc/hooks/use-store.md)
- Advanced: [state management](./doc/advanced/state-management.md), [uncontrolled flow](./doc/advanced/uncontrolled-flow.md), [performance](./doc/advanced/performance.md)
- Recipes: [drag-and-drop sidebar](./doc/recipes/dragndrop.md), [auto-layout](./doc/recipes/layouting.md)

## Design choices

- **Reactivity**: Each store field is a `Signal<T>` injected through Dioxus' `use_context_provider`. Hooks read a single signal for granular re-renders. This is the Rust equivalent of Zustand's selector pattern.
- **DOM**: This crate targets Dioxus *desktop* (webview). DOM-only APIs (`ResizeObserver`, `getBoundingClientRect`) are bridged through `dioxus::desktop::use_eval` in the `dom/` module. Everything else uses Dioxus' native pointer/wheel/keyboard events.
- **State machines**: All non-DOM logic lives in `rgraph-core` (drag, pan/zoom, handle, resizer, minimap). This crate is responsible only for translating Dioxus events into core calls and rendering.
- **CSS**: Stylesheets are copied verbatim from `xyflow-react/src/styles/`. Class names (`react-flow__*`) are kept identical so the stylesheets work without modification.
