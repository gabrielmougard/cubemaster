//! Port of `xyflow-react/src/components/EdgeWrapper/index.tsx`.
//!
//! Status: Phase 6 — implemented.
//!
//! The wrapper:
//!   * Reads the edge from `edge_lookup`.
//!   * Resolves its source/target positions via
//!     [`rgraph_core::utils::edges::positions::get_edge_position`].
//!   * Resolves its z-index via
//!     [`rgraph_core::utils::edges::general::get_elevated_edge_z_index`].
//!   * Selects a built-in edge renderer based on `edge.type_`.
//!   * Wires click/double-click/mouse/keyboard handlers.
//!
//! Reconnect anchors (`<EdgeUpdateAnchors>`) and the custom-edge
//! registry are Phase-7 territory — see the TODOs inside.

#![allow(clippy::module_name_repetitions)]

pub mod update_anchors;
pub mod utils;

use dioxus::events::{KeyboardData, MouseData};
use dioxus::prelude::*;

use rgraph_core::types::geometry::Position;
use rgraph_core::utils::edges::general::{get_elevated_edge_z_index, GetEdgeZIndexParams};
use rgraph_core::utils::edges::positions::{get_edge_position, GetEdgePositionParams};
use rgraph_core::utils::marker::get_marker_id;

use crate::components::a11y_descriptions::ARIA_EDGE_DESC_KEY;
use crate::components::edge_wrapper::utils::BuiltInEdgeType;
use crate::components::edges::{
    BezierEdge, BezierEdgeComponentProps, SimpleBezierEdge, SimpleBezierEdgeComponentProps,
    SmoothStepEdge, SmoothStepEdgeComponentProps, StepEdge, StepEdgeComponentProps,
    StraightEdge, StraightEdgeComponentProps,
};
use crate::context::use_rgraph_store;
use crate::store::RGraphStore;
use crate::types::edges::{Edge, EdgeLabelOptions, EdgeMouseHandler, EdgeMouseHandlerArgs};

