//! Port of `xyflow-react/src/components/Handle/index.tsx`.
//!
//! Status: Phase 6 — partial implementation.
//!
//! The visual handle div is fully ported (positions, classes for
//! connect state, data attributes). The pointer-driven connection
//! drag through `XYHandle` is deferred to Phase 7 since it requires
//! the pane bounds + `<RGraph>` integration. The click-connect path
//! (TS lines 163–215) is implemented and works without DOM
//! measurement.

#![allow(clippy::module_name_repetitions)]

use dioxus::events::{MouseData, PointerData};
use dioxus::html::point_interaction::InteractionLocation;
use dioxus::prelude::*;

use rgraph_core::types::connection::{
    Connection, ConnectionInProgress, ConnectionMode, ConnectionState,
};
use rgraph_core::types::geometry::{Position, XYPosition};
use rgraph_core::types::handles::{Handle as CoreHandle, HandleType};
use rgraph_core::utils::general::get_node_dimensions;

use crate::context::use_rgraph_store;
use crate::contexts::node_id::use_node_id;
use crate::store::RGraphStore;
use crate::types::component_props::OnConnect;
use crate::types::nodes::BuiltInNodeData;
use crate::types::store::ConnectionClickStartHandle;

#[derive(Props, Clone, PartialEq)]
pub struct HandleProps {
    #[props(default = HandleType::Source)]
    pub type_: HandleType,
    #[props(default = Position::Top)]
    pub position: Position,
    #[props(default = true)]
    pub is_connectable: bool,
    #[props(default = true)]
    pub is_connectable_start: bool,
    #[props(default = true)]
    pub is_connectable_end: bool,
    #[props(default)]
    pub id: Option<String>,
    #[props(default)]
    pub on_connect: Option<OnConnect>,
    #[props(default)]
    pub class_name: Option<String>,
    #[props(default)]
    pub children: Element,
}

