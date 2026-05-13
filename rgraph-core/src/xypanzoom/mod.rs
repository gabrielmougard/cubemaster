//! Port of `xyflow-core/src/xypanzoom/XYPanZoom.ts` — viewport pan/zoom
//! manager.
//!
//! Wraps [`rgraph_zoom::ZoomBehavior`] and provides the high-level API
//! React Flow / Svelte Flow expose to their stores.
//!
//! Status: implemented (phase 4).
//!
//! ## API shape
//!
//! The TS `XYPanZoom({...})` factory becomes [`XYPanZoom::new`]. Methods
//! such as `setViewport`, `scaleBy`, `scaleTo`, `setViewportConstrained`
//! are provided as inherent methods returning [`Promise<bool>`] (or
//! `Promise<Option<Transform>>` where the TS variant returned a
//! `ZoomTransform | undefined`).
//!
//! ## Differences vs TS
//!
//! * TS attaches a d3 `wheel.zoom`, `dblclick.zoom` handler to the
//!   provided DOM node. The Rust port does not; instead the Dioxus
//!   consumer wires those events and feeds them to the
//!   [`XYPanZoom::handle_wheel`], [`XYPanZoom::handle_dblclick`], and
//!   [`XYPanZoom::handle_pointer`] forwarders, which honour the same
//!   filter / event-handler logic.
//! * Animated transitions go through
//!   [`rgraph_transition::TransitionEngine`]. The consumer provides the
//!   engine (typically one per app); this struct holds a reference but
//!   does not own it.
//!
//! See the inner submodule docs (`utils`, `filter`, `eventhandler`) for
//! lower-level helpers.

#![allow(clippy::module_name_repetitions)]

pub mod eventhandler;
pub mod filter;
pub mod utils;

use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

use rgraph_transition::TransitionEngine;
use rgraph_zoom::{
    DoubleClickInput, Extent, PointerId as ZoomPointerId, PointerInput,
    Transform as ZoomTransform, WheelInput, ZoomBehavior,
};

use crate::promise::{channel, Promise};
use crate::types::geometry::{CoordinateExtent, Rect, Transform};
use crate::types::nodes::PointerEventLike;
use crate::types::panzoom::{
    PanZoomParams, PanZoomTransformOptions, PanZoomUpdateOptions,
};
use crate::types::viewport::Viewport;
use crate::utils::general::is_numeric;
use crate::xypanzoom::eventhandler::{shared_zoom_pan_values, SharedZoomPanValues};
use crate::xypanzoom::filter::{FilterDecision, FilterEvent};
use crate::xypanzoom::utils::{transform_to_viewport, viewport_to_transform};

// ---------------------------------------------------------------------------
// Result aliases
// ---------------------------------------------------------------------------

/// `Promise<bool>` returned by animated viewport-change methods.
pub type ViewportChangePromise = Promise<bool>;

/// `Promise<Option<Transform>>` returned by `set_viewport_constrained`,
/// matching the TS shape `Promise<ZoomTransform | undefined>`.
pub type ConstrainedTransformPromise = Promise<Option<Transform>>;

type CachedFilter = Box<dyn Fn(&FilterEvent<'_>) -> FilterDecision>;

// ---------------------------------------------------------------------------
// XYPanZoom
// ---------------------------------------------------------------------------

/// Pan/zoom manager wrapping [`rgraph_zoom::ZoomBehavior`].
///
/// Generic over a `K` target key (defaults to `()` for single-pane
/// apps). The behaviour is shared via `Rc` so cloning is cheap.
pub struct XYPanZoom<K: Hash + Eq + Clone + 'static = ()> {
    inner: Rc<XYPanZoomInner<K>>,
}

impl<K: Hash + Eq + Clone + 'static> Clone for XYPanZoom<K> {
    fn clone(&self) -> Self {
        XYPanZoom {
            inner: Rc::clone(&self.inner),
        }
    }
}

struct XYPanZoomInner<K: Hash + Eq + Clone + 'static> {
    zoom: ZoomBehavior<K, ()>,
    transitions: RefCell<Option<TransitionEngine<K>>>,
    target: K,
    dom_bbox: RefCell<Rect>,
    /// Shared zoom-pan bookkeeping (start/zoom/end glue with eventhandler).
    #[allow(dead_code)]
    zoom_pan_values: SharedZoomPanValues,
    /// Cached filter from the most recent `update` call.
    cached_filter: RefCell<Option<CachedFilter>>,
    destroyed: RefCell<bool>,
}

