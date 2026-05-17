//! Port of `xyflow-react/src/additional-components/Controls/Controls.tsx`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_signals::{ReadableExt, WritableExt};
use rgraph_core::types::viewport::PanelPosition;

use crate::components::panel::Panel;
use crate::context::use_rgraph_store;
use crate::hooks::use_viewport_helper::use_viewport_helper;
use crate::store::RGraphStore;

use super::control_button::ControlButton;
use super::icons::{FitViewIcon, LockIcon, MinusIcon, PlusIcon, UnlockIcon};
use super::types::{ControlsFitViewOptions, ControlsOrientation, default_controls_position};

#[derive(Props, Clone)]
pub struct ControlsProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default = true)]
    pub show_zoom: bool,
    #[props(default = true)]
    pub show_fit_view: bool,
    #[props(default = true)]
    pub show_interactive: bool,
    #[props(default)]
    pub fit_view_options: Option<Rc<ControlsFitViewOptions>>,
    #[props(default)]
    pub on_zoom_in: Option<EventHandler<()>>,
    #[props(default)]
    pub on_zoom_out: Option<EventHandler<()>>,
    #[props(default)]
    pub on_fit_view: Option<EventHandler<()>>,
    #[props(default)]
    pub on_interactive_change: Option<EventHandler<bool>>,
    #[props(default)]
    pub position: Option<PanelPosition>,
    #[props(default)]
    pub orientation: Option<ControlsOrientation>,
    #[props(default)]
    pub class_name: Option<String>,
    #[props(default)]
    pub style: Option<String>,
    #[props(default)]
    pub aria_label: Option<String>,
    #[props(default)]
    pub children: Element,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> PartialEq
    for ControlsProps<N, E>
{
    fn eq(&self, other: &Self) -> bool {
        self.show_zoom == other.show_zoom
            && self.show_fit_view == other.show_fit_view
            && self.show_interactive == other.show_interactive
            && self.fit_view_options.as_ref().map(Rc::as_ptr)
                == other.fit_view_options.as_ref().map(Rc::as_ptr)
            && self.position == other.position
            && self.orientation == other.orientation
            && self.class_name == other.class_name
            && self.style == other.style
            && self.aria_label == other.aria_label
    }
}

#[component]
pub fn Controls<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static>(
    props: ControlsProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let viewport = use_viewport_helper::<N, E>();

    let nodes_draggable = *store.nodes_draggable.read();
    let nodes_connectable = *store.nodes_connectable.read();
    let elements_selectable = *store.elements_selectable.read();
    let is_interactive = nodes_draggable || nodes_connectable || elements_selectable;
    let transform = *store.transform.read();
    let zoom = transform.scale();
    let min_zoom = *store.min_zoom.read();
    let max_zoom = *store.max_zoom.read();
    let min_zoom_reached = zoom <= min_zoom;
    let max_zoom_reached = zoom >= max_zoom;

    let orientation = props.orientation.unwrap_or_default();
    let position = props.position.unwrap_or_else(default_controls_position);

    let user_class = props.class_name.clone().unwrap_or_default();
    let class_name = format!("react-flow__controls {} {}", orientation.as_str(), user_class);

    let on_zoom_in = props.on_zoom_in;
    let on_zoom_out = props.on_zoom_out;
    let on_fit_view = props.on_fit_view;
    let on_interactive_change = props.on_interactive_change;
    let fit_view_options = props.fit_view_options.clone();

    // Capture mutable store handles for the toggle handler.
    let mut nodes_draggable_sig = store.nodes_draggable;
    let mut nodes_connectable_sig = store.nodes_connectable;
    let mut elements_selectable_sig = store.elements_selectable;

    let zoom_in_handler = move |_| {
        viewport.zoom_in(None);
        if let Some(h) = &on_zoom_in {
            h.call(());
        }
    };
    let zoom_out_handler = move |_| {
        viewport.zoom_out(None);
        if let Some(h) = &on_zoom_out {
            h.call(());
        }
    };
    let fit_view_handler = move |_| {
        viewport.fit_view(fit_view_options.as_ref().map(|rc| rc.as_ref()));
        if let Some(h) = &on_fit_view {
            h.call(());
        }
    };
    let toggle_interactivity = move |_| {
        let next = !is_interactive;
        nodes_draggable_sig.set(next);
        nodes_connectable_sig.set(next);
        elements_selectable_sig.set(next);
        if let Some(h) = &on_interactive_change {
            h.call(next);
        }
    };

    let aria_label = props
        .aria_label
        .clone()
        .unwrap_or_else(|| "React Flow controls".to_string());

    rsx! {
        Panel {
            class_name: Some(class_name),
            position: Some(position),
            style: props.style.clone(),
            "data-testid": "rf__controls",
            "aria-label": "{aria_label}",
            if props.show_zoom {
                ControlButton {
                    on_click: zoom_in_handler,
                    class_name: Some("react-flow__controls-zoomin".to_string()),
                    title: Some("zoom in".to_string()),
                    aria_label: Some("zoom in".to_string()),
                    disabled: max_zoom_reached,
                    PlusIcon {}
                }
                ControlButton {
                    on_click: zoom_out_handler,
                    class_name: Some("react-flow__controls-zoomout".to_string()),
                    title: Some("zoom out".to_string()),
                    aria_label: Some("zoom out".to_string()),
                    disabled: min_zoom_reached,
                    MinusIcon {}
                }
            }
            if props.show_fit_view {
                ControlButton {
                    class_name: Some("react-flow__controls-fitview".to_string()),
                    on_click: fit_view_handler,
                    title: Some("fit view".to_string()),
                    aria_label: Some("fit view".to_string()),
                    FitViewIcon {}
                }
            }
            if props.show_interactive {
                ControlButton {
                    class_name: Some("react-flow__controls-interactive".to_string()),
                    on_click: toggle_interactivity,
                    title: Some("toggle interactivity".to_string()),
                    aria_label: Some("toggle interactivity".to_string()),
                    if is_interactive { UnlockIcon {} } else { LockIcon {} }
                }
            }
            {props.children}
        }
    }
}
