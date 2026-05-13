//! Port of `xyflow-core/src/types/panzoom.ts`.
//!
//! Status: implemented (phase 1).
//!
//! TS uses TypeScript callbacks (`(event, viewport) => void`) for the
//! `onPanZoom*` events; in Rust these are stored as boxed `FnMut`
//! values (`Send + Sync` so the params struct can be moved across
//! threads).

#![allow(clippy::module_name_repetitions)]

use crate::promise::Promise;
use crate::types::geometry::{CoordinateExtent, Rect, Transform};
use crate::types::nodes::PointerEventLike;
use crate::types::viewport::{
    EaseFn, InterpolationKind, PanOnScrollMode, ViewportHelperFunctionOptions, Viewport,
};

// ---------------------------------------------------------------------------
// Callback aliases.
// ---------------------------------------------------------------------------

/// `(dragging: bool) -> ()` — fired by [`crate::xypanzoom`] whenever
/// the dragging-or-panning flag flips.
///
/// Note: callback aliases in this module are NOT `Send + Sync`. The
/// `rgraph-zoom`/`rgraph-drag` engines and Dioxus signals are
/// single-threaded by default, so we keep these closures `!Send` to
/// allow capturing `Rc<RefCell<_>>` state.
pub type OnDraggingChange = Box<dyn FnMut(bool)>;

/// `(transform) -> ()` — fired by [`crate::xypanzoom`] every time the
/// affine transform changes (e.g. while a gesture is in progress).
pub type OnTransformChange = Box<dyn FnMut(Transform)>;

/// Fired at the start, during, and end of a pan/zoom gesture.
pub type OnPanZoom = Box<dyn FnMut(Option<&PointerEventLike>, &Viewport)>;

/// Fired when the user requests the pane's context menu (right click).
pub type OnPaneContextMenu = Box<dyn FnMut(&PointerEventLike)>;

// ---------------------------------------------------------------------------
// `pan_on_drag` — TS uses `boolean | number[]`.
// ---------------------------------------------------------------------------

/// Configuration for pan-on-drag. TS allows a boolean or a list of
/// mouse buttons (e.g. `[0, 1]`). Rust models that as an enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanOnDrag {
    /// Disabled.
    Off,
    /// Enabled with the default left mouse button.
    On,
    /// Enabled, but only when one of the listed buttons is pressed.
    Buttons(Vec<u8>),
}

impl Default for PanOnDrag {
    fn default() -> Self {
        Self::On
    }
}

impl From<bool> for PanOnDrag {
    fn from(b: bool) -> Self {
        if b {
            PanOnDrag::On
        } else {
            PanOnDrag::Off
        }
    }
}

impl From<Vec<u8>> for PanOnDrag {
    fn from(buttons: Vec<u8>) -> Self {
        PanOnDrag::Buttons(buttons)
    }
}

// ---------------------------------------------------------------------------
// Construction-time params.
// ---------------------------------------------------------------------------

/// Construction parameters for [`crate::xypanzoom::XYPanZoom`].
///
/// Note: TS takes `domNode: Element` and reads `getBoundingClientRect()`
/// internally. Per the workspace porting decision the caller measures
/// the bounding rect (Dioxus `MountedData::get_client_rect()`) and
/// passes it in via [`Self::dom_bbox`].
pub struct PanZoomParams {
    pub min_zoom: f64,
    pub max_zoom: f64,
    pub viewport: Viewport,
    pub translate_extent: CoordinateExtent,
    pub dom_bbox: Rect,
    pub on_dragging_change: OnDraggingChange,
    pub on_pan_zoom_start: Option<OnPanZoom>,
    pub on_pan_zoom: Option<OnPanZoom>,
    pub on_pan_zoom_end: Option<OnPanZoom>,
}

impl std::fmt::Debug for PanZoomParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanZoomParams")
            .field("min_zoom", &self.min_zoom)
            .field("max_zoom", &self.max_zoom)
            .field("viewport", &self.viewport)
            .field("translate_extent", &self.translate_extent)
            .field("dom_bbox", &self.dom_bbox)
            .field("on_dragging_change", &"<fn>")
            .field("on_pan_zoom_start", &self.on_pan_zoom_start.as_ref().map(|_| "<fn>"))
            .field("on_pan_zoom", &self.on_pan_zoom.as_ref().map(|_| "<fn>"))
            .field("on_pan_zoom_end", &self.on_pan_zoom_end.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Per-call options.
// ---------------------------------------------------------------------------

/// Options used by `set_viewport`, `scale_to`, `scale_by`, …
///
/// Mirrors the TS `PanZoomTransformOptions`, which is the same shape as
/// [`ViewportHelperFunctionOptions`] but defined separately in the JS
/// source — we re-export both for parity.
#[derive(Default)]
pub struct PanZoomTransformOptions {
    pub duration: Option<f64>,
    pub ease: Option<EaseFn>,
    pub interpolate: Option<InterpolationKind>,
}

impl std::fmt::Debug for PanZoomTransformOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanZoomTransformOptions")
            .field("duration", &self.duration)
            .field("ease", &self.ease.as_ref().map(|_| "<fn>"))
            .field("interpolate", &self.interpolate)
            .finish()
    }
}