impl<K: Hash + Eq + Clone + 'static> XYPanZoom<K> {
    /// Construct a new `XYPanZoom` for `target`.
    ///
    /// The TS counterpart takes a `domNode` and reads its bounding
    /// rect. The Rust port takes the bounding rect directly via
    /// [`PanZoomParams::dom_bbox`]. The initial viewport is clamped to
    /// `[min_zoom, max_zoom]` and constrained by `translate_extent`.
    pub fn new(target: K, params: PanZoomParams) -> Self {
        let zoom = ZoomBehavior::<K, ()>::new();
        zoom.scale_extent(params.min_zoom, params.max_zoom);
        zoom.translate_extent(
            [params.translate_extent[0][0], params.translate_extent[0][1]],
            [params.translate_extent[1][0], params.translate_extent[1][1]],
        );

        // d3-zoom default extent is `(0, 0)` to `(width, height)`.
        let bbox = params.dom_bbox;
        zoom.extent_const(Extent::new([0.0, 0.0], [bbox.width, bbox.height]));

        // Apply initial viewport (clamped to scale extent).
        let scale_ext = zoom.get_scale_extent();
        let clamped_zoom = params.viewport.zoom.clamp(scale_ext[0], scale_ext[1]);
        let initial = Viewport {
            x: params.viewport.x,
            y: params.viewport.y,
            zoom: clamped_zoom,
        };
        zoom.set_transform(target.clone(), viewport_to_transform(initial), None);

        // The `on_dragging_change` etc. callbacks supplied through
        // PanZoomParams are not yet wired here — they are reapplied
        // each `update()` call in the React/Svelte counterparts. We
        // drop them after applying the initial viewport.
        let _ = (
            params.on_pan_zoom_start,
            params.on_pan_zoom,
            params.on_pan_zoom_end,
            params.on_dragging_change,
        );

        XYPanZoom {
            inner: Rc::new(XYPanZoomInner {
                zoom,
                transitions: RefCell::new(None),
                target,
                dom_bbox: RefCell::new(bbox),
                zoom_pan_values: shared_zoom_pan_values(),
                cached_filter: RefCell::new(None),
                destroyed: RefCell::new(false),
            }),
        }
    }

    /// Inject (or replace) the [`TransitionEngine`] used by animated
    /// viewport-change methods.
    pub fn with_transition_engine(self, engine: TransitionEngine<K>) -> Self {
        *self.inner.transitions.borrow_mut() = Some(engine);
        self
    }

    /// Replace the cached DOM bbox (e.g. after a resize). Updates the
    /// underlying zoom extent.
    pub fn set_dom_bbox(&self, bbox: Rect) {
        *self.inner.dom_bbox.borrow_mut() = bbox;
        self.inner
            .zoom
            .extent_const(Extent::new([0.0, 0.0], [bbox.width, bbox.height]));
    }

    /// Borrow a clone of the underlying [`ZoomBehavior`] for advanced
    /// listener installation.
    pub fn zoom(&self) -> ZoomBehavior<K, ()> {
        self.inner.zoom.clone()
    }

    /// Returns the currently-cached [`Viewport`].
    #[must_use]
    pub fn get_viewport(&self) -> Viewport {
        transform_to_viewport(self.inner.zoom.transform(&self.inner.target))
    }

    /// Mirrors the TS `update()` call. Reapplies all run-time options:
    /// click distance, filter, etc.
    pub fn update(&self, opts: &PanZoomUpdateOptions) {
        if *self.inner.destroyed.borrow() {
            return;
        }

        let pan_on_drag = opts.pan_on_drag.clone();
        let f = filter::create_filter(filter::CreateFilterParams {
            zoom_activation_key_pressed: opts.zoom_activation_key_pressed,
            zoom_on_scroll: opts.zoom_on_scroll,
            zoom_on_pinch: opts.zoom_on_pinch,
            pan_on_drag: pan_on_drag.clone(),
            pan_on_scroll: opts.pan_on_scroll,
            zoom_on_double_click: opts.zoom_on_double_click,
            user_selection_active: opts.user_selection_active,
            no_wheel_class_name: opts.no_wheel_class_name.clone(),
            no_pan_class_name: opts.no_pan_class_name.clone(),
            lib: opts.lib.clone(),
            connection_in_progress: opts.connection_in_progress,
        });
        // The filter from `create_filter` is `Send + Sync`, but we
        // store it as a non-Send `Box<dyn Fn>` to keep
        // `XYPanZoomInner: !Send`. The relaxation is one-directional.
        let f_local: CachedFilter = Box::new(move |evt: &FilterEvent<'_>| f(evt));
        *self.inner.cached_filter.borrow_mut() = Some(f_local);

        let click_distance = if opts.selection_on_drag.unwrap_or(false) {
            f64::INFINITY
        } else if !is_numeric(opts.pane_click_distance) || opts.pane_click_distance < 0.0 {
            0.0
        } else {
            opts.pane_click_distance
        };
        self.inner.zoom.click_distance(click_distance);
    }

    /// Apply the most-recently-installed filter to a candidate event.
    /// The Dioxus consumer calls this before forwarding wheel /
    /// mousedown / touchstart events. Returns
    /// [`FilterDecision::Accept`] when no filter has been installed.
    pub fn filter_event(&self, event: &FilterEvent<'_>) -> FilterDecision {
        let f = self.inner.cached_filter.borrow();
        match &*f {
            Some(closure) => closure(event),
            None => FilterDecision::Accept,
        }
    }

    /// Detach all listeners. Subsequent `update` calls are no-ops.
    pub fn destroy(&self) {
        *self.inner.destroyed.borrow_mut() = true;
        self.inner.zoom.on("zoom", None);
        self.inner.zoom.on("start", None);
        self.inner.zoom.on("end", None);
    }

    // -----------------------------------------------------------------
    // Programmatic transform setters
    // -----------------------------------------------------------------

    /// Replace the current viewport with `viewport`. Animates if the
    /// caller supplies `options.duration > 0` AND a transition engine
    /// has been registered.
    #[must_use]
    pub fn set_viewport(
        &self,
        viewport: Viewport,
        options: Option<PanZoomTransformOptions>,
    ) -> ViewportChangePromise {
        self.set_transform_internal(viewport_to_transform(viewport), options)
    }

    /// Apply `viewport` constrained by an extent (typically the dom
    /// bbox) and the configured `translate_extent`.
    #[must_use]
    pub fn set_viewport_constrained(
        &self,
        viewport: Viewport,
        extent: CoordinateExtent,
        translate_extent: CoordinateExtent,
    ) -> ConstrainedTransformPromise {
        let next = viewport_to_transform(viewport);
        let constrained = rgraph_zoom::default_constrain(
            next,
            Extent::new([extent[0][0], extent[0][1]], [extent[1][0], extent[1][1]]),
            translate_extent,
        );
        self.inner
            .zoom
            .set_transform(self.inner.target.clone(), constrained, None);
        let geometry = Transform(constrained.x, constrained.y, constrained.k);
        Promise::resolved(Some(geometry))
    }

    /// Sync the engine's stored transform to `viewport`.
    pub fn sync_viewport(&self, viewport: Viewport) {
        self.inner.zoom.set_transform(
            self.inner.target.clone(),
            viewport_to_transform(viewport),
            None,
        );
    }

    /// Multiplicatively scale by `factor` at the extent centroid.
    #[must_use]
    pub fn scale_by(
        &self,
        factor: f64,
        options: Option<PanZoomTransformOptions>,
    ) -> ViewportChangePromise {
        let t0 = self.inner.zoom.transform(&self.inner.target);
        let target_t = scale_by_centroid(t0, factor, &self.inner);
        self.set_transform_internal(target_t, options)
    }

    /// Set absolute scale at the extent centroid.
    #[must_use]
    pub fn scale_to(
        &self,
        scale: f64,
        options: Option<PanZoomTransformOptions>,
    ) -> ViewportChangePromise {
        let t0 = self.inner.zoom.transform(&self.inner.target);
        let target_t = scale_to_centroid(t0, scale, &self.inner);
        self.set_transform_internal(target_t, options)
    }

    /// Replace the engine's scale extent.
    pub fn set_scale_extent(&self, scale_extent: (f64, f64)) {
        self.inner.zoom.scale_extent(scale_extent.0, scale_extent.1);
    }

    /// Replace the engine's translate extent.
    pub fn set_translate_extent(&self, translate_extent: CoordinateExtent) {
        self.inner.zoom.translate_extent(
            [translate_extent[0][0], translate_extent[0][1]],
            [translate_extent[1][0], translate_extent[1][1]],
        );
    }

    /// Replace the click-distance threshold.
    pub fn set_click_distance(&self, distance: f64) {
        let valid = if !is_numeric(distance) || distance < 0.0 {
            0.0
        } else {
            distance
        };
        self.inner.zoom.click_distance(valid);
    }

    // -----------------------------------------------------------------
    // Pointer / wheel / dblclick forwarders
    // -----------------------------------------------------------------

    /// Forward a wheel event to the engine.
    pub fn handle_wheel(&self, w: WheelInput) -> bool {
        self.inner.zoom.handle_wheel(self.inner.target.clone(), w)
    }

    /// Forward a double-click event.
    pub fn handle_dblclick(&self, evt: DoubleClickInput) {
        let engine = self.inner.transitions.borrow();
        let _ = self.inner.zoom.handle_dblclick(
            self.inner.target.clone(),
            evt,
            engine.as_ref(),
            None,
        );
    }

    /// Forward a pointer event. Returns `true` if the engine consumed it.
    pub fn handle_pointer(&self, input: PointerInput<()>) -> bool {
        self.inner.zoom.handle_pointer(self.inner.target.clone(), input)
    }

    /// Convenience: forward a pointer-down derived from a Dioxus mouse
    /// or touch event.
    pub fn handle_pointer_down(
        &self,
        id: ZoomPointerId,
        event: &PointerEventLike,
        button: u8,
        ctrl: bool,
    ) -> bool {
        self.handle_pointer(PointerInput::Down {
            id,
            x: event.client_x,
            y: event.client_y,
            button,
            ctrl,
            datum: None,
        })
    }

    // -----------------------------------------------------------------
    // Convenience: callback installation
    // -----------------------------------------------------------------

    /// Install a `start`/`zoom`/`end` listener on the underlying zoom
    /// behaviour.
    pub fn on_zoom_event<F>(&self, type_: &str, callback: F)
    where
        F: Fn(&rgraph_zoom::ZoomEvent<K, ()>) + 'static,
    {
        self.inner.zoom.on(type_, Some(Rc::new(callback)));
    }

    // -----------------------------------------------------------------
    // Private: transform pipeline
    // -----------------------------------------------------------------

    fn set_transform_internal(
        &self,
        target_transform: ZoomTransform,
        options: Option<PanZoomTransformOptions>,
    ) -> Promise<bool> {
        let opts = options.unwrap_or_default();
        let duration = opts.duration.unwrap_or(0.0);

        let engine = self.inner.transitions.borrow();
        let Some(engine) = engine.as_ref() else {
            self.inner.zoom.set_transform(
                self.inner.target.clone(),
                target_transform,
                None,
            );
            return Promise::resolved(true);
        };
        if duration <= 0.0 {
            self.inner.zoom.set_transform(
                self.inner.target.clone(),
                target_transform,
                None,
            );
            return Promise::resolved(true);
        }

        let (promise, resolver) = channel::<bool>();
        let id = self.inner.zoom.transition_to(
            engine,
            self.inner.target.clone(),
            target_transform,
            None,
            None,
        );
        engine.duration(&self.inner.target, id, duration);

        let resolver_cell = Rc::new(RefCell::new(Some(resolver)));
        let resolver_clone = Rc::clone(&resolver_cell);
        engine.on(&self.inner.target, id, "end", move |_ctx| {
            if let Some(r) = resolver_clone.borrow_mut().take() {
                r.resolve(true);
            }
        });
        promise
    }
}

