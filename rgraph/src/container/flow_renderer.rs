//! Port of `xyflow-react/src/container/FlowRenderer/index.tsx`.
//!
//! Status: Phase 7 — implemented.
//!
//! `FlowRenderer` is the bridge between [`crate::container::zoom_pane::ZoomPane`]
//! and [`crate::container::pane::Pane`]. It wires up the global key
//! handlers (delete, multi-selection), resolves the
//! `panOnDrag`/`selectionOnDrag`/`selectionKeyPressed` interactions
//! the same way the TS source does (lines 81–85), and conditionally
//! renders `<NodesSelection>` when `nodes_selection_active` is on.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::panzoom::PanOnDrag;
use rgraph_core::types::viewport::{KeyCode, PanOnScrollMode, SelectionMode, Viewport};

use crate::components::nodes_selection::NodesSelection;
use crate::container::pane::Pane;
use crate::container::zoom_pane::ZoomPane;
use crate::context::use_rgraph_store;
use crate::hooks::use_global_key_handler::use_global_key_handler;
use crate::hooks::use_key_press::{use_key_press, UseKeyPressOptions};
use crate::store::RGraphStore;
use crate::types::component_props::{OnPaneScroll, OnViewportChange, PaneMouseHandler};

#[derive(Props, Clone, PartialEq)]
pub struct FlowRendererProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    pub id: String,

    // Pane handlers.
    #[props(default)]
    pub on_pane_click: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_mouse_enter: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_mouse_move: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_mouse_leave: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_context_menu: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_scroll: Option<OnPaneScroll>,
    #[props(default = 1.0)]
    pub pane_click_distance: f64,

    // Keyboard / selection.
    #[props(default)]
    pub delete_key_code: Option<KeyCode>,
    #[props(default)]
    pub selection_key_code: Option<KeyCode>,
    #[props(default)]
    pub selection_on_drag: bool,
    #[props(default = SelectionMode::Full)]
    pub selection_mode: SelectionMode,
    #[props(default)]
    pub multi_selection_key_code: Option<KeyCode>,
    #[props(default)]
    pub pan_activation_key_code: Option<KeyCode>,
    #[props(default)]
    pub zoom_activation_key_code: Option<KeyCode>,

    // Pan / zoom.
    #[props(default = true)]
    pub zoom_on_scroll: bool,
    #[props(default = true)]
    pub zoom_on_pinch: bool,
    #[props(default = true)]
    pub zoom_on_double_click: bool,
    #[props(default)]
    pub pan_on_scroll: bool,
    #[props(default = 0.5)]
    pub pan_on_scroll_speed: f64,
    #[props(default)]
    pub pan_on_scroll_mode: PanOnScrollMode,
    #[props(default = PanOnDrag::On)]
    pub pan_on_drag: PanOnDrag,
    #[props(default = true)]
    pub auto_pan_on_selection: bool,
    #[props(default)]
    pub default_viewport: Viewport,
    #[props(default = 0.5)]
    pub min_zoom: f64,
    #[props(default = 2.0)]
    pub max_zoom: f64,
    #[props(default = true)]
    pub prevent_scrolling: bool,

    // Class names.
    #[props(default = "nodrag".to_string())]
    pub no_drag_class_name: String,
    #[props(default = "nowheel".to_string())]
    pub no_wheel_class_name: String,
    #[props(default = "nopan".to_string())]
    pub no_pan_class_name: String,

    // Misc.
    #[props(default)]
    pub disable_keyboard_a11y: bool,
    #[props(default)]
    pub on_viewport_change: Option<OnViewportChange>,
    #[props(default)]
    pub is_controlled_viewport: bool,

    pub children: Element,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn FlowRenderer<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: FlowRendererProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();

    let user_selection_active = *store.user_selection_active.read();
    let nodes_selection_active = *store.nodes_selection_active.read();

    let selection_key = use_key_press(props.selection_key_code.clone(), UseKeyPressOptions::default());
    let pan_activation_key = use_key_press(
        props.pan_activation_key_code.clone(),
        UseKeyPressOptions::default(),
    );

    // Global delete + multi-selection wiring.
    let _global = use_global_key_handler::<N, E>(
        props.delete_key_code.clone(),
        props.multi_selection_key_code.clone(),
    );

    let selection_key_pressed = *selection_key.pressed.read();
    let pan_key_pressed = *pan_activation_key.pressed.read();

    // The TS source coerces `panOnDrag = panActivationKeyPressed || _panOnDrag`
    // — i.e. the pan-activation key forces drag-to-pan on. We mirror it
    // by overriding the variant to `PanOnDrag::On` while the key is held.
    let effective_pan_on_drag = if pan_key_pressed {
        PanOnDrag::On
    } else {
        props.pan_on_drag.clone()
    };
    let effective_pan_on_scroll = pan_key_pressed || props.pan_on_scroll;
    let selection_on_drag_resolved = props.selection_on_drag && !matches!(effective_pan_on_drag, PanOnDrag::On);
    let is_selecting = selection_key_pressed || user_selection_active || selection_on_drag_resolved;

    // ZoomPane's pan-on-drag is suppressed while the selection key is
    // pressed (TS line 98).
    let zoom_pane_pan_on_drag = if selection_key_pressed {
        PanOnDrag::Off
    } else {
        effective_pan_on_drag.clone()
    };

    rsx! {
        ZoomPane::<N, E> {
            id: props.id.clone(),
            zoom_on_scroll: props.zoom_on_scroll,
            zoom_on_pinch: props.zoom_on_pinch,
            zoom_on_double_click: props.zoom_on_double_click,
            pan_on_scroll: effective_pan_on_scroll,
            pan_on_scroll_speed: props.pan_on_scroll_speed,
            pan_on_scroll_mode: props.pan_on_scroll_mode,
            pan_on_drag: zoom_pane_pan_on_drag,
            default_viewport: props.default_viewport,
            min_zoom: props.min_zoom,
            max_zoom: props.max_zoom,
            zoom_activation_key_code: props.zoom_activation_key_code.clone(),
            prevent_scrolling: props.prevent_scrolling,
            no_wheel_class_name: props.no_wheel_class_name.clone(),
            no_pan_class_name: props.no_pan_class_name.clone(),
            on_viewport_change: props.on_viewport_change,
            is_controlled_viewport: props.is_controlled_viewport,
            pane_click_distance: props.pane_click_distance,
            selection_on_drag: Some(selection_on_drag_resolved),
            Pane::<N, E> {
                is_selecting,
                selection_key_pressed,
                selection_mode: Some(props.selection_mode),
                pan_on_drag: Some(effective_pan_on_drag),
                auto_pan_on_selection: Some(props.auto_pan_on_selection),
                pane_click_distance: Some(props.pane_click_distance),
                selection_on_drag: Some(selection_on_drag_resolved),
                on_pane_click: props.on_pane_click,
                on_pane_context_menu: props.on_pane_context_menu,
                on_pane_scroll: props.on_pane_scroll,
                on_pane_mouse_enter: props.on_pane_mouse_enter,
                on_pane_mouse_move: props.on_pane_mouse_move,
                on_pane_mouse_leave: props.on_pane_mouse_leave,
                {props.children}
                if nodes_selection_active {
                    NodesSelection::<N, E> {
                        no_pan_class_name: props.no_pan_class_name.clone(),
                        disable_keyboard_a11y: props.disable_keyboard_a11y,
                    }
                }
            }
        }
    }
}
