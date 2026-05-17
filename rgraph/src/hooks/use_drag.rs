//! Port of `xyflow-react/src/hooks/useDrag.ts`.
//!
//! Status: Phase 5 — implemented.
//!
//! ## What this hook does
//!
//! Returns a bundle ([`UseDragApi`]) the host wraps around its
//! `<NodeWrapper>` (or `<NodesSelection>`) element:
//!
//! * `dragging: Signal<bool>` — flips while a drag is in progress.
//! * `on_pointer_down`, `on_pointer_move`, `on_pointer_up`,
//!   `on_pointer_cancel` — `Callback<Event<PointerData>>` handlers
//!   the host wires to the corresponding HTML events.
//!
//! Internally the hook mounts an [`rgraph_core::xydrag::XYDrag`]
//! engine and routes events through [`crate::dom::pointer`]. The
//! engine snapshots store state every tick through a
//! [`rgraph_core::xydrag::GetStoreItemsFn`] closure built from the
//! current [`RGraphStore`].

#![allow(clippy::module_name_repetitions)]

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::events::PointerData;
use dioxus::prelude::*;

use rgraph_core::types::geometry::Rect;
use rgraph_core::types::nodes::PointerEventLike;
use rgraph_core::utils::store::adopt_user_nodes;
use rgraph_core::xydrag::{
    DragUpdateParams, GetStoreItemsFn, StoreSnapshot, XYDrag, XYDragParams,
};

use crate::context::use_rgraph_store;
use crate::dom::pointer::{from_dioxus as pointer_from_dioxus, PointerEventKind};
use crate::dom::PaneBounds;
use crate::store::RGraphStore;

/// Parameters accepted by [`use_drag`]. Mirrors the TS `UseDragParams`.
#[derive(Default, Clone)]
pub struct UseDragParams {
    pub disabled: bool,
    pub no_drag_class_name: Option<String>,
    pub handle_selector: Option<String>,
    /// Node id for a single-node drag, `None` for selection drag.
    pub node_id: Option<String>,
    pub is_selectable: bool,
    pub node_click_distance: Option<f64>,
}

/// API bundle returned by [`use_drag`]. The host wires the four
/// handlers to its element's pointer events.
#[derive(Clone, Copy)]
pub struct UseDragApi<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    pub dragging: Signal<bool>,
    pub on_pointer_down: Callback<Event<PointerData>>,
    pub on_pointer_move: Callback<Event<PointerData>>,
    pub on_pointer_up: Callback<Event<PointerData>>,
    pub on_pointer_cancel: Callback<Event<PointerData>>,
    #[allow(dead_code)]
    pub(crate) _types: std::marker::PhantomData<(N, E)>,
}/// Mount a drag engine for the calling component. Mirrors TS
/// `useDrag({ nodeRef, ... })` but returns Dioxus-native callbacks
/// instead of attaching listeners through `d3.select`.
///
/// The host must supply the pane bounds (so client→pane-local
/// coordinates can be computed). For Phase 5 we accept a
/// `Signal<PaneBounds>` argument — typically the one already created
/// by `<ZoomPane>` and threaded down through context. When the host
/// doesn't have access to it, an immutable default-zero bounds works
/// for hosts whose pane sits at the document origin.
pub fn use_drag<N, E>(
    params: UseDragParams,
    bounds_signal: Signal<PaneBounds>,
) -> UseDragApi<N, E>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let dragging = use_signal(|| false);

    // Construct the engine once. The engine holds an `Rc<XYDragInner>`
    // so we keep it in a hook-allocated `Rc<RefCell<Option<…>>>` and
    // initialise it on first render.
    let engine: Rc<RefCell<Option<XYDrag<N>>>> = use_hook(|| Rc::new(RefCell::new(None)));

    let params_clone = params.clone();
    let engine_for_init = engine.clone();
    use_hook(move || {
        let store_for_snapshot = store;
        let dragging_for_engine = dragging;
        let get_store_items: GetStoreItemsFn<N> = Rc::new(move || {
            // The snapshot captures every store slot the drag engine
            // reads. `pan_by` / `update_node_positions` /
            // `unselect_nodes_and_edges` are wrapped in `Rc`-backed
            // closures so the engine can call them across ticks.
            let lookup = store_for_snapshot.node_lookup.peek().clone();
            let node_extent = *store_for_snapshot.node_extent.peek();
            let snap_grid = *store_for_snapshot.snap_grid.peek();
            let snap_to_grid = *store_for_snapshot.snap_to_grid.peek();
            let node_origin = *store_for_snapshot.node_origin.peek();
            let multi = *store_for_snapshot.multi_selection_active.peek();
            let transform = *store_for_snapshot.transform.peek();
            let auto_pan = *store_for_snapshot.auto_pan_on_node_drag.peek();
            let draggable = *store_for_snapshot.nodes_draggable.peek();
            let select_on_drag = *store_for_snapshot.select_nodes_on_drag.peek();
            let drag_threshold = *store_for_snapshot.node_drag_threshold.peek();
            let auto_pan_speed = *store_for_snapshot.auto_pan_speed.peek();

            let store_clone = store_for_snapshot;
            StoreSnapshot::<N> {
                node_lookup: Rc::new(lookup),
                node_extent,
                snap_grid,
                snap_to_grid,
                node_origin,
                multi_selection_active: multi,
                transform,
                auto_pan_on_node_drag: auto_pan,
                nodes_draggable: draggable,
                select_nodes_on_drag: select_on_drag,
                node_drag_threshold: drag_threshold,
                auto_pan_speed,
                pan_by: Rc::new(move |delta| {
                    rgraph_core::Promise::resolved(store_clone.pan_by(delta))
                }),
                update_node_positions: Rc::new(move |drag_items, dragging| {
                    store_clone.update_node_positions(drag_items, dragging);
                }),
                unselect_nodes_and_edges: Rc::new(move || {
                    use crate::types::general::UnselectNodesAndEdgesParams as UP;
                    store_clone.unselect_nodes_and_edges(UP::<N, E> {
                        nodes: None,
                        edges: None,
                    });
                }),
                on_error: None,
            }
        });

        let dragging_start = dragging_for_engine;
        let dragging_stop = dragging_for_engine;
        let on_drag_start = Some({
            let d = dragging_start;
            Rc::new(move |_evt: &rgraph_core::types::nodes::PointerEventLike,
                          _items: &std::collections::HashMap<String, rgraph_core::types::nodes::NodeDragItem>,
                          _node: &Option<rgraph_core::types::nodes::Node<N>>,
                          _all: &[rgraph_core::types::nodes::Node<N>]| {
                use dioxus::prelude::WritableExt;
                d.clone().set(true);
            }) as rgraph_core::xydrag::OnDragFn<N>
        });
        let on_drag_stop = Some({
            let d = dragging_stop;
            Rc::new(move |_evt: &rgraph_core::types::nodes::PointerEventLike,
                          _items: &std::collections::HashMap<String, rgraph_core::types::nodes::NodeDragItem>,
                          _node: &Option<rgraph_core::types::nodes::Node<N>>,
                          _all: &[rgraph_core::types::nodes::Node<N>]| {
                use dioxus::prelude::WritableExt;
                d.clone().set(false);
            }) as rgraph_core::xydrag::OnDragFn<N>
        });

        let engine_instance = XYDrag::new(XYDragParams::<N> {
            get_store_items,
            on_drag_start,
            on_drag: None,
            on_drag_stop,
            on_node_drag_start: None,
            on_node_drag: None,
            on_node_drag_stop: None,
            on_selection_drag_start: None,
            on_selection_drag: None,
            on_selection_drag_stop: None,
            on_node_mouse_down: None,
        });

        // Apply the supplied update params (handle selector, click
        // distance, etc.).
        engine_instance.update(DragUpdateParams {
            no_drag_class_name: params_clone.no_drag_class_name.clone(),
            handle_selector: params_clone.handle_selector.clone(),
            is_selectable: params_clone.is_selectable,
            node_id: params_clone.node_id.clone(),
            node_click_distance: params_clone.node_click_distance.unwrap_or(0.0),
        });

        *engine_for_init.borrow_mut() = Some(engine_instance);
    });

    let on_pointer_down = build_pointer_callback::<N>(engine.clone(), bounds_signal, PointerEventKind::Down, params.disabled);
    let on_pointer_move = build_pointer_callback::<N>(engine.clone(), bounds_signal, PointerEventKind::Move, params.disabled);
    let on_pointer_up = build_pointer_callback::<N>(engine.clone(), bounds_signal, PointerEventKind::Up, params.disabled);
    let on_pointer_cancel = build_pointer_callback::<N>(engine, bounds_signal, PointerEventKind::Cancel, params.disabled);

    UseDragApi {
        dragging,
        on_pointer_down,
        on_pointer_move,
        on_pointer_up,
        on_pointer_cancel,
        _types: std::marker::PhantomData,
    }
}

