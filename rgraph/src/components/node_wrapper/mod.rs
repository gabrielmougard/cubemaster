//! Port of `xyflow-react/src/components/NodeWrapper/`.
//!
//! Status: Phase 5 — implemented.

#![allow(clippy::module_name_repetitions)]

pub mod use_node_observer;
pub mod utils;

use dioxus::events::{KeyboardData, MouseData, PointerData};
use dioxus::html::input_data::keyboard_types::Modifiers;
use dioxus::html::point_interaction::ModifiersInteraction;
use dioxus::prelude::*;

use rgraph_core::utils::general::{get_node_dimensions, node_has_dimensions};

use crate::components::a11y_descriptions::ARIA_NODE_DESC_KEY;
use crate::components::node_wrapper::use_node_observer::use_node_observer;
use crate::components::node_wrapper::utils::{
    arrow_key_diff, get_node_inline_style_dimensions, BuiltInNodeType,
};
use crate::components::nodes::default_node::DefaultNode;
use crate::components::nodes::group_node::GroupNode;
use crate::components::nodes::input_node::InputNode;
use crate::components::nodes::output_node::OutputNode;
use crate::components::nodes::utils::{handle_node_click, HandleNodeClickArgs};
use crate::context::use_rgraph_store;
use crate::contexts::node_id::provide_node_id;
use crate::hooks::use_drag::{use_drag, UseDragParams};
use crate::hooks::use_move_selected_nodes::{use_move_selected_nodes, MoveSelectedNodesParams};
use crate::store::RGraphStore;
use crate::types::nodes::{
    BuiltInNodeData, Node, NodeMouseHandler, NodeMouseHandlerArgs, NodeProps,
};

/// Props for [`NodeWrapper`]. Phase 5 covers the built-in
/// `BuiltInNodeData` data type only — custom node types (`nodeTypes`
/// prop) require a more elaborate component-registry machinery that
/// lands in Phase 7 once the full `<RGraph>` component is in place.
#[derive(Props, Clone, PartialEq)]
pub struct NodeWrapperProps<N: Clone + PartialEq + 'static = BuiltInNodeData> {
    /// Node id. Matches the `data-id` attribute on the wrapper div.
    pub id: String,

    /// Effective `noDragClassName` — when present, descendants with
    /// this class suppress drag start.
    #[props(default = "nodrag".to_string())]
    pub no_drag_class_name: String,
    /// Effective `noPanClassName` — when present on the wrapper,
    /// suppresses pan-on-drag.
    #[props(default = "nopan".to_string())]
    pub no_pan_class_name: String,

    /// React Flow id (used to address the aria-description div).
    pub rf_id: String,

    /// Keyboard-a11y is opt-out, defaulting to enabled (TS parity).
    #[props(default)]
    pub disable_keyboard_a11y: bool,

    // Per-event handler overrides — usually plumbed from `<RGraph>` props.
    #[props(default)]
    pub on_click: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_double_click: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_mouse_enter: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_mouse_move: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_mouse_leave: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_context_menu: Option<NodeMouseHandler<N>>,

    /// Distance (in px) the cursor can move between mousedown/up that
    /// still counts as a click. Forwarded to the drag engine.
    #[props(default = 0.0)]
    pub node_click_distance: f64,

    #[props(default)]
    pub _types: std::marker::PhantomData<N>,
}