// ---------------------------------------------------------------------------
// Convenience for the common `K = ()` single-pane case
// ---------------------------------------------------------------------------

impl XYPanZoom<()> {
    /// Construct an `XYPanZoom` for the single-pane case
    /// (`K = ()`) without manually specifying a target.
    pub fn new_single(params: PanZoomParams) -> Self {
        Self::new((), params)
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

fn scale_to_centroid<K: Hash + Eq + Clone + 'static>(
    t0: ZoomTransform,
    k: f64,
    inner: &XYPanZoomInner<K>,
) -> ZoomTransform {
    let bbox = *inner.dom_bbox.borrow();
    let centroid = [bbox.width / 2.0, bbox.height / 2.0];
    let p1 = t0.invert(centroid);
    let scale_extent = inner.zoom.get_scale_extent();
    let new_k = k.clamp(scale_extent[0], scale_extent[1]);
    let scaled = ZoomTransform::new(new_k, t0.x, t0.y);
    let x = centroid[0] - p1[0] * scaled.k;
    let y = centroid[1] - p1[1] * scaled.k;
    ZoomTransform::new(scaled.k, x, y)
}

fn scale_by_centroid<K: Hash + Eq + Clone + 'static>(
    t0: ZoomTransform,
    factor: f64,
    inner: &XYPanZoomInner<K>,
) -> ZoomTransform {
    scale_to_centroid(t0, t0.k * factor, inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params(width: f64, height: f64) -> PanZoomParams {
        PanZoomParams {
            min_zoom: 0.5,
            max_zoom: 2.0,
            viewport: Viewport::IDENTITY,
            translate_extent: [
                [f64::NEG_INFINITY, f64::NEG_INFINITY],
                [f64::INFINITY, f64::INFINITY],
            ],
            dom_bbox: Rect::new(0.0, 0.0, width, height),
            on_dragging_change: Box::new(|_| {}),
            on_pan_zoom_start: None,
            on_pan_zoom: None,
            on_pan_zoom_end: None,
        }
    }

    #[test]
    fn new_initialises_to_supplied_viewport() {
        let p = default_params(800.0, 600.0);
        let pz = XYPanZoom::<()>::new_single(p);
        assert_eq!(pz.get_viewport(), Viewport::IDENTITY);
    }

    #[test]
    fn new_clamps_initial_zoom_to_scale_extent() {
        let mut p = default_params(800.0, 600.0);
        p.viewport.zoom = 100.0; // way above max_zoom
        let pz = XYPanZoom::<()>::new_single(p);
        assert!((pz.get_viewport().zoom - 2.0).abs() < 1e-9);
    }

    #[test]
    fn set_viewport_synchronously_updates() {
        let pz = XYPanZoom::<()>::new_single(default_params(800.0, 600.0));
        let p = pz.set_viewport(
            Viewport {
                x: 50.0,
                y: 100.0,
                zoom: 1.5,
            },
            None,
        );
        assert_eq!(p.try_take(), Some(true));
        let v = pz.get_viewport();
        assert!((v.x - 50.0).abs() < 1e-9);
        assert!((v.y - 100.0).abs() < 1e-9);
        assert!((v.zoom - 1.5).abs() < 1e-9);
    }

    #[test]
    fn scale_to_clamps_to_extent() {
        let pz = XYPanZoom::<()>::new_single(default_params(100.0, 100.0));
        let _ = pz.scale_to(100.0, None);
        assert!((pz.get_viewport().zoom - 2.0).abs() < 1e-9);
        let _ = pz.scale_to(0.001, None);
        assert!((pz.get_viewport().zoom - 0.5).abs() < 1e-9);
    }

    #[test]
    fn scale_by_zooms_at_centroid() {
        let pz = XYPanZoom::<()>::new_single(default_params(100.0, 100.0));
        // Initial zoom = 1, scale_by(2) at centroid (50, 50).
        // Resulting transform: k=2, x = 50 - 50*2 = -50.
        let _ = pz.scale_by(2.0, None);
        let v = pz.get_viewport();
        assert!((v.zoom - 2.0).abs() < 1e-9);
        assert!((v.x - (-50.0)).abs() < 1e-9);
        assert!((v.y - (-50.0)).abs() < 1e-9);
    }

    #[test]
    fn set_viewport_constrained_returns_constrained_transform() {
        let pz = XYPanZoom::<()>::new_single(default_params(100.0, 100.0));
        let promise = pz.set_viewport_constrained(
            Viewport {
                x: 0.0,
                y: 0.0,
                zoom: 1.0,
            },
            [[0.0, 0.0], [100.0, 100.0]],
            [[f64::NEG_INFINITY, f64::NEG_INFINITY], [f64::INFINITY, f64::INFINITY]],
        );
        let value = promise.try_take().expect("resolved");
        assert!(value.is_some());
        let t = value.unwrap();
        assert!((t.scale() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn destroy_disables_update() {
        let pz = XYPanZoom::<()>::new_single(default_params(100.0, 100.0));
        pz.destroy();
        // After destroy, update is a no-op. Just verify it doesn't
        // panic.
        let opts = PanZoomUpdateOptions {
            no_wheel_class_name: "nowheel".into(),
            no_pan_class_name: "nopan".into(),
            on_pane_context_menu: None,
            prevent_scrolling: true,
            pan_on_scroll: false,
            pan_on_drag: crate::types::panzoom::PanOnDrag::On,
            pan_on_scroll_mode: crate::types::viewport::PanOnScrollMode::Free,
            pan_on_scroll_speed: 1.0,
            user_selection_active: false,
            zoom_on_pinch: true,
            zoom_on_scroll: true,
            zoom_on_double_click: true,
            zoom_activation_key_pressed: false,
            lib: "react".into(),
            on_transform_change: Box::new(|_| {}),
            connection_in_progress: false,
            pane_click_distance: 0.0,
            selection_on_drag: None,
        };
        pz.update(&opts);
        // No filter installed.
        let evt = FilterEvent {
            kind: filter::FilterEventKind::MouseDown,
            button: 0,
            ctrl_key: false,
            touches_len: 0,
            ancestor_classes: &[],
        };
        // After destroy, cached filter was never installed → Accept.
        assert_eq!(pz.filter_event(&evt), FilterDecision::Accept);
    }

    #[test]
    fn update_installs_filter_that_can_reject_user_selection() {
        let pz = XYPanZoom::<()>::new_single(default_params(100.0, 100.0));
        let opts = PanZoomUpdateOptions {
            no_wheel_class_name: "nowheel".into(),
            no_pan_class_name: "nopan".into(),
            on_pane_context_menu: None,
            prevent_scrolling: true,
            pan_on_scroll: false,
            pan_on_drag: crate::types::panzoom::PanOnDrag::On,
            pan_on_scroll_mode: crate::types::viewport::PanOnScrollMode::Free,
            pan_on_scroll_speed: 1.0,
            user_selection_active: true, // <-- the gate
            zoom_on_pinch: true,
            zoom_on_scroll: true,
            zoom_on_double_click: true,
            zoom_activation_key_pressed: false,
            lib: "react".into(),
            on_transform_change: Box::new(|_| {}),
            connection_in_progress: false,
            pane_click_distance: 0.0,
            selection_on_drag: None,
        };
        pz.update(&opts);
        let evt = FilterEvent {
            kind: filter::FilterEventKind::MouseDown,
            button: 0,
            ctrl_key: false,
            touches_len: 0,
            ancestor_classes: &[],
        };
        assert_eq!(pz.filter_event(&evt), FilterDecision::Reject);
    }

    #[test]
    fn handle_wheel_zooms_in() {
        let pz = XYPanZoom::<()>::new_single(default_params(100.0, 100.0));
        // Wheel up (delta_y < 0) → zoom in.
        let consumed = pz.handle_wheel(WheelInput {
            delta_y: -100.0,
            delta_mode: 0,
            ctrl: false,
            x: 50.0,
            y: 50.0,
        });
        assert!(consumed);
        assert!(pz.get_viewport().zoom > 1.0);
    }

    #[test]
    fn handle_pointer_pan_changes_viewport() {
        let pz = XYPanZoom::<()>::new_single(default_params(100.0, 100.0));
        let target_dn = pz.handle_pointer_down(
            ZoomPointerId::Mouse,
            &PointerEventLike {
                client_x: 50.0,
                client_y: 50.0,
                ..Default::default()
            },
            0,
            false,
        );
        assert!(target_dn);
        let _ = pz.handle_pointer(PointerInput::Move {
            id: ZoomPointerId::Mouse,
            x: 60.0,
            y: 70.0,
        });
        let _ = pz.handle_pointer(PointerInput::Up {
            id: ZoomPointerId::Mouse,
            x: 60.0,
            y: 70.0,
        });
        let v = pz.get_viewport();
        assert!((v.x - 10.0).abs() < 1e-9);
        assert!((v.y - 20.0).abs() < 1e-9);
    }

    #[test]
    fn set_dom_bbox_updates_extent() {
        let pz = XYPanZoom::<()>::new_single(default_params(100.0, 100.0));
        pz.set_dom_bbox(Rect::new(0.0, 0.0, 200.0, 150.0));
        // Just verify we can resize without panicking; and that
        // scale_to (which reads the centroid) uses the new bbox.
        let _ = pz.scale_to(2.0, None);
        let v = pz.get_viewport();
        // centroid = (100, 75); new transform: k=2, x = 100 - 100*2 = -100,
        // y = 75 - 75*2 = -75.
        assert!((v.x - (-100.0)).abs() < 1e-9);
        assert!((v.y - (-75.0)).abs() < 1e-9);
    }

    #[test]
    fn on_zoom_event_callback_fires() {
        let pz = XYPanZoom::<()>::new_single(default_params(100.0, 100.0));
        let counter = Rc::new(RefCell::new(0u32));
        let cc = Rc::clone(&counter);
        pz.on_zoom_event("zoom", move |_| {
            *cc.borrow_mut() += 1;
        });
        let _ = pz.scale_to(1.5, None);
        assert!(*counter.borrow() >= 1);
    }
}
