//! Port of `xyflow-react/src/additional-components/NodeResizer/NodeResizeControl.tsx`.
//!
//! Status: Phase 8 — fully implemented.
//!
//! Creates an [`XYResizer`] state machine on mount and forwards
//! `pointerdown/move/up/cancel` events to it. The `on_change`
//! callback turns the per-tick [`ResizerChange`] into
//! [`NodeChange::Dimensions`] and [`NodeChange::Position`] entries
//! that flow through [`crate::store::actions::trigger_node_changes`],
//! the same path used by drag-to-move.

#![allow(clippy::module_name_repetitions)]

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::events::PointerData;
use dioxus::html::input_data::keyboard_types::Modifiers;
use dioxus::html::input_data::MouseButton;
use dioxus::html::point_interaction::{
    InteractionLocation, ModifiersInteraction, PointerInteraction,
};
use dioxus::prelude::*;
use dioxus_signals::ReadableExt;
use rgraph_core::types::changes::{NodeChange, SetAttributesMode};
use rgraph_core::types::geometry::{Dimensions, XYPosition};
use rgraph_core::types::nodes::PointerEventLike;
use rgraph_core::utils::general::evaluate_absolute_position;
use rgraph_core::utils::store::{handle_expand_parent, ParentExpandChild};
use rgraph_core::xyresizer::types::{
    ControlPosition, ResizeBoundaries, ResizeControlDirection, ResizeControlVariant, ResizerChange,
};
use rgraph_core::xyresizer::{
    OnResizerChangeFn, ResizerStoreSnapshot, XYResizer, XYResizerParams, XYResizerUpdateParams,
};
use rgraph_drag::PointerId;

use crate::context::use_rgraph_store;
use crate::contexts::node_id::use_node_id;
use crate::store::RGraphStore;
use crate::types::nodes::BuiltInNodeData;

use super::types::{control_position_classes, default_position_for, NodeResizerCommon};

#[derive(Props, Clone, PartialEq)]
pub struct NodeResizeControlProps<
    N: Clone + PartialEq + 'static = BuiltInNodeData,
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default)]
    pub node_id: Option<String>,
    #[props(default)]
    pub position: Option<ControlPosition>,
    #[props(default = ResizeControlVariant::Handle)]
    pub variant: ResizeControlVariant,
    #[props(default)]
    pub resize_direction: Option<ResizeControlDirection>,
    #[props(default)]
    pub class_name: Option<String>,
    #[props(default)]
    pub style: Option<String>,
    #[props(default)]
    pub color: Option<String>,
    #[props(default = NodeResizerCommon::default())]
    pub common: NodeResizerCommon,
    #[props(default)]
    pub children: Element,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

fn pointer_event_like(evt: &Event<PointerData>) -> PointerEventLike {
    let data: &PointerData = evt;
    let coords = <PointerData as InteractionLocation>::client_coordinates(data);
    let mods = <PointerData as ModifiersInteraction>::modifiers(data);
    let button = match <PointerData as PointerInteraction>::trigger_button(data) {
        Some(MouseButton::Primary) => 0,
        Some(MouseButton::Auxiliary) => 1,
        Some(MouseButton::Secondary) => 2,
        Some(MouseButton::Fourth) => 3,
        Some(MouseButton::Fifth) => 4,
        _ => 0,
    };
    PointerEventLike {
        client_x: coords.x,
        client_y: coords.y,
        button,
        buttons: 0,
        ctrl_key: mods.contains(Modifiers::CONTROL),
        shift_key: mods.contains(Modifiers::SHIFT),
        alt_key: mods.contains(Modifiers::ALT),
        meta_key: mods.contains(Modifiers::META),
    }
}

fn pointer_id_from(evt: &Event<PointerData>) -> PointerId {
    let data: &PointerData = evt;
    let kind = data.pointer_type();
    let id_i32 = data.pointer_id();
    match kind.as_str() {
        "mouse" => PointerId::Mouse,
        "touch" => PointerId::Touch(id_i32 as u64),
        _ => PointerId::Pointer(id_i32 as u64),
    }
}