/// `<NodeWrapper>` Dioxus component.
///
/// Mirrors `xyflow-react/src/components/NodeWrapper/index.tsx`. The
/// component is **not** generic over `<E>` because the wrapper does
/// not touch the edge data type; only the node data shape matters.
#[component]
pub fn NodeWrapper<N: Clone + PartialEq + 'static>(
    props: NodeWrapperProps<N>,
) -> Element {
    let store: RGraphStore<N, ()> = use_rgraph_store::<N, ()>();

    // Subscribe to the relevant slices: full node lookup + the
    // parent-set membership.
    let internal_opt = store.node_lookup.read().get(&props.id).cloned();
    let is_parent = store.parent_lookup.read().contains_key(&props.id);

    let Some(internal) = internal_opt else {
        // The node was removed between the renderer's id-list scan
        // and our render — render nothing.
        return rsx! {};
    };

    let nodes_draggable = *store.nodes_draggable.read();
    let elements_selectable = *store.elements_selectable.read();
    let nodes_focusable = *store.nodes_focusable.read();

    // Node-type resolution. The TS source falls back to "default" when
    // the supplied type isn't in `nodeTypes` and emits an error.
    let raw_type = internal.user.type_.clone().unwrap_or_else(|| "default".to_string());
    let resolved_type = BuiltInNodeType::parse(&raw_type).unwrap_or(BuiltInNodeType::Default);
    let node_type_name = resolved_type.as_str();

    // Per-element flag resolution mirroring TS lines 64–67. The TS
    // expression `node.draggable || (nodesDraggable && node.draggable === undefined)`
    // reduces to "use the node-level value if set, otherwise the global
    // default" — which `unwrap_or(default)` already expresses.
    let is_draggable = internal.user.draggable.unwrap_or(nodes_draggable);
    let is_selectable = internal.user.selectable.unwrap_or(elements_selectable);
    let is_focusable = internal.user.focusable_or_default(nodes_focusable);

    if internal.user.hidden.unwrap_or(false) {
        return rsx! {};
    }

    let has_dims = node_has_dimensions(&internal);
    let dimensions = get_node_dimensions(&internal);
    let inline_dims = get_node_inline_style_dimensions(&internal);

    // Wire the drag engine. `use_drag` returns pointer-event callbacks
    // we attach to the wrapper div, plus a `Signal<bool>` reflecting
    // the active-drag state for class-name composition.
    let drag_api = use_drag::<N, ()>(
        UseDragParams {
            disabled: !is_draggable,
            no_drag_class_name: Some(props.no_drag_class_name.clone()),
            handle_selector: None,
            node_id: Some(props.id.clone()),
            is_selectable,
            node_click_distance: Some(props.node_click_distance),
        },
        store.dom_bbox,
    );
    let dragging = *drag_api.dragging.read();

    // Z-index + transform inline style. Mirrors TS lines 203–210.
    let mut style = format!(
        "z-index:{};transform:translate({}px,{}px);visibility:{};",
        internal.internals.z,
        internal.internals.position_absolute.x,
        internal.internals.position_absolute.y,
        if has_dims { "visible" } else { "hidden" }
    );
    if let Some(w) = inline_dims.width {
        style.push_str(&format!("width:{w}px;"));
    }
    if let Some(h) = inline_dims.height {
        style.push_str(&format!("height:{h}px;"));
    }

    let has_pointer_events = is_selectable
        || is_draggable
        || props.on_click.is_some()
        || props.on_mouse_enter.is_some()
        || props.on_mouse_move.is_some()
        || props.on_mouse_leave.is_some();
    if !has_pointer_events {
        style.push_str("pointer-events:none;");
    }

    // ClassName composition (TS lines 186–201).
    let mut classes = String::from("react-flow__node");
    classes.push(' ');
    classes.push_str("react-flow__node-");
    classes.push_str(node_type_name);
    if is_draggable {
        classes.push(' ');
        classes.push_str(&props.no_pan_class_name);
    }
    if internal.user.selected.unwrap_or(false) {
        classes.push_str(" selected");
    }
    if is_selectable {
        classes.push_str(" selectable");
    }
    if is_parent {
        classes.push_str(" parent");
    }
    if is_draggable {
        classes.push_str(" draggable");
    }
    if dragging {
        classes.push_str(" dragging");
    }

    let node_id_for_observer = props.id.clone();
    let observer = use_node_observer::<N, ()>(node_id_for_observer);
    let on_mounted = observer.on_mounted;

    // Click handler.
    // The Cmd/Ctrl modifier on the click is mirrored into
    // `multi_selection_active` for the duration of the action so the
    // store's selection helpers treat this click as a multi-select.
    // (The global Cmd/Ctrl key listener that should drive this is a
    // Phase-3 stub.)
    let on_click = {
        let id = props.id.clone();
        let user_node = internal.user.clone();
        let on_click = props.on_click;
        move |event: Event<MouseData>| {
            use dioxus::prelude::{ReadableExt, WritableExt};
            // Stop click from bubbling to `<Pane>::onclick`, which would
            // otherwise call `store.reset_selected_elements()` and
            // undo the selection we're about to make.
            event.stop_propagation();
            if is_selectable {
                let mods = event.modifiers();
                let multi = mods.contains(Modifiers::META) || mods.contains(Modifiers::CONTROL);
                let prev_multi = *store.multi_selection_active.peek();
                if multi != prev_multi {
                    store.multi_selection_active.clone().set(multi);
                }

                handle_node_click(HandleNodeClickArgs::<N, ()> {
                    id: id.clone(),
                    store,
                    unselect: false,
                });
                if !multi && prev_multi {
                    store.multi_selection_active.clone().set(false);
                }
            }
            if let Some(cb) = on_click {
                cb.call(NodeMouseHandlerArgs {
                    event: std::rc::Rc::new(event),
                    node: user_node.clone(),
                });
            }
        }
    };

    let on_mouse_enter = mouse_forwarder::<N>(&props.on_mouse_enter, &internal.user);
    let on_mouse_move = mouse_forwarder::<N>(&props.on_mouse_move, &internal.user);
    let on_mouse_leave = mouse_forwarder::<N>(&props.on_mouse_leave, &internal.user);
    let on_context_menu = mouse_forwarder::<N>(&props.on_context_menu, &internal.user);
    let on_double_click = mouse_forwarder::<N>(&props.on_double_click, &internal.user);

    // Keyboard handler: arrow-key movement + enter/space selection.
    let on_key_down = {
        let id = props.id.clone();
        let mover = use_move_selected_nodes::<N, ()>();
        let disable_a11y = props.disable_keyboard_a11y;
        let selected = internal.user.selected.unwrap_or(false);
        let abs_x = internal.internals.position_absolute.x.trunc() as i64;
        let abs_y = internal.internals.position_absolute.y.trunc() as i64;
        move |evt: Event<KeyboardData>| {
            if disable_a11y {
                return;
            }
            let key = evt.key().to_string();
            // Backspace / Delete: remove every selected node + edge.
            // See the comment on the same branch in `EdgeWrapper` —
            // the global key listener that should drive this is a
            // Phase-3 stub, so we route through per-element keydown.
            if key == "Backspace" || key == "Delete" {
                evt.prevent_default();
                crate::hooks::use_global_key_handler::GlobalKeyHandlerEffects::<N, ()> {
                    store,
                }
                .run_delete();
                return;
            }
            // Selection keys: Enter, Space, Escape.
            const SELECT_KEYS: &[&str] = &["Enter", " ", "Escape"];
            if SELECT_KEYS.contains(&key.as_str()) && is_selectable {
                let unselect = key == "Escape";
                handle_node_click(HandleNodeClickArgs::<N, ()> {
                    id: id.clone(),
                    store,
                    unselect,
                });
                return;
            }
            if is_draggable
                && selected
                && let Some(direction) = arrow_key_diff(&key)
            {
                evt.prevent_default();
                let mods = evt.modifiers();
                let factor = if mods.contains(Modifiers::SHIFT) { 4.0 } else { 1.0 };

                // Update the aria-live message so screen readers
                // announce the move (TS lines 145–154).
                let direction_word = match key.as_str() {
                    "ArrowUp" => "up",
                    "ArrowDown" => "down",
                    "ArrowLeft" => "left",
                    "ArrowRight" => "right",
                    _ => "",
                };
                let aria_msg = rgraph_core::aria_live_message(direction_word, abs_x as f64, abs_y as f64);
                use dioxus::prelude::WritableExt;
                store.aria_live_message.clone().set(aria_msg);

                mover.call(MoveSelectedNodesParams { direction, factor });
            }
        }
    };

    let id_attr = props.id.clone();
    let test_id = format!("rf__node-{}", &props.id);
    let described_by = if props.disable_keyboard_a11y {
        String::new()
    } else {
        format!("{}-{}", ARIA_NODE_DESC_KEY, props.rf_id)
    };
    // `aria_role` lives on `NodePresentation` (Phase-7 plumbing); for
    // Phase 5 we default to "group" when focusable.
    let role = if is_focusable { "group".to_string() } else { String::new() };
    let aria_label = internal.user.aria_label.clone().unwrap_or_default();
    let tab_index = if is_focusable { Some(0i64) } else { None };

    // Provide the node id to descendants so `<Handle>` (Phase 6) can
    // read it from context.
    provide_node_id(props.id.clone());

    let inner_props = NodeProps::<BuiltInNodeData> {
        id: props.id.clone(),
        type_: Some(node_type_name.to_string()),
        data: built_in_data_from(&internal.user),
        selected: internal.user.selected,
        dragging: Some(dragging),
        is_connectable: Some(true),
        x_pos: internal.internals.position_absolute.x,
        y_pos: internal.internals.position_absolute.y,
        z_index: Some(internal.internals.z as i32),
        source_position: internal.user.source_position,
        target_position: internal.user.target_position,
        drag_handle: internal.user.drag_handle.clone(),
        parent_id: internal.user.parent_id.clone(),
        width: Some(dimensions.width),
        height: Some(dimensions.height),
        deletable: internal.user.deletable,
        selectable: internal.user.selectable,
    };

    // Render. Branch on tab_index so we don't emit a stray `tabindex`
    // attribute when the node isn't focusable.
    let inner: Element = match resolved_type {
        BuiltInNodeType::Input => rsx! { InputNode { ..inner_props.clone() } },
        BuiltInNodeType::Output => rsx! { OutputNode { ..inner_props.clone() } },
        BuiltInNodeType::Group => rsx! { GroupNode { ..inner_props.clone() } },
        BuiltInNodeType::Default => rsx! { DefaultNode { ..inner_props.clone() } },
    };

    // Pointer-capture wraps the four drag handlers so the cursor stays
    // glued to the node element even when the user yanks the mouse off
    // the node's box mid-drag. Without this, the wrapper stops receiving
    // pointermove the instant the cursor leaves its rect and the node
    // visibly lags behind the cursor. Wrapped in `Callback` (which is
    // `Copy`) so the same handler can be attached in both rendering
    // branches below.
    let capture_selector = format!("[data-id=\"{}\"]", props.id);
    let cap_for_down = capture_selector.clone();
    let cap_for_up = capture_selector.clone();
    let cap_for_cancel = capture_selector;

    let drag_pointer_down_inner = drag_api.on_pointer_down;
    let drag_pointer_move = drag_api.on_pointer_move;
    let drag_pointer_up_inner = drag_api.on_pointer_up;
    let drag_pointer_cancel_inner = drag_api.on_pointer_cancel;

    let drag_pointer_down: Callback<Event<PointerData>> = use_callback(move |e: Event<PointerData>| {
        // Stop pointerdown from bubbling to `<ZoomPane>`, otherwise the
        // pan/zoom engine starts a viewport-pan in lockstep with the
        // node drag (both shift by the same delta and the node appears
        // not to move). It also eats the subsequent `click` that would
        // normally select the node.
        e.stop_propagation();
        let pid = e.pointer_id();
        crate::dom::pointer::set_pointer_capture(&cap_for_down, pid);
        drag_pointer_down_inner.call(e);
    });
    let drag_pointer_up: Callback<Event<PointerData>> = use_callback(move |e: Event<PointerData>| {
        let pid = e.pointer_id();
        crate::dom::pointer::release_pointer_capture(&cap_for_up, pid);
        drag_pointer_up_inner.call(e);
    });
    let drag_pointer_cancel: Callback<Event<PointerData>> = use_callback(move |e: Event<PointerData>| {
        let pid = e.pointer_id();
        crate::dom::pointer::release_pointer_capture(&cap_for_cancel, pid);
        drag_pointer_cancel_inner.call(e);
    });

    if let Some(idx) = tab_index {
        rsx! {
            div {
                class: "{classes}",
                style: "{style}",
                "data-id": "{id_attr}",
                "data-testid": "{test_id}",
                "aria-roledescription": "node",
                "aria-describedby": "{described_by}",
                "aria-label": "{aria_label}",
                role: "{role}",
                tabindex: "{idx}",
                onmounted: on_mounted,
                onclick: on_click,
                ondoubleclick: on_double_click,
                onmouseenter: on_mouse_enter,
                onmousemove: on_mouse_move,
                onmouseleave: on_mouse_leave,
                oncontextmenu: on_context_menu,
                onkeydown: on_key_down,
                onpointerdown: move |e: Event<PointerData>| drag_pointer_down.call(e),
                onpointermove: move |e: Event<PointerData>| drag_pointer_move.call(e),
                onpointerup: move |e: Event<PointerData>| drag_pointer_up.call(e),
                onpointercancel: move |e: Event<PointerData>| drag_pointer_cancel.call(e),
                {inner}
            }
        }
    } else {
        rsx! {
            div {
                class: "{classes}",
                style: "{style}",
                "data-id": "{id_attr}",
                "data-testid": "{test_id}",
                "aria-roledescription": "node",
                "aria-describedby": "{described_by}",
                "aria-label": "{aria_label}",
                role: "{role}",
                onmounted: on_mounted,
                onclick: on_click,
                ondoubleclick: on_double_click,
                onmouseenter: on_mouse_enter,
                onmousemove: on_mouse_move,
                onmouseleave: on_mouse_leave,
                oncontextmenu: on_context_menu,
                onpointerdown: move |e: Event<PointerData>| drag_pointer_down.call(e),
                onpointermove: move |e: Event<PointerData>| drag_pointer_move.call(e),
                onpointerup: move |e: Event<PointerData>| drag_pointer_up.call(e),
                onpointercancel: move |e: Event<PointerData>| drag_pointer_cancel.call(e),
                {inner}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mouse_forwarder<N: Clone + PartialEq + 'static>(
    handler: &Option<NodeMouseHandler<N>>,
    user_node: &Node<N>,
) -> impl FnMut(Event<MouseData>) + 'static {
    let handler = *handler;
    let user_node = user_node.clone();
    move |event: Event<MouseData>| {
        if let Some(cb) = handler {
            cb.call(NodeMouseHandlerArgs {
                event: std::rc::Rc::new(event),
                node: user_node.clone(),
            });
        }
    }
}

/// Coerce a user-data payload to [`BuiltInNodeData`]. The TS source
/// passes `node.data` straight through (it's already typed as
/// `BuiltInNode`'s data shape on built-in components). The wrapper
/// is generic over `N`, but the inner built-in components only know
/// how to render `BuiltInNodeData`, so we runtime-downcast:
///
/// * If `N == BuiltInNodeData`, we forward the label payload verbatim.
/// * Otherwise, we fall back to [`BuiltInNodeData::Empty`] so the
///   built-in renderers stay well-typed.
///
/// When Phase 7 introduces the typed `nodeTypes` registry, this
/// coercion is replaced by dispatch through the registry.
fn built_in_data_from<N: Clone + 'static>(node: &Node<N>) -> BuiltInNodeData {
    let any: &dyn std::any::Any = &node.data;
    if let Some(d) = any.downcast_ref::<BuiltInNodeData>() {
        return d.clone();
    }
    BuiltInNodeData::Empty
}

// `node_has_dimensions` import is exercised through `has_dims` above.
// `Modifiers` / keyboard imports are exercised by the `on_key_down`
// closure. No additional suppressions required.

// Extension trait for `Node<N>` to express the "fall back to global"
// rule for the `focusable` flag (TS line 67) without growing the
// `Node` struct in `rgraph-core`. Lives here because it's
// React-side presentation logic.
trait NodeFocusableExt {
    fn focusable_or_default(&self, default_focusable: bool) -> bool;
}

impl<N: Clone> NodeFocusableExt for crate::types::nodes::Node<N> {
    fn focusable_or_default(&self, default_focusable: bool) -> bool {
        // TS: !!(node.focusable || (nodesFocusable && typeof node.focusable === 'undefined'))
        self.focusable_or_default_inner(default_focusable)
    }
}

trait NodeFocusableInner {
    fn focusable_or_default_inner(&self, default_focusable: bool) -> bool;
}

// The `Node<N>` type doesn't carry a `focusable` field in our port;
// it lives on `NodePresentation` (Phase 1). For Phase 5 we honour the
// global default only — Phase 7 will plumb `NodePresentation` through
// the wrapper props.
impl<N: Clone> NodeFocusableInner for crate::types::nodes::Node<N> {
    fn focusable_or_default_inner(&self, default_focusable: bool) -> bool {
        default_focusable
    }
}
