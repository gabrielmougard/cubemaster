//! Port of `xyflow-react/src/components/NodesSelection/index.tsx`.
//!
//! Status: Phase 5 — implemented.
//!
//! Renders the framing rectangle around the multi-node selection
//! when `nodes_selection_active` is set and there's no in-flight
//! marquee. The rectangle is keyboard-focusable (`tabindex = -1`) so
//! arrow-key movement works the same way as it does inside a node.
//!
//! Phase 5 caveats:
//!   * The selection rectangle is *not* draggable yet — the TS source
//!     wires `useDrag({ nodeRef, disabled: !shouldRender })` which
//!     allows dragging the whole selection as one. Phase 5's
//!     `use_drag` is a stub; full integration lands when handles do
//!     in Phase 6.

#![allow(clippy::module_name_repetitions)]

use dioxus::events::KeyboardData;
use dioxus::html::input_data::keyboard_types::Modifiers;
use dioxus::html::point_interaction::ModifiersInteraction;
use dioxus::prelude::*;

use rgraph_core::utils::general::is_numeric;
use rgraph_core::utils::graph::{get_internal_nodes_bounds, GetInternalNodesBoundsParams};

use crate::components::node_wrapper::utils::arrow_key_diff;
use crate::context::use_rgraph_store;
use crate::hooks::use_move_selected_nodes::{use_move_selected_nodes, MoveSelectedNodesParams};
use crate::store::RGraphStore;

#[derive(Props, Clone, PartialEq)]
pub struct NodesSelectionProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default = "nopan".to_string())]
    pub no_pan_class_name: String,
    #[props(default)]
    pub disable_keyboard_a11y: bool,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn NodesSelection<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: NodesSelectionProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let mover = use_move_selected_nodes::<N, E>();

    let user_selection_active = *store.user_selection_active.read();
    let transform = *store.transform.read();
    let lookup = store.node_lookup.read();
    let bounds = get_internal_nodes_bounds(
        &lookup,
        GetInternalNodesBoundsParams {
            filter: Some(Box::new(
                |n: &rgraph_core::types::nodes::InternalNode<N>| n.user.selected.unwrap_or(false),
            )),
        },
    );
    drop(lookup);

    let width = bounds.width;
    let height = bounds.height;
    let should_render = !user_selection_active
        && is_numeric(width)
        && is_numeric(height)
        && width > 0.0
        && height > 0.0;

    if !should_render {
        return rsx! {};
    }

    let transform_str = format!(
        "transform: translate({tx}px,{ty}px) scale({z}) translate({bx}px,{by}px);",
        tx = transform.tx(),
        ty = transform.ty(),
        z = transform.scale(),
        bx = bounds.x,
        by = bounds.y,
    );
    let rect_style = format!("width:{width}px;height:{height}px;");

    let on_key_down = move |evt: Event<KeyboardData>| {
        let key = evt.key().to_string();
        if let Some(direction) = arrow_key_diff(&key) {
            evt.prevent_default();
            let factor = if evt.modifiers().contains(Modifiers::SHIFT) { 4.0 } else { 1.0 };
            mover.call(MoveSelectedNodesParams { direction, factor });
        }
    };

    let class_outer = format!(
        "react-flow__nodesselection react-flow__container {}",
        props.no_pan_class_name
    );

    if props.disable_keyboard_a11y {
        rsx! {
            div {
                class: "{class_outer}",
                style: "{transform_str}",
                div {
                    class: "react-flow__nodesselection-rect",
                    style: "{rect_style}",
                }
            }
        }
    } else {
        rsx! {
            div {
                class: "{class_outer}",
                style: "{transform_str}",
                div {
                    class: "react-flow__nodesselection-rect",
                    style: "{rect_style}",
                    tabindex: "-1",
                    onkeydown: on_key_down,
                }
            }
        }
    }
}