/// The `<Handle />` component defines a connection point on a custom
/// node. Mirrors the TS `Handle`.
#[component]
pub fn Handle(props: HandleProps) -> Element {
    // Read store + nearest node id from context.
    type DefaultN = crate::types::nodes::BuiltInNodeData;
    let store: RGraphStore<DefaultN, ()> = use_rgraph_store::<DefaultN, ()>();
    let node_id = use_node_id();

    let handle_id: Option<String> = props.id.clone();
    let is_target = matches!(props.type_, HandleType::Target);
    let rf_id = store.rf_id.read().clone();
    let no_pan = store.no_pan_class_name.read().clone();
    let connect_on_click = *store.connect_on_click.read();
    let connection_state = store.connection.read().clone();
    let click_start = store.connection_click_start_handle.read().clone();
    let connection_mode = *store.connection_mode.read();

    let nid = node_id.clone().unwrap_or_default();

    // Determine the per-state visual flags (TS lines 47–63).
    let (connecting_from, connecting_to, click_connecting, is_possible_end_handle,
        connection_in_process, click_connection_in_process, valid) =
        compute_connection_flags(
            &connection_state,
            click_start.as_ref(),
            &nid,
            handle_id.as_deref(),
            props.type_,
            connection_mode,
        );

    // Class composition.
    let mut classes = String::from("react-flow__handle ");
    classes.push_str(match props.position {
        Position::Top => "react-flow__handle-top",
        Position::Bottom => "react-flow__handle-bottom",
        Position::Left => "react-flow__handle-left",
        Position::Right => "react-flow__handle-right",
    });
    classes.push_str(" nodrag ");
    classes.push_str(&no_pan);
    if let Some(extra) = &props.class_name {
        classes.push(' ');
        classes.push_str(extra);
    }
    if is_target { classes.push_str(" target"); } else { classes.push_str(" source"); }
    if props.is_connectable { classes.push_str(" connectable"); }
    if props.is_connectable_start { classes.push_str(" connectablestart"); }
    if props.is_connectable_end { classes.push_str(" connectableend"); }
    if click_connecting { classes.push_str(" clickconnecting"); }
    if connecting_from { classes.push_str(" connectingfrom"); }
    if connecting_to { classes.push_str(" connectingto"); }
    if valid { classes.push_str(" valid"); }

    let connection_indicator =
        props.is_connectable
            && (!connection_in_process || is_possible_end_handle)
            && (if connection_in_process || click_connection_in_process {
                props.is_connectable_end
            } else {
                props.is_connectable_start
            });
    if connection_indicator {
        classes.push_str(" connectionindicator");
    }

    // Compose data attributes.
    let pos_str = match props.position {
        Position::Top => "top",
        Position::Bottom => "bottom",
        Position::Left => "left",
        Position::Right => "right",
    };
    let handle_id_str = handle_id.clone().unwrap_or_default();
    let data_id = format!(
        "{rf_id}-{nid}-{handle_id_str}-{}",
        match props.type_ {
            HandleType::Source => "source",
            HandleType::Target => "target",
        }
    );

    // Connection finaliser shared by click-to-connect (second click on a
    // handle) and pointer-drag-to-connect (release on another handle).
    let user_on_connect = props.on_connect;
    let finalise = {
        let node_id = node_id.clone();
        let handle_id = handle_id.clone();
        move |from: &ConnectionClickStartHandle| {
            use dioxus::prelude::WritableExt;
            let Some(nid) = node_id.clone() else { return };
            // Reject self-connections so dragging back onto the source
            // doesn't emit a degenerate `n → n` edge.
            if from.node_id == nid && from.id == handle_id {
                store.connection_click_start_handle.clone().set(None);
                return;
            }
            let connection = if matches!(from.type_, HandleType::Source) {
                Connection {
                    source: from.node_id.clone(),
                    source_handle: from.id.clone(),
                    target: nid,
                    target_handle: handle_id.clone(),
                }
            } else {
                Connection {
                    source: nid,
                    source_handle: handle_id.clone(),
                    target: from.node_id.clone(),
                    target_handle: from.id.clone(),
                }
            };
            if let Some(handler) = *store.on_connect.peek() {
                handler.call(connection.clone());
            }
            if let Some(handler) = user_on_connect {
                handler.call(connection);
            }
            store.connection_click_start_handle.clone().set(None);
        }
    };

    // Pointer-down on a handle. We stop propagation so the surrounding
    // `<NodeWrapper>`'s drag engine doesn't also start moving the
    // node. Only set the click-start when nothing is pending.
    // Otherwise a second press would overwrite the "first click" of a
    // click-to-connect flow before pointer-up on this handle gets a
    // chance to commit. In addition, we seed `store.connection` with an
    // `InProgress` state so `<ConnectionLine>` can draw a live preview
    // from the handle to the cursor while the user drags.
    let on_pointer_down = {
        let node_id = node_id.clone();
        let handle_id = handle_id.clone();
        let handle_type = props.type_;
        let position = props.position;
        let is_connectable_start = props.is_connectable_start;
        move |evt: Event<PointerData>| {
            use dioxus::prelude::WritableExt;
            evt.stop_propagation();
            let Some(nid) = node_id.clone() else { return };
            if !is_connectable_start {
                return;
            }

            if store.connection_click_start_handle.peek().is_some() {
                return;
            }

            store
                .connection_click_start_handle
                .clone()
                .set(Some(ConnectionClickStartHandle {
                    node_id: nid.clone(),
                    id: handle_id.clone(),
                    type_: handle_type,
                }));

            // Seed the live connection-line state. The drag-engine
            // doesn't run for handle clicks (we stopped propagation
            // above), so `store.connection` is the only thing that
            // tells `<ConnectionLine>` to render.
            let node_opt = {
                use dioxus::prelude::ReadableExt;
                store.node_lookup.peek().get(&nid).cloned()
            };
            let Some(node) = node_opt else { return };
            let dim = get_node_dimensions(&node);
            let (anchor_dx, anchor_dy) = match position {
                Position::Top => (dim.width / 2.0, 0.0),
                Position::Bottom => (dim.width / 2.0, dim.height),
                Position::Left => (0.0, dim.height / 2.0),
                Position::Right => (dim.width, dim.height / 2.0),
            };
            let anchor_x = node.internals.position_absolute.x + anchor_dx;
            let anchor_y = node.internals.position_absolute.y + anchor_dy;

            let from_handle = CoreHandle {
                id: handle_id.clone(),
                node_id: nid,
                type_: handle_type,
                position,
                x: anchor_dx,
                y: anchor_dy,
                width: 6.0,
                height: 6.0,
            };

            let bbox = *store.dom_bbox.peek();
            let client = evt.client_coordinates();
            let pane_x = client.x - bbox.x;
            let pane_y = client.y - bbox.y;

            let state = ConnectionInProgress::<rgraph_core::types::nodes::InternalNode<BuiltInNodeData>> {
                is_valid: None,
                from: XYPosition::new(anchor_x, anchor_y),
                from_handle,
                from_position: position,
                from_node: node,
                // `to` is stored in pane-relative screen coords; the
                // `use_connection` hook converts to flow coords on
                // read.
                to: XYPosition::new(pane_x, pane_y),
                to_handle: None,
                to_position: position.opposite(),
                to_node: None,
                pointer: XYPosition::new(client.x, client.y),
            };
            store.connection.clone().set(ConnectionState::InProgress(state));
        }
    };

    // Pointer-up. Two behaviours collapse here:
    //   * Drag-to-connect : pointer was pressed on handle A, dragged
    //     to handle B, released over B -> finalise A -> B.
    //   * Click-to-connect : first click sets `click_start` to A
    //     (via pointer-down + same-handle pointer-up no-op); a later
    //     click anywhere else lands here on a *different* handle and
    //     finalises the same way.
    let on_pointer_up = {
        let finalise = finalise.clone();
        let node_id = node_id.clone();
        let handle_id = handle_id.clone();
        move |evt: Event<PointerData>| {
            use dioxus::prelude::WritableExt;
            evt.stop_propagation();
            let Some(nid) = node_id.clone() else { return };
            let from = store.connection_click_start_handle.peek().clone();
            if let Some(from) = from && (from.node_id != nid || from.id != handle_id) {
                finalise(&from);
            }

            // Clear the live preview either way. Finalised or not,
            // the drag is over once the pointer is released on a handle.
            store
                .connection
                .clone()
                .set(ConnectionState::NoConnection);
        }
    };

    // Click handler retained for keyboard / touch fallbacks. Stops
    // propagation only. The actual state changes happen on
    // pointerdown / pointerup which fire first.
    let on_click_outer = move |evt: Event<MouseData>| {
        evt.stop_propagation();
    };

    // Both pointer-drag and click flows are wired regardless of the
    // legacy `connect_on_click` toggle.
    let _ = connect_on_click;
    let _ = click_start; // remains read for class-name composition above
    rsx! {
        div {
            "data-handleid": "{handle_id_str}",
            "data-nodeid": "{nid}",
            "data-handlepos": "{pos_str}",
            "data-id": "{data_id}",
            class: "{classes}",
            onpointerdown: on_pointer_down,
            onpointerup: on_pointer_up,
            onclick: on_click_outer,
            {props.children}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn compute_connection_flags(
    connection: &rgraph_core::types::connection::ConnectionState<
        rgraph_core::types::nodes::InternalNode<crate::types::nodes::BuiltInNodeData>,
    >,
    click_start: Option<&ConnectionClickStartHandle>,
    node_id: &str,
    handle_id: Option<&str>,
    type_: HandleType,
    connection_mode: ConnectionMode,
) -> (bool, bool, bool, bool, bool, bool, bool) {
    use rgraph_core::types::connection::ConnectionState;
    match connection {
        ConnectionState::NoConnection => {
            let click_connecting = click_start.is_some_and(|c| {
                c.node_id == node_id && c.id.as_deref() == handle_id && c.type_ == type_
            });
            (
                false,
                false,
                click_connecting,
                true,
                false,
                click_start.is_some(),
                false,
            )
        }
        ConnectionState::InProgress(p) => {
            let from = &p.from_handle;
            let to = p.to_handle.as_ref();
            let connecting_from = from.node_id == node_id
                && from.id.as_deref() == handle_id
                && from.type_ == type_;
            let connecting_to = to.is_some_and(|t| {
                t.node_id == node_id && t.id.as_deref() == handle_id && t.type_ == type_
            });
            let is_possible_end_handle = match connection_mode {
                ConnectionMode::Strict => from.type_ != type_,
                ConnectionMode::Loose => from.node_id != node_id || from.id.as_deref() != handle_id,
            };
            let valid = connecting_to && p.is_valid == Some(true);
            (
                connecting_from,
                connecting_to,
                false,
                is_possible_end_handle,
                true,
                click_start.is_some(),
                valid,
            )
        }
    }
}
