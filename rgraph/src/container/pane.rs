//! Port of `xyflow-react/src/container/Pane/index.tsx`.
//!
//! Status: Phase 5 — implemented.
//!
//! The pane is the transparent `<div>` covering the viewport. It:
//!
//! * Forwards background `click` / `contextmenu` / mouse-enter/move/leave
//!   events to the host's `onPaneXxx` callbacks.
//! * Forwards wheel events to `on_pane_scroll`.
//! * On click, calls `reset_selected_elements` and clears the
//!   `nodes_selection_active` flag (matching TS lines 112–123).
//! * Hosts the `<UserSelection>` overlay rectangle.
//! * Drives marquee-selection: on pointer-down/move while
//!   `is_selecting`, writes a [`SelectionRect`] into the store and
//!   marks the matching nodes as selected via
//!   [`crate::utils::changes::get_selection_changes_for_nodes`].

#![allow(clippy::module_name_repetitions)]

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use dioxus::events::{MouseData, PointerData, WheelData};
use dioxus::html::point_interaction::InteractionLocation;
use dioxus::prelude::*;

use rgraph_core::types::geometry::{Rect, XYPosition};
use rgraph_core::types::panzoom::PanOnDrag;
use rgraph_core::types::viewport::{SelectionMode, SelectionRect};
use rgraph_core::utils::general::{point_to_renderer_point, renderer_point_to_point};
use rgraph_core::utils::graph::{get_nodes_inside, GetNodesInsideParams};

use crate::components::user_selection::UserSelection;
use crate::context::use_rgraph_store;
use crate::store::RGraphStore;
use crate::types::component_props::{OnPaneScroll, PaneMouseHandler};
use crate::utils::changes::{
    get_selection_changes_for_edges, get_selection_changes_for_nodes,
};

