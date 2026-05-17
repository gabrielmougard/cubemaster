//! Port of `xyflow-react/src/additional-components/MiniMap/MiniMap.tsx`.
//!
//! Status: Phase 8 — fully implemented.
//!
//! The state machine ([`XYMiniMap`]) is constructed on mount via
//! [`XYMiniMapParams`], driven by `set_viewport_constrained` /
//! `scale_to` closures that delegate to the live `PanZoomInstance`
//! stored under [`crate::store::RGraphStore::pan_zoom`].
//!
//! Pointer events on the SVG are translated to
//! [`rgraph_core::types::nodes::PointerEventLike`] and dispatched
//! through `handle_pointer_down/move/up`. The viewport indicator
//! follows the cursor while the user drags inside the minimap.

#![allow(clippy::module_name_repetitions)]

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::events::{MouseEvent, PointerData};
use dioxus::html::input_data::keyboard_types::Modifiers;
use dioxus::html::input_data::MouseButton;
use dioxus::html::point_interaction::{InteractionLocation, ModifiersInteraction, PointerInteraction};
use dioxus::prelude::*;
use dioxus_signals::ReadableExt;
use rgraph_core::types::geometry::{Rect, Transform};
use rgraph_core::types::nodes::PointerEventLike;
use rgraph_core::types::viewport::PanelPosition;
use rgraph_core::utils::general::get_bounds_of_rects;
use rgraph_core::utils::graph::{get_internal_nodes_bounds, GetInternalNodesBoundsParams};
use rgraph_core::xyminimap::{
    XYMiniMap, XYMiniMapParams, XYMiniMapUpdateParams,
};

use crate::components::panel::Panel;
use crate::context::use_rgraph_store;
use crate::store::{RGraphStore, SharedPanZoom};

use super::minimap_nodes::MiniMapNodes;
use super::types::MiniMapNodeAttr;

const DEFAULT_WIDTH: f64 = 200.0;
const DEFAULT_HEIGHT: f64 = 150.0;
const ARIA_LABEL_KEY: &str = "react-flow__minimap-desc";

#[derive(Props, Clone)]
pub struct MiniMapProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default = DEFAULT_WIDTH)]
    pub width: f64,
    #[props(default = DEFAULT_HEIGHT)]
    pub height: f64,
    #[props(default)]
    pub class_name: Option<String>,
    #[props(default)]
    pub style: Option<String>,
    #[props(default)]
    pub node_color: Option<MiniMapNodeAttr<N>>,
    #[props(default)]
    pub node_stroke_color: Option<MiniMapNodeAttr<N>>,
    #[props(default)]
    pub node_class_name: Option<MiniMapNodeAttr<N>>,
    #[props(default = 5.0)]
    pub node_border_radius: f64,
    #[props(default)]
    pub node_stroke_width: Option<f64>,
    #[props(default)]
    pub bg_color: Option<String>,
    #[props(default)]
    pub mask_color: Option<String>,
    #[props(default)]
    pub mask_stroke_color: Option<String>,
    #[props(default)]
    pub mask_stroke_width: Option<f64>,
    #[props(default)]
    pub position: Option<PanelPosition>,
    #[props(default)]
    pub on_click: Option<EventHandler<(MouseEvent, (f64, f64))>>,
    #[props(default)]
    pub on_node_click: Option<EventHandler<(MouseEvent, String)>>,
    /// Drag the minimap to pan the parent viewport.
    #[props(default = false)]
    pub pannable: bool,
    /// **Deferred** — wheel-zoom inside the minimap. Wheel events on
    /// SVG elements need additional Dioxus desktop plumbing to expose
    /// `ctrlKey` and `preventDefault`; tracked alongside the
    /// `ZoomPane` wheel bridge.
    #[props(default = false)]
    pub zoomable: bool,
    #[props(default)]
    pub aria_label: Option<String>,
    #[props(default = false)]
    pub inverse_pan: bool,
    #[props(default = 1.0)]
    pub zoom_step: f64,
    #[props(default = 5.0)]
    pub offset_scale: f64,
    #[props(default)]
    pub _types: std::marker::PhantomData<E>,
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> PartialEq for MiniMapProps<N, E> {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.class_name == other.class_name
            && self.style == other.style
            && self.node_color == other.node_color
            && self.node_stroke_color == other.node_stroke_color
            && self.node_class_name == other.node_class_name
            && self.node_border_radius == other.node_border_radius
            && self.node_stroke_width == other.node_stroke_width
            && self.bg_color == other.bg_color
            && self.mask_color == other.mask_color
            && self.mask_stroke_color == other.mask_stroke_color
            && self.mask_stroke_width == other.mask_stroke_width
            && self.position == other.position
            && self.pannable == other.pannable
            && self.zoomable == other.zoomable
            && self.aria_label == other.aria_label
            && self.inverse_pan == other.inverse_pan
            && self.zoom_step == other.zoom_step
            && self.offset_scale == other.offset_scale
    }
}

