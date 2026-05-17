# rgraph

Dioxus port of [`@xyflow/react`](https://github.com/xyflow/xyflow/tree/main/packages/react) (a.k.a. React Flow) built on top of [`rgraph-core`](../rgraph-core/) (framework-agnostic state-machines and types ported from `@xyflow/system`).

> Status: **Phase 0 — skeleton**. Every file in `src/` is a stub with TODO markers pointing back to the TypeScript reference under `../xyflow-react/src/`.

## Roadmap

See the workspace plan for the full porting roadmap. Phases:

- **Phase 0** [DONE]: Crate skeleton & stubs.
- **Phase 1** [DONE]: Pure data types & `utils/changes` (`types/*`, `utils/changes`, `utils/general`).
- **Phase 2** [DONE]: Store + `RGraphProvider` (Dioxus context).
- **Phase 3** [DONE]: Core hooks (no rendering yet).
- **Phase 4** [DONE]: Viewport rendering (ZoomPane, Pane, Panel).
- **Phase 5** [DONE]: Nodes (NodeRenderer, NodeWrapper, default node types, drag).
- **Phase 6** [DONE]: Edges + Handles + Connections + EdgeRenderer.

- **Phase 7** [DONE]: Top-level `RGraph` component -> first companion integration.
- **Phase 8** [TODO]: Additional components (Background, Controls, MiniMap, NodeToolbar, EdgeToolbar, NodeResizer).

## Module map (mirrors `xyflow-react/src/`)

| xyflow-react path                          | rgraph path                                 |
|--------------------------------------------|---------------------------------------------|
| `container/ReactFlow/index.tsx`            | `container/rgraph/mod.rs` (`RGraph`)        |
| `container/GraphView/index.tsx`            | `container/graph_view/mod.rs`               |
| `container/FlowRenderer/index.tsx`         | `container/flow_renderer.rs`                |
| `container/NodeRenderer/index.tsx`         | `container/node_renderer/mod.rs`            |
| `container/EdgeRenderer/index.tsx`         | `container/edge_renderer/mod.rs`            |
| `container/Pane/index.tsx`                 | `container/pane.rs`                         |
| `container/ZoomPane/index.tsx`             | `container/zoom_pane.rs`                    |
| `container/Viewport/index.tsx`             | `container/viewport.rs`                     |
| `components/*`                             | `components/*`                              |
| `hooks/use*.ts`                            | `hooks/use_*.rs`                            |
| `store/{index,initialState}.ts`            | `store/{mod,initial_state,actions}.rs`      |
| `types/*.ts`                               | `types/*.rs`                                |
| `contexts/*.ts`                            | `contexts/*.rs` + `context.rs`              |
| `utils/{changes,general}.ts`               | `utils/{changes,general}.rs`                |
| `additional-components/*`                  | `additional_components/*` (phase 2)         |
| `styles/{base,style}.css`                  | `assets/{base,style}.css` (verbatim copies) |

## Design choices

- **Reactivity**: Each store field is a `Signal<T>` injected through Dioxus' `use_context_provider`. Hooks read a single signal for granular re-renders — the Rust equivalent of Zustand's selector pattern.
- **DOM**: This crate targets Dioxus *desktop* (webview). DOM-only APIs (`ResizeObserver`, `getBoundingClientRect`) are bridged through `dioxus::desktop::use_eval` in the `dom/` module. Everything else uses Dioxus' native pointer/wheel/keyboard events.
- **State machines**: All non-DOM logic lives in `rgraph-core` (drag, pan/zoom, handle, resizer, minimap). This crate is responsible only for translating Dioxus events into core calls and rendering.
- **CSS**: Stylesheets are copied verbatim from `xyflow-react/src/styles/`. Class names (`react-flow__*`) are kept identical so the stylesheets work without modification.