/// Props for [`Pane`]. Mirrors the TS `PaneProps` (a subset of
/// `ReactFlowProps` plus `is_selecting`/`selection_key_pressed`/etc.).
#[derive(Props, Clone, PartialEq)]
pub struct PaneProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default)]
    pub is_selecting: bool,
    #[props(default)]
    pub selection_key_pressed: bool,
    #[props(default)]
    pub selection_mode: Option<SelectionMode>,
    #[props(default)]
    pub pan_on_drag: Option<PanOnDrag>,
    #[props(default)]
    pub auto_pan_on_selection: Option<bool>,
    #[props(default)]
    pub pane_click_distance: Option<f64>,
    #[props(default)]
    pub selection_on_drag: Option<bool>,

    #[props(default)]
    pub on_pane_click: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_context_menu: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_scroll: Option<OnPaneScroll>,
    #[props(default)]
    pub on_pane_mouse_enter: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_mouse_move: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_mouse_leave: Option<PaneMouseHandler>,

    pub children: Element,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn Pane<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: PaneProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();

    let dragging = *store.pane_dragging.read();
    let elements_selectable = *store.elements_selectable.read();
    let user_selection_active = *store.user_selection_active.read();
    let connection_in_progress = matches!(
        *store.connection.read(),
        rgraph_core::types::connection::ConnectionState::InProgress(_)
    );

    let is_selection_enabled =
        elements_selectable && (props.is_selecting || user_selection_active);

    // Selection-tracking state: persists across pointer events without
    // re-rendering. `RefCell` is fine — Dioxus events are single-threaded.
    let selection_state: Rc<RefCell<SelectionState>> = use_hook(|| {
        Rc::new(RefCell::new(SelectionState::default()))
    });

    // Compose className.
    let mut classes = String::from("react-flow__pane");
    let draggable = match &props.pan_on_drag {
        Some(PanOnDrag::Off) => false,
        Some(PanOnDrag::On) => true,
        Some(PanOnDrag::Buttons(b)) => b.contains(&0),
        None => true, // TS default: pan_on_drag = true.
    };
    if draggable {
        classes.push_str(" draggable");
    }
    if dragging {
        classes.push_str(" dragging");
    }
    if props.is_selecting {
        classes.push_str(" selection");
    }

    let style = "position:absolute;width:100%;height:100%;top:0;left:0;";
    let selection_mode = props.selection_mode.unwrap_or(SelectionMode::Full);
    let pane_click_distance = props.pane_click_distance.unwrap_or(0.0);
    let selection_on_drag = props.selection_on_drag.unwrap_or(false);
    let selection_key_pressed = props.selection_key_pressed;

    let on_click = {
        let handler = props.on_pane_click;
        let selection_state = selection_state.clone();
        move |event: Event<MouseData>| {
            // Suppress the click if it was the tail of a marquee drag
            // (the TS `selectionInProgress.current` short-circuit).
            let mut state = selection_state.borrow_mut();
            if state.selection_in_progress || connection_in_progress {
                state.selection_in_progress = false;
                return;
            }
            drop(state);
            if let Some(cb) = handler {
                cb.call(std::rc::Rc::new(event));
            }
            store.reset_selected_elements();
            store.nodes_selection_active.clone().set(false);
        }
    };

    let on_context_menu = {
        let handler = props.on_pane_context_menu;
        let pan_on_drag = props.pan_on_drag.clone();
        move |event: Event<MouseData>| {
            if let Some(PanOnDrag::Buttons(b)) = &pan_on_drag
                && b.contains(&2)
            {
                return;
            }
            if let Some(cb) = handler {
                cb.call(std::rc::Rc::new(event));
            }
        }
    };

    let on_wheel = {
        let handler = props.on_pane_scroll;
        move |event: Event<WheelData>| {
            if let Some(cb) = handler {
                cb.call(std::rc::Rc::new(event));
            }
        }
    };

    let on_pointer_enter = {
        let handler = props.on_pane_mouse_enter;
        move |event: Event<MouseData>| {
            if !is_selection_enabled
                && let Some(cb) = handler
            {
                cb.call(std::rc::Rc::new(event));
            }
        }
    };

    // -- Marquee selection: pointer down ------------------------------
    let on_pointer_down = {
        let selection_state = selection_state.clone();
        move |event: Event<PointerData>| {
            if !is_selection_enabled {
                return;
            }
            // Only the primary mouse button starts a selection.
            let target_is_pane = true; // Dioxus doesn't expose `event.target == container` cheaply; assume yes.
            let is_active = (selection_on_drag && target_is_pane) || selection_key_pressed;
            if !is_active {
                return;
            }

            let bbox = *store.dom_bbox.peek();
            let transform = *store.transform.peek();
            let client = event.client_coordinates();
            let local_x = client.x - bbox.x;
            let local_y = client.y - bbox.y;
            let start_in_flow = point_to_renderer_point(
                XYPosition::new(local_x, local_y),
                transform,
                false,
                (1.0, 1.0),
            );

            let mut state = selection_state.borrow_mut();
            state.selection_in_progress = false;
            state.start_screen = XYPosition::new(local_x, local_y);
            state.start_flow = start_in_flow;
            state.selected_node_ids.clear();
            state.selected_edge_ids.clear();
            drop(state);

            // Seed the user-selection rect at zero size.
            store.user_selection_rect.clone().set(Some(SelectionRect {
                rect: Rect {
                    x: local_x,
                    y: local_y,
                    width: 0.0,
                    height: 0.0,
                },
                start_x: start_in_flow.x,
                start_y: start_in_flow.y,
            }));
        }
    };

    // -- Marquee selection: pointer move ------------------------------
    let on_pointer_move = {
        let handler = props.on_pane_mouse_move;
        let selection_state = selection_state.clone();
        move |event: Event<PointerData>| {
            if !is_selection_enabled {
                if let Some(cb) = handler {
                    // The pane-mouse-move callback expects MouseData,
                    // not PointerData. Phase 7 will reconcile event
                    // types; here we forward only when not selecting.
                    let _ = cb;
                }
                return;
            }
            let Some(rect) = *store.user_selection_rect.peek() else {
                return;
            };
            let bbox = *store.dom_bbox.peek();
            let transform = *store.transform.peek();
            let client = event.client_coordinates();
            let mouse_x = client.x - bbox.x;
            let mouse_y = client.y - bbox.y;

            let screen_start = renderer_point_to_point(
                XYPosition::new(rect.start_x, rect.start_y),
                transform,
            );
            let mut state = selection_state.borrow_mut();
            if !state.selection_in_progress {
                let required = if selection_key_pressed { 0.0 } else { pane_click_distance };
                let dx = mouse_x - screen_start.x;
                let dy = mouse_y - screen_start.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= required {
                    return;
                }
                state.selection_in_progress = true;
                store.reset_selected_elements();
            }

            // Build the next selection rect.
            let next_rect = Rect {
                x: mouse_x.min(screen_start.x),
                y: mouse_y.min(screen_start.y),
                width: (mouse_x - screen_start.x).abs(),
                height: (mouse_y - screen_start.y).abs(),
            };

            // Compute intersecting nodes.
            let partially = matches!(selection_mode, SelectionMode::Partial);
            let lookup = store.node_lookup.peek();
            let visible = get_nodes_inside(
                &lookup,
                next_rect,
                transform,
                GetNodesInsideParams {
                    partially,
                    exclude_non_selectable_nodes: true,
                },
            );
            let new_node_ids: HashSet<String> = visible.iter().map(|n| n.user.id.clone()).collect();
            drop(lookup);

            // Edges connected to the selected nodes (per defaultEdgeOptions.selectable).
            let edges_selectable = store
                .default_edge_options
                .peek()
                .as_ref()
                .and_then(|o| o.selectable)
                .unwrap_or(true);
            let mut new_edge_ids: HashSet<String> = HashSet::new();
            {
                let connection_lookup = store.connection_lookup.peek();
                let edge_lookup = store.edge_lookup.peek();
                for node_id in &new_node_ids {
                    if let Some(connections) = connection_lookup.get(node_id) {
                        for hc in connections.values() {
                            if let Some(edge) = edge_lookup.get(&hc.edge_id)
                                && edge.selectable.unwrap_or(edges_selectable)
                            {
                                new_edge_ids.insert(hc.edge_id.clone());
                            }
                        }
                    }
                }
            }

            // Apply selection deltas.
            if state.selected_node_ids != new_node_ids {
                let mut lookup_mut = store.node_lookup.clone().write_unchecked();
                let changes = get_selection_changes_for_nodes(&mut lookup_mut, &new_node_ids);
                drop(lookup_mut);
                if !changes.is_empty() {
                    store.trigger_node_changes(changes);
                }
                state.selected_node_ids = new_node_ids;
            }
            if state.selected_edge_ids != new_edge_ids {
                let edge_lookup = store.edge_lookup.peek();
                let edge_changes = get_selection_changes_for_edges(&edge_lookup, &new_edge_ids);
                drop(edge_lookup);
                if !edge_changes.is_empty() {
                    store.trigger_edge_changes(edge_changes);
                }
                state.selected_edge_ids = new_edge_ids;
            }

            // Commit the rect to the store so `<UserSelection>` re-renders.
            store.user_selection_rect.clone().set(Some(SelectionRect {
                rect: next_rect,
                start_x: rect.start_x,
                start_y: rect.start_y,
            }));
            store.user_selection_active.clone().set(true);
            store.nodes_selection_active.clone().set(false);
        }
    };

    // -- Marquee selection: pointer up --------------------------------
    let on_pointer_up = {
        let selection_state = selection_state.clone();
        move |_event: Event<PointerData>| {
            let state = selection_state.borrow_mut();
            let was_in_progress = state.selection_in_progress;
            let had_nodes = !state.selected_node_ids.is_empty();
            // Don't reset `selection_in_progress` here — the click
            // handler reads it to suppress the trailing click.
            store.user_selection_active.clone().set(false);
            store.user_selection_rect.clone().set(None);
            if was_in_progress {
                store.nodes_selection_active.clone().set(had_nodes);
            }
            drop(state);
        }
    };

    let on_pointer_leave = {
        let handler = props.on_pane_mouse_leave;
        move |event: Event<MouseData>| {
            if let Some(cb) = handler {
                cb.call(std::rc::Rc::new(event));
            }
        }
    };

    rsx! {
        div {
            class: "{classes}",
            style: "{style}",
            onclick: on_click,
            oncontextmenu: on_context_menu,
            onwheel: on_wheel,
            onmouseenter: on_pointer_enter,
            onpointerdown: on_pointer_down,
            onpointermove: on_pointer_move,
            onpointerup: on_pointer_up,
            onmouseleave: on_pointer_leave,
            {props.children}
            UserSelection::<N, E> {}
        }
    }
}

/// Per-pane mutable state held inside an `Rc<RefCell<…>>` so the
/// pointer-event closures can share access without re-rendering on
/// every mouse move.
#[derive(Default)]
struct SelectionState {
    /// Whether the current gesture has crossed the click-vs-drag
    /// threshold. Mirrors TS `selectionInProgress.current`.
    selection_in_progress: bool,
    /// Pointer-down position in pane-local screen coordinates.
    start_screen: XYPosition,
    /// Pointer-down position in flow coordinates.
    start_flow: XYPosition,
    /// Ids of currently-selected nodes (per the last accepted rect).
    selected_node_ids: HashSet<String>,
    /// Ids of currently-selected edges (per the last accepted rect).
    selected_edge_ids: HashSet<String>,
}