/// Build an [`XYMiniMap`] backed by the `Rc<RefCell<Box<dyn PanZoomInstance>>>`
/// stored on the rgraph store. We can't use [`XYMiniMap::from_panzoom`]
/// because that constructor requires the concrete `XYPanZoom<K>` type,
/// but the store erases it behind `dyn PanZoomInstance`. Instead we
/// build closures that route through the trait.
fn build_minimap_instance(
    pan_zoom: SharedPanZoom,
    view_scale: Rc<RefCell<f64>>,
) -> XYMiniMap {
    let pz_transform = Rc::clone(&pan_zoom);
    let pz_set = Rc::clone(&pan_zoom);
    let pz_scale = Rc::clone(&pan_zoom);
    let vs_for_minimap = Rc::clone(&view_scale);
    XYMiniMap::new(XYMiniMapParams {
        get_transform: Rc::new(move || {
            let v = pz_transform.borrow().get_viewport();
            Transform(v.x, v.y, v.zoom)
        }),
        get_view_scale: Rc::new(move || *vs_for_minimap.borrow()),
        set_viewport_constrained: Rc::new(move |viewport, extent, translate_extent| {
            let p = pz_set
                .borrow_mut()
                .set_viewport_constrained(viewport, extent, translate_extent);
            let (out, resolver) = rgraph_core::promise::channel::<bool>();
            match p.try_take() {
                Some(_) => resolver.resolve(true),
                None => resolver.resolve(false),
            }
            out
        }),
        scale_to: Rc::new(move |zoom| {
            let p = pz_scale.borrow_mut().scale_to(zoom, None);
            let (out, resolver) = rgraph_core::promise::channel::<bool>();
            match p.try_take() {
                Some(v) => resolver.resolve(v),
                None => resolver.resolve(false),
            }
            out
        }),
    })
}