#[component]
pub fn NodeResizeControl<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: NodeResizeControlProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let context_id = use_node_id();
    let resolved_id = props
        .node_id
        .clone()
        .or(context_id)
        .unwrap_or_default();

    let control_position = props
        .position
        .unwrap_or_else(|| default_position_for(props.variant));
    let positions = control_position_classes(control_position);
    let variant_class = match props.variant {
        ResizeControlVariant::Handle => "handle",
        ResizeControlVariant::Line => "line",
    };

    let mut class = String::from("react-flow__resize-control nodrag");
    for p in positions {
        class.push(' ');
        class.push_str(p);
    }
    class.push(' ');
    class.push_str(variant_class);
    if let Some(extra) = &props.class_name {
        class.push(' ');
        class.push_str(extra);
    }

    let mut style_str = props.style.clone().unwrap_or_default();
    if let Some(color) = props.color.as_ref().or(props.common.color.as_ref()) {
        let prop = if matches!(props.variant, ResizeControlVariant::Handle) {
            "background-color"
        } else {
            "border-color"
        };
        style_str.push_str(&format!("{prop}:{color};"));
    }

    // ----- XYResizer lifecycle + change callbacks -----
    //
    // The resizer is `D`-generic. The store carries `N`, so we plumb
    // `D = N` through here. `on_change` writes back via
    // `trigger_node_changes` (also `N`-generic).
    let resizer_slot: Rc<RefCell<Option<XYResizer<N>>>> = use_hook(|| Rc::new(RefCell::new(None)));
    let id_for_init = resolved_id.clone();
    let id_for_change = resolved_id.clone();
    let resize_dir_for_change = props.resize_direction;

    if id_for_init.is_empty() {
        // No node id available — render visual chrome only.
        return rsx! {
            div { class: "{class}", style: "{style_str}", {props.children} }
        };
    }

    if resizer_slot.borrow().is_none() {
        let store_for_get = store;
        let get_store_items: Rc<dyn Fn() -> ResizerStoreSnapshot<N>> = {
            let id = id_for_init.clone();
            let _ = id; // kept for future per-node specialization
            Rc::new(move || ResizerStoreSnapshot {
                node_lookup: Rc::new(store_for_get.node_lookup.peek().clone()),
                transform: *store_for_get.transform.peek(),
                snap_grid: *store_for_get.snap_grid.peek(),
                snap_to_grid: *store_for_get.snap_to_grid.peek(),
                node_origin: *store_for_get.node_origin.peek(),
            })
        };

        let store_for_change = store;
        let on_change: OnResizerChangeFn = {
            let id = id_for_change.clone();
            let resize_direction = resize_dir_for_change;
            Rc::new(move |change: &ResizerChange, child_changes: &[_]| {
                let mut changes: Vec<NodeChange<N>> = Vec::new();
                let lookup = store_for_change.node_lookup.peek().clone();
                let parent_lookup = store_for_change.parent_lookup.peek().clone();
                let node_origin = *store_for_change.node_origin.peek();
                let mut next_x = change.x;
                let mut next_y = change.y;

                if let Some(node) = lookup.get(&id) {
                    let has_parent = node.user.parent_id.is_some();
                    let expand = node.user.expand_parent.unwrap_or(false);
                    if let (true, true, Some(parent_id)) =
                        (has_parent, expand, node.user.parent_id.as_ref())
                    {
                        let origin = node.user.origin.unwrap_or(node_origin);
                        let width = change.width.or(node.measured.width).unwrap_or(0.0);
                        let height = change.height.or(node.measured.height).unwrap_or(0.0);
                        let absolute = evaluate_absolute_position(
                            XYPosition {
                                x: change.x.unwrap_or(node.user.position.x),
                                y: change.y.unwrap_or(node.user.position.y),
                            },
                            Dimensions { width, height },
                            parent_id,
                            &lookup,
                            origin,
                        );
                        let child = ParentExpandChild {
                            id: node.user.id.clone(),
                            parent_id: parent_id.clone(),
                            rect: rgraph_core::types::geometry::Rect::new(
                                absolute.x,
                                absolute.y,
                                width,
                                height,
                            ),
                        };
                        let expansions = handle_expand_parent(
                            &[child],
                            &lookup,
                            &parent_lookup,
                            node_origin,
                        );
                        changes.extend(expansions);
                        next_x = change.x.map(|cx| (origin.0 * width).max(cx));
                        next_y = change.y.map(|cy| (origin.1 * height).max(cy));
                    }
                }

                if let (Some(nx), Some(ny)) = (next_x, next_y) {
                    changes.push(NodeChange::Position {
                        id: id.clone(),
                        position: Some(XYPosition { x: nx, y: ny }),
                        position_absolute: None,
                        dragging: None,
                    });
                }

                if let (Some(w), Some(h)) = (change.width, change.height) {
                    let set_attrs = match resize_direction {
                        None => SetAttributesMode::All,
                        Some(ResizeControlDirection::Horizontal) => SetAttributesMode::WidthOnly,
                        Some(ResizeControlDirection::Vertical) => SetAttributesMode::HeightOnly,
                    };
                    changes.push(NodeChange::Dimensions {
                        id: id.clone(),
                        dimensions: Some(Dimensions { width: w, height: h }),
                        resizing: Some(true),
                        set_attributes: set_attrs,
                    });
                }

                for cc in child_changes {
                    changes.push(NodeChange::Position {
                        id: cc.id.clone(),
                        position: Some(cc.position),
                        position_absolute: None,
                        dragging: None,
                    });
                }

                store_for_change.trigger_node_changes(changes);
            })
        };

        let store_for_end = store;
        let on_end: rgraph_core::xyresizer::OnResizerEndStoreFn = {
            let id = id_for_change.clone();
            Rc::new(move |change: &ResizerChange| {
                let dims = match (change.width, change.height) {
                    (Some(w), Some(h)) => Some(Dimensions { width: w, height: h }),
                    _ => None,
                };
                store_for_end.trigger_node_changes(vec![NodeChange::Dimensions {
                    id: id.clone(),
                    dimensions: dims,
                    resizing: Some(false),
                    set_attributes: SetAttributesMode::None,
                }]);
            })
        };

        let resizer = XYResizer::<N>::new(XYResizerParams {
            node_id: id_for_init.clone(),
            get_store_items,
            on_change,
            on_end: Some(on_end),
        });
        *resizer_slot.borrow_mut() = Some(resizer);
    }

    // Push the latest update params to the state machine.
    if let Some(r) = resizer_slot.borrow().as_ref() {
        r.update(XYResizerUpdateParams {
            control_position,
            boundaries: ResizeBoundaries {
                min_width: props.common.min_width,
                min_height: props.common.min_height,
                max_width: props.common.max_width,
                max_height: props.common.max_height,
            },
            keep_aspect_ratio: props.common.keep_aspect_ratio,
            resize_direction: props.resize_direction,
            on_resize_start: props.common.on_resize_start.clone(),
            on_resize: props.common.on_resize.clone(),
            on_resize_end: props.common.on_resize_end.clone(),
            should_resize: props.common.should_resize.clone(),
        });
    }

    let on_pointer_down = {
        let slot = Rc::clone(&resizer_slot);
        move |evt: Event<PointerData>| {
            if let Some(r) = slot.borrow().as_ref() {
                let pl = pointer_event_like(&evt);
                r.handle_pointer_down(pointer_id_from(&evt), &pl, pl.button, pl.ctrl_key);
            }
        }
    };
    let on_pointer_move = {
        let slot = Rc::clone(&resizer_slot);
        move |evt: Event<PointerData>| {
            if let Some(r) = slot.borrow().as_ref() {
                r.handle_pointer_move(pointer_id_from(&evt), &pointer_event_like(&evt));
            }
        }
    };
    let on_pointer_up = {
        let slot = Rc::clone(&resizer_slot);
        move |evt: Event<PointerData>| {
            if let Some(r) = slot.borrow().as_ref() {
                r.handle_pointer_up(pointer_id_from(&evt), &pointer_event_like(&evt));
            }
        }
    };
    let on_pointer_cancel = {
        let slot = Rc::clone(&resizer_slot);
        move |evt: Event<PointerData>| {
            if let Some(r) = slot.borrow().as_ref() {
                r.handle_pointer_cancel(pointer_id_from(&evt), &pointer_event_like(&evt));
            }
        }
    };
    // ---------------------------------------------------

    rsx! {
        div {
            class: "{class}",
            style: "{style_str}",
            onpointerdown: on_pointer_down,
            onpointermove: on_pointer_move,
            onpointerup: on_pointer_up,
            onpointercancel: on_pointer_cancel,
            {props.children}
        }
    }
}

/// Convenience wrapper that forces `variant = ResizeControlVariant::Line`.
/// Mirrors the TS `ResizeControlLine`.
#[component]
pub fn ResizeControlLine<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: NodeResizeControlProps<N, E>,
) -> Element {
    let mut p = props.clone();
    p.variant = ResizeControlVariant::Line;
    rsx! { NodeResizeControl::<N, E> { ..p } }
}