impl From<ViewportHelperFunctionOptions> for PanZoomTransformOptions {
    fn from(o: ViewportHelperFunctionOptions) -> Self {
        PanZoomTransformOptions {
            duration: o.duration,
            ease: o.ease,
            interpolate: o.interpolate,
        }
    }
}

// ---------------------------------------------------------------------------
// `update()` options.
// ---------------------------------------------------------------------------

/// Run-time options reapplied to the pan/zoom instance every render
/// cycle (TS `PanZoomUpdateOptions`).
pub struct PanZoomUpdateOptions {
    /// Class added to elements that should suppress wheel handling.
    pub no_wheel_class_name: String,
    /// Class added to elements that should suppress pan handling.
    pub no_pan_class_name: String,
    pub on_pane_context_menu: Option<OnPaneContextMenu>,
    pub prevent_scrolling: bool,
    pub pan_on_scroll: bool,
    pub pan_on_drag: PanOnDrag,
    pub pan_on_scroll_mode: PanOnScrollMode,
    pub pan_on_scroll_speed: f64,
    pub user_selection_active: bool,
    pub zoom_on_pinch: bool,
    pub zoom_on_scroll: bool,
    pub zoom_on_double_click: bool,
    pub zoom_activation_key_pressed: bool,
    /// Library identifier, used for class-name prefixes (e.g. `"react"`,
    /// `"svelte"`). The Dioxus consumer should pass `"dioxus"`.
    pub lib: String,
    pub on_transform_change: OnTransformChange,
    pub connection_in_progress: bool,
    pub pane_click_distance: f64,
    pub selection_on_drag: Option<bool>,
}

impl std::fmt::Debug for PanZoomUpdateOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanZoomUpdateOptions")
            .field("no_wheel_class_name", &self.no_wheel_class_name)
            .field("no_pan_class_name", &self.no_pan_class_name)
            .field(
                "on_pane_context_menu",
                &self.on_pane_context_menu.as_ref().map(|_| "<fn>"),
            )
            .field("prevent_scrolling", &self.prevent_scrolling)
            .field("pan_on_scroll", &self.pan_on_scroll)
            .field("pan_on_drag", &self.pan_on_drag)
            .field("pan_on_scroll_mode", &self.pan_on_scroll_mode)
            .field("pan_on_scroll_speed", &self.pan_on_scroll_speed)
            .field("user_selection_active", &self.user_selection_active)
            .field("zoom_on_pinch", &self.zoom_on_pinch)
            .field("zoom_on_scroll", &self.zoom_on_scroll)
            .field("zoom_on_double_click", &self.zoom_on_double_click)
            .field("zoom_activation_key_pressed", &self.zoom_activation_key_pressed)
            .field("lib", &self.lib)
            .field("on_transform_change", &"<fn>")
            .field("connection_in_progress", &self.connection_in_progress)
            .field("pane_click_distance", &self.pane_click_distance)
            .field("selection_on_drag", &self.selection_on_drag)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Public instance trait — useful for mocking in tests.
// ---------------------------------------------------------------------------

/// Trait describing the high-level pan/zoom instance API.
///
/// Mirrors the TS `PanZoomInstance` interface. The concrete
/// implementation in [`crate::xypanzoom::XYPanZoom`] (phase 4) returns
/// real [`Promise<bool>`] / [`Promise<Option<Transform>>`] handles; this
/// trait is supplied so downstream code can be written against the
/// abstraction and mocks can be substituted in tests.
pub trait PanZoomInstance {
    fn update(&mut self, options: PanZoomUpdateOptions);
    fn destroy(&mut self);
    fn get_viewport(&self) -> Viewport;
    fn set_viewport(
        &mut self,
        viewport: Viewport,
        options: Option<PanZoomTransformOptions>,
    ) -> Promise<bool>;
    fn set_viewport_constrained(
        &mut self,
        viewport: Viewport,
        extent: CoordinateExtent,
        translate_extent: CoordinateExtent,
    ) -> Promise<Option<Transform>>;
    fn set_scale_extent(&mut self, scale_extent: (f64, f64));
    fn set_translate_extent(&mut self, translate_extent: CoordinateExtent);
    fn scale_to(&mut self, scale: f64, options: Option<PanZoomTransformOptions>) -> Promise<bool>;
    fn scale_by(&mut self, factor: f64, options: Option<PanZoomTransformOptions>) -> Promise<bool>;
    fn sync_viewport(&mut self, viewport: Viewport);
    fn set_click_distance(&mut self, distance: f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pan_on_drag_from_bool() {
        assert_eq!(PanOnDrag::from(true), PanOnDrag::On);
        assert_eq!(PanOnDrag::from(false), PanOnDrag::Off);
        assert_eq!(PanOnDrag::default(), PanOnDrag::On);
    }

    #[test]
    fn pan_on_drag_from_buttons() {
        let pd: PanOnDrag = vec![0u8, 1u8].into();
        assert_eq!(pd, PanOnDrag::Buttons(vec![0, 1]));
    }

    #[test]
    fn options_conversion_keeps_fields() {
        let mut o = ViewportHelperFunctionOptions::default();
        o.duration = Some(500.0);
        o.interpolate = Some(InterpolationKind::Linear);
        let p: PanZoomTransformOptions = o.into();
        assert_eq!(p.duration, Some(500.0));
        assert_eq!(p.interpolate, Some(InterpolationKind::Linear));
    }
}
