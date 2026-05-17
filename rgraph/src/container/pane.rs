//! Port of `xyflow-react/src/container/Pane/index.tsx`.
//!
//! Status: Phase 4 — partial implementation.
//!
//! The pane is the transparent `<div>` covering the viewport. It:
//!
//! * Forwards background `click` / `contextmenu` / mouse-enter/move/leave
//!   events to the host's `onPaneXxx` callbacks.
//! * Forwards wheel events to `on_pane_scroll`.
//! * On click, calls `reset_selected_elements` and clears the
//!   `nodes_selection_active` flag (matching TS lines 112–123).
//! * Hosts the `<UserSelection>` overlay rectangle.
//! * Phase 5 will add the marquee-selection drag (currently a TODO so
//!   the API stays stable while drag plumbing lands).

#![allow(clippy::module_name_repetitions)]

use dioxus::events::{MouseData, WheelData};
use dioxus::prelude::*;

use rgraph_core::types::panzoom::PanOnDrag;
use rgraph_core::types::viewport::SelectionMode;

use crate::components::user_selection::UserSelection;
use crate::context::use_rgraph_store;
use crate::store::RGraphStore;
use crate::types::component_props::{OnPaneScroll, PaneMouseHandler};

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

    // Compose className: "react-flow__pane" + " draggable" if pan is
    // enabled with the left button + " dragging" if currently panning
    // + " selection" if marquee-selecting.
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

    // Pane click — only fires when the click target is the pane
    // itself (not a child). The `wrapHandler` from TS is the same
    // semantics as `event.target == container`, but Dioxus doesn't
    // expose `event.target` cheaply; for Phase 4 we trust the host's
    // children to stop propagation themselves.
    let on_click = {
        let handler = props.on_pane_click;
        move |event: Event<MouseData>| {
            if connection_in_progress {
                return;
            }
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
            // TS lines 125–129: when pan_on_drag is `[…2…]`, suppress
            // the default context menu so right-click pan works.
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
    let on_pointer_move = {
        let handler = props.on_pane_mouse_move;
        move |event: Event<MouseData>| {
            if let Some(cb) = handler {
                cb.call(std::rc::Rc::new(event));
            }
            // TODO(rgraph/phase5): when `is_selection_enabled` is true,
            // call into the marquee-selection drag plumbing here
            // (TS lines 282–312 of `Pane/index.tsx`).
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
            onmousemove: on_pointer_move,
            onmouseleave: on_pointer_leave,
            {props.children}
            UserSelection::<N, E> {}
        }
    }
}