#[component]
pub fn MiniMap<N: Clone + PartialEq + 'static>(
    props: MiniMapProps<N, ()>,
) -> Element {
    let store: RGraphStore<N, ()> = use_rgraph_store::<N, ()>();
    let transform = *store.transform.read();
    let flow_width = *store.width.read();
    let flow_height = *store.height.read();
    let rf_id = store.rf_id.read().clone();
    let lookup = store.node_lookup.read();
    let zoom = transform.scale();
    let safe_zoom = if zoom == 0.0 { 1.0 } else { zoom };
    let view_bb = Rect::new(
        -transform.tx() / safe_zoom,
        -transform.ty() / safe_zoom,
        flow_width / safe_zoom,
        flow_height / safe_zoom,
    );
    let bounding_rect = if !lookup.is_empty() {
        let inner = get_internal_nodes_bounds(
            &lookup,
            GetInternalNodesBoundsParams {
                filter: Some(Box::new(|n| !n.user.hidden.unwrap_or(false))),
            },
        );
        get_bounds_of_rects(inner, view_bb)
    } else {
        view_bb
    };
    drop(lookup);

    let element_width = props.width;
    let element_height = props.height;
    let scaled_width = bounding_rect.width / element_width;
    let scaled_height = bounding_rect.height / element_height;
    let view_scale = scaled_width.max(scaled_height);
    let view_width = view_scale * element_width;
    let view_height = view_scale * element_height;
    let offset = props.offset_scale * view_scale;
    let x = bounding_rect.x - (view_width - bounding_rect.width) / 2.0 - offset;
    let y = bounding_rect.y - (view_height - bounding_rect.height) / 2.0 - offset;
    let width = view_width + offset * 2.0;
    let height = view_height + offset * 2.0;
    let labelled_by = format!("{ARIA_LABEL_KEY}-{rf_id}");

    // ----- XYMiniMap instance + pointer wiring -----
    // The instance is kept in `use_hook` so it survives renders. It is
    // (re)built only when the `PanZoomInstance` becomes available
    // (i.e. once `<ZoomPane>` has mounted).
    let view_scale_ref = use_hook(|| Rc::new(RefCell::new(view_scale)));
    *view_scale_ref.borrow_mut() = view_scale;

    let minimap_slot = use_hook(|| Rc::new(RefCell::new(None::<XYMiniMap>)));
    {
        let pz_opt = store.pan_zoom.peek().clone();
        let mut slot = minimap_slot.borrow_mut();
        if slot.is_none()
            && let Some(pz) = pz_opt
        {
            *slot = Some(build_minimap_instance(pz, Rc::clone(&view_scale_ref)));
        }
    }

    // Push current update params to the state machine on every render.
    if let Some(instance) = minimap_slot.borrow().as_ref() {
        instance.update(XYMiniMapUpdateParams {
            translate_extent: *store.translate_extent.read(),
            width: flow_width,
            height: flow_height,
            inverse_pan: props.inverse_pan,
            zoom_step: props.zoom_step,
            pannable: props.pannable,
            zoomable: props.zoomable,
        });
    }

    let on_pointer_down = {
        let slot = Rc::clone(&minimap_slot);
        move |evt: Event<PointerData>| {
            if let Some(m) = slot.borrow().as_ref() {
                m.handle_pointer_down(&pointer_event_like(&evt));
            }
        }
    };
    let on_pointer_move = {
        let slot = Rc::clone(&minimap_slot);
        move |evt: Event<PointerData>| {
            if let Some(m) = slot.borrow().as_ref() {
                let _ = m.handle_pointer_move(&pointer_event_like(&evt));
            }
        }
    };
    let on_pointer_up = {
        let slot = Rc::clone(&minimap_slot);
        move |evt: Event<PointerData>| {
            if let Some(m) = slot.borrow().as_ref() {
                m.handle_pointer_up(&pointer_event_like(&evt));
            }
        }
    };
    // ------------------------------------------------

    let mut style_str = String::new();
    if let Some(c) = &props.bg_color {
        style_str.push_str(&format!("--xy-minimap-background-color-props:{c};"));
    }
    if let Some(c) = &props.mask_color {
        style_str.push_str(&format!("--xy-minimap-mask-background-color-props:{c};"));
    }
    if let Some(c) = &props.mask_stroke_color {
        style_str.push_str(&format!("--xy-minimap-mask-stroke-color-props:{c};"));
    }
    if let Some(w) = props.mask_stroke_width {
        style_str.push_str(&format!(
            "--xy-minimap-mask-stroke-width-props:{};",
            w * view_scale
        ));
    }
    if let Some(s) = &props.style {
        style_str.push_str(s);
    }

    let class_name = match &props.class_name {
        Some(extra) => format!("react-flow__minimap {extra}"),
        None => "react-flow__minimap".to_string(),
    };
    let position = props.position.unwrap_or(PanelPosition::BottomRight);
    let aria_label = props
        .aria_label
        .clone()
        .unwrap_or_else(|| "Mini Map".to_string());

    let view_box = format!("{x} {y} {width} {height}");
    let mask_path = format!(
        "M{},{}h{}v{}h{}z M{},{}h{}v{}h{}z",
        x - offset,
        y - offset,
        width + offset * 2.0,
        height + offset * 2.0,
        -(width + offset * 2.0),
        view_bb.x,
        view_bb.y,
        view_bb.width,
        view_bb.height,
        -view_bb.width,
    );

    // Forward host-supplied on_click; coords are screen-space client
    // coordinates here. Callers expecting flow coords can use the
    // `ViewportHelper::screen_to_flow_position` hook.
    let on_click = props.on_click;
    let svg_on_click = move |evt: MouseEvent| {
        if let Some(h) = &on_click {
            let coords = evt.client_coordinates();
            h.call((evt, (coords.x, coords.y)));
        }
    };

    rsx! {
        Panel {
            position: Some(position),
            class_name: Some(class_name),
            style: Some(style_str),
            "data-testid": "rf__minimap",
            svg {
                width: "{element_width}",
                height: "{element_height}",
                "viewBox": "{view_box}",
                class: "react-flow__minimap-svg",
                role: "img",
                "aria-labelledby": "{labelled_by}",
                onpointerdown: on_pointer_down,
                onpointermove: on_pointer_move,
                onpointerup: on_pointer_up,
                onclick: svg_on_click,
                title { id: "{labelled_by}", "{aria_label}" }
                MiniMapNodes::<N> {
                    on_click: props.on_node_click,
                    node_color: props.node_color.clone(),
                    node_stroke_color: props.node_stroke_color.clone(),
                    node_class_name: props.node_class_name.clone(),
                    node_border_radius: props.node_border_radius,
                    node_stroke_width: props.node_stroke_width,
                }
                path {
                    class: "react-flow__minimap-mask",
                    d: "{mask_path}",
                    "fill-rule": "evenodd",
                    "pointer-events": "none",
                }
            }
        }
    }
}

/// Translate a Dioxus pointer event into [`PointerEventLike`] consumed
/// by the [`XYMiniMap`] / [`rgraph_core::xyresizer::XYResizer`] state
/// machines.
pub(crate) fn pointer_event_like(evt: &Event<PointerData>) -> PointerEventLike {
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