#[derive(Props, Clone, PartialEq)]
pub struct EdgeWrapperComponentProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    pub id: String,
    pub rf_id: String,
    #[props(default = "nopan".to_string())]
    pub no_pan_class_name: String,
    #[props(default)]
    pub disable_keyboard_a11y: bool,
    #[props(default)]
    pub on_click: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub on_double_click: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub on_mouse_enter: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub on_mouse_move: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub on_mouse_leave: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub on_context_menu: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn EdgeWrapper<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: EdgeWrapperComponentProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();

    // Resolve edge + the source/target nodes.
    let Some(edge) = store.edge_lookup.read().get(&props.id).cloned() else {
        return rsx! {};
    };
    let lookup = store.node_lookup.read();
    let Some(source) = lookup.get(&edge.source) else {
        return rsx! {};
    };
    let Some(target) = lookup.get(&edge.target) else {
        return rsx! {};
    };
    let connection_mode = *store.connection_mode.read();

    let edge_position = get_edge_position(GetEdgePositionParams::<N> {
        id: &props.id,
        source_node: source,
        target_node: target,
        source_handle: edge.source_handle.as_deref(),
        target_handle: edge.target_handle.as_deref(),
        connection_mode,
        on_error: None,
    });
    let Some(edge_position) = edge_position else {
        return rsx! {};
    };

    let z = get_elevated_edge_z_index(GetEdgeZIndexParams::<N> {
        source_node: source,
        target_node: target,
        selected: edge.selected.unwrap_or(false),
        z_index: edge.z_index.map(f64::from).unwrap_or(0.0),
        elevate_on_select: *store.elevate_edges_on_select.read(),
        z_index_mode: *store.z_index_mode.read(),
    });
    drop(lookup);

    if edge.hidden.unwrap_or(false) {
        return rsx! {};
    }

    let edges_focusable = *store.edges_focusable.read();
    let elements_selectable = *store.elements_selectable.read();
    let is_focusable = edges_focusable;
    let is_selectable = edge.selectable.unwrap_or(elements_selectable);

    let edge_type_raw = edge.type_.clone().unwrap_or_else(|| "default".to_string());
    let resolved = BuiltInEdgeType::parse(&edge_type_raw).unwrap_or(BuiltInEdgeType::Default);

    // Marker URLs.
    let marker_start_url = if edge.marker_start.is_some() {
        let id = get_marker_id(edge.marker_start.as_ref(), Some(&props.rf_id));
        if id.is_empty() { None } else { Some(format!("url('#{id}')")) }
    } else {
        None
    };
    let marker_end_url = if edge.marker_end.is_some() {
        let id = get_marker_id(edge.marker_end.as_ref(), Some(&props.rf_id));
        if id.is_empty() { None } else { Some(format!("url('#{id}')")) }
    } else {
        None
    };

    // Class composition.
    let mut classes = String::from("react-flow__edge");
    classes.push(' ');
    classes.push_str("react-flow__edge-");
    classes.push_str(resolved.as_str());
    classes.push(' ');
    classes.push_str(&props.no_pan_class_name);
    if edge.selected.unwrap_or(false) {
        classes.push_str(" selected");
    }
    if edge.animated.unwrap_or(false) {
        classes.push_str(" animated");
    }
    if !is_selectable && props.on_click.is_none() {
        classes.push_str(" inactive");
    }
    if is_selectable {
        classes.push_str(" selectable");
    }

    let svg_style = format!("z-index: {z};");
    let test_id = format!("rf__edge-{}", &props.id);
    let described_by = if is_focusable {
        format!("{ARIA_EDGE_DESC_KEY}-{}", props.rf_id)
    } else {
        String::new()
    };
    let aria_label = edge
        .aria_label
        .clone()
        .unwrap_or_else(|| format!("Edge from {} to {}", edge.source, edge.target));
    let role = if is_focusable { "group" } else { "img" };

    let id_attr = props.id.clone();

    // Handler wrappers.
    let edge_for_handlers = edge.clone();
    let on_click_outer = {
        let id = props.id.clone();
        let user_on_click = props.on_click;
        let edge_clone = edge_for_handlers.clone();
        move |evt: Event<MouseData>| {
            use dioxus::prelude::WritableExt;
            if is_selectable {
                store.nodes_selection_active.clone().set(false);
                let multi = *store.multi_selection_active.peek();
                if edge_clone.selected.unwrap_or(false) && multi {
                    store.unselect_nodes_and_edges(
                        crate::types::general::UnselectNodesAndEdgesParams {
                            nodes: Some(Vec::new()),
                            edges: Some(vec![edge_clone.clone()]),
                        },
                    );
                } else {
                    store.add_selected_edges(vec![id.clone()]);
                }
            }
            if let Some(cb) = user_on_click {
                cb.call(EdgeMouseHandlerArgs {
                    event: std::rc::Rc::new(evt),
                    edge: edge_clone.clone(),
                });
            }
        }
    };

    let on_double_click_outer = edge_mouse_forwarder(&props.on_double_click, &edge_for_handlers);
    let on_context_menu_outer = edge_mouse_forwarder(&props.on_context_menu, &edge_for_handlers);
    let on_mouse_enter_outer = edge_mouse_forwarder(&props.on_mouse_enter, &edge_for_handlers);
    let on_mouse_move_outer = edge_mouse_forwarder(&props.on_mouse_move, &edge_for_handlers);
    let on_mouse_leave_outer = edge_mouse_forwarder(&props.on_mouse_leave, &edge_for_handlers);

    let on_key_down = {
        let id = props.id.clone();
        let edge_clone = edge_for_handlers.clone();
        let disable_a11y = props.disable_keyboard_a11y;
        move |evt: Event<KeyboardData>| {
            if disable_a11y {
                return;
            }
            let key = evt.key().to_string();
            const SELECT_KEYS: &[&str] = &["Enter", " ", "Escape"];
            if SELECT_KEYS.contains(&key.as_str()) && is_selectable {
                let unselect = key == "Escape";
                if unselect {
                    store.unselect_nodes_and_edges(
                        crate::types::general::UnselectNodesAndEdgesParams {
                            nodes: Some(Vec::new()),
                            edges: Some(vec![edge_clone.clone()]),
                        },
                    );
                } else {
                    store.add_selected_edges(vec![id.clone()]);
                }
            }
        }
    };

    let label_options = edge_label_options_from(&edge);
    let style_str = String::new(); // TS reads `edge.style` (CSS string); our `Edge` doesn't carry it directly — Phase 7 wires `EdgePresentation`.

    // Render the body. Bind values to local clones so each branch can
    // move them freely.
    let body: Element = match resolved {
        BuiltInEdgeType::Default => rsx! {
            BezierEdge {
                id: Some(id_attr.clone()),
                source_x: edge_position.source_x,
                source_y: edge_position.source_y,
                target_x: edge_position.target_x,
                target_y: edge_position.target_y,
                source_position: edge_position.source_position,
                target_position: edge_position.target_position,
                path_options: None,
                label_options: label_options.clone(),
                style: Some(style_str.clone()),
                marker_start: marker_start_url.clone(),
                marker_end: marker_end_url.clone(),
                interaction_width: edge.interaction_width,
            }
        },
        BuiltInEdgeType::Straight => rsx! {
            StraightEdge {
                id: Some(id_attr.clone()),
                source_x: edge_position.source_x,
                source_y: edge_position.source_y,
                target_x: edge_position.target_x,
                target_y: edge_position.target_y,
                label_options: label_options.clone(),
                style: Some(style_str.clone()),
                marker_start: marker_start_url.clone(),
                marker_end: marker_end_url.clone(),
                interaction_width: edge.interaction_width,
            }
        },
        BuiltInEdgeType::Step => rsx! {
            StepEdge {
                id: Some(id_attr.clone()),
                source_x: edge_position.source_x,
                source_y: edge_position.source_y,
                target_x: edge_position.target_x,
                target_y: edge_position.target_y,
                source_position: edge_position.source_position,
                target_position: edge_position.target_position,
                path_options: None,
                label_options: label_options.clone(),
                style: Some(style_str.clone()),
                marker_start: marker_start_url.clone(),
                marker_end: marker_end_url.clone(),
                interaction_width: edge.interaction_width,
            }
        },
        BuiltInEdgeType::SmoothStep => rsx! {
            SmoothStepEdge {
                id: Some(id_attr.clone()),
                source_x: edge_position.source_x,
                source_y: edge_position.source_y,
                target_x: edge_position.target_x,
                target_y: edge_position.target_y,
                source_position: edge_position.source_position,
                target_position: edge_position.target_position,
                path_options: None,
                label_options: label_options.clone(),
                style: Some(style_str.clone()),
                marker_start: marker_start_url.clone(),
                marker_end: marker_end_url.clone(),
                interaction_width: edge.interaction_width,
            }
        },
        BuiltInEdgeType::SimpleBezier => rsx! {
            SimpleBezierEdge {
                id: Some(id_attr.clone()),
                source_x: edge_position.source_x,
                source_y: edge_position.source_y,
                target_x: edge_position.target_x,
                target_y: edge_position.target_y,
                source_position: edge_position.source_position,
                target_position: edge_position.target_position,
                label_options: label_options.clone(),
                style: Some(style_str.clone()),
                marker_start: marker_start_url.clone(),
                marker_end: marker_end_url.clone(),
                interaction_width: edge.interaction_width,
            }
        },
    };

    let tab_idx = if is_focusable { Some(0i64) } else { None };
    if let Some(idx) = tab_idx {
        rsx! {
            svg {
                style: "{svg_style}",
                g {
                    class: "{classes}",
                    "data-id": "{id_attr}",
                    "data-testid": "{test_id}",
                    "aria-roledescription": "edge",
                    "aria-describedby": "{described_by}",
                    "aria-label": "{aria_label}",
                    role: "{role}",
                    tabindex: "{idx}",
                    onclick: on_click_outer,
                    ondoubleclick: on_double_click_outer,
                    oncontextmenu: on_context_menu_outer,
                    onmouseenter: on_mouse_enter_outer,
                    onmousemove: on_mouse_move_outer,
                    onmouseleave: on_mouse_leave_outer,
                    onkeydown: on_key_down,
                    {body}
                }
            }
        }
    } else {
        rsx! {
            svg {
                style: "{svg_style}",
                g {
                    class: "{classes}",
                    "data-id": "{id_attr}",
                    "data-testid": "{test_id}",
                    "aria-roledescription": "edge",
                    "aria-label": "{aria_label}",
                    role: "{role}",
                    onclick: on_click_outer,
                    ondoubleclick: on_double_click_outer,
                    oncontextmenu: on_context_menu_outer,
                    onmouseenter: on_mouse_enter_outer,
                    onmousemove: on_mouse_move_outer,
                    onmouseleave: on_mouse_leave_outer,
                    {body}
                }
            }
        }
    }
}

fn edge_mouse_forwarder<E: Clone + PartialEq + 'static>(
    handler: &Option<EdgeMouseHandler<E>>,
    edge: &Edge<E>,
) -> impl FnMut(Event<MouseData>) + 'static {
    let handler = *handler;
    let edge = edge.clone();
    move |evt: Event<MouseData>| {
        if let Some(cb) = handler {
            cb.call(EdgeMouseHandlerArgs {
                event: std::rc::Rc::new(evt),
                edge: edge.clone(),
            });
        }
    }
}

fn edge_label_options_from<E: Clone + PartialEq + 'static>(_edge: &Edge<E>) -> EdgeLabelOptions {
    // The TS source pulls `edge.label`/`labelStyle`/… directly off the
    // edge. Our `Edge<E>` doesn't carry those fields (they live on the
    // `EdgePresentation` slice; Phase 7 plumbs it). For Phase 6 we
    // return an empty label-options bag — built-in edges still render
    // their paths and markers correctly without a label.
    EdgeLabelOptions::default()
}

// Position import kept for downstream callers.
#[allow(dead_code)]
type _P = Position;