fn build_pointer_callback<N: Clone + PartialEq + 'static>(
    engine: Rc<RefCell<Option<XYDrag<N>>>>,
    bounds_signal: Signal<PaneBounds>,
    kind: PointerEventKind,
    disabled: bool,
) -> Callback<Event<PointerData>> {
    Callback::new(move |evt: Event<PointerData>| {
        if disabled {
            return;
        }
        let Some(engine) = engine.borrow().clone() else { return };
        let bounds = *bounds_signal.read();
        let input = pointer_from_dioxus::<()>(&evt, kind, bounds, None);

        // Map the input variant to the matching engine method.
        use rgraph_zoom::PointerInput;
        match input {
            PointerInput::Down { id, x, y, button, ctrl, datum: _ } => {
                let event = PointerEventLike {
                    client_x: x,
                    client_y: y,
                    button,
                    buttons: 0,
                    ctrl_key: ctrl,
                    shift_key: false,
                    alt_key: false,
                    meta_key: false,
                };
                let _ = engine.handle_pointer_down(id, &event, button, ctrl);
            }
            PointerInput::Move { id, x, y } => {
                let event = PointerEventLike {
                    client_x: x,
                    client_y: y,
                    button: 0,
                    buttons: 0,
                    ctrl_key: false,
                    shift_key: false,
                    alt_key: false,
                    meta_key: false,
                };
                let _ = engine.handle_pointer_move(id, &event);
            }
            PointerInput::Up { id, x, y } => {
                let event = PointerEventLike {
                    client_x: x,
                    client_y: y,
                    button: 0,
                    buttons: 0,
                    ctrl_key: false,
                    shift_key: false,
                    alt_key: false,
                    meta_key: false,
                };
                let _ = engine.handle_pointer_up(id, &event);
            }
            PointerInput::Cancel { id } => {
                let event = PointerEventLike::default();
                let _ = engine.handle_pointer_cancel(id, &event);
            }
        }
    })
}

// `Rect` retained as an import for future expansions (auto-pan bbox).
#[allow(dead_code)]
type _R = Rect;
// `adopt_user_nodes` retained for documentation cross-references.
#[allow(dead_code)]
type _Adopt = fn();
#[allow(dead_code)]
fn _adopt_ref() {
    let _ = adopt_user_nodes::<()>;
}
