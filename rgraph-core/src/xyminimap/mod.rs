//! Port of `xyflow-core/src/xyminimap/index.ts` — minimap viewport
//! interaction.
//!
//! Status: implemented (phase 8).
//!
//! The TS source attaches `d3-zoom` to the minimap SVG; here we
//! re-implement the math directly because the minimap doesn't need
//! d3-zoom's full pointer state machine — just pan-on-drag and
//! zoom-on-wheel, both translated back to the parent
//! [`crate::xypanzoom::XYPanZoom`] via its [`PanByFn`] /
//! `set_viewport_constrained` API.

#![allow(clippy::module_name_repetitions)]

use std::cell::RefCell;
use std::rc::Rc;

use crate::promise::Promise;
use crate::types::geometry::{CoordinateExtent, Transform, XYPosition};
use crate::types::nodes::PointerEventLike;
use crate::types::viewport::Viewport;
use crate::xypanzoom::utils::{xy_wheel_delta, WheelEventLike};

// ---------------------------------------------------------------------------
// Public accessor / callback aliases
// ---------------------------------------------------------------------------

/// Closure that returns the *parent* viewport's current transform.
pub type GetTransformFn = Rc<dyn Fn() -> Transform>;

/// Closure that returns the parent viewport's view-scale (the ratio of
/// minimap pixels to flow-coordinate pixels). The TS source reads it
/// from the React store; the consumer typically computes it as
/// `minimap_width / pane_width`.
pub type GetViewScaleFn = Rc<dyn Fn() -> f64>;

/// Closure that applies a constrained viewport to the parent. Mirrors
/// the `setViewportConstrained` method on `PanZoomInstance` but
/// returns only a `Promise<bool>` since the minimap doesn't need the
/// resolved transform.
pub type SetViewportConstrainedFn =
    Rc<dyn Fn(Viewport, CoordinateExtent, CoordinateExtent) -> Promise<bool>>;

/// Closure that applies an absolute scale. Mirrors `XYPanZoom::scale_to`.
pub type ScaleToFn = Rc<dyn Fn(f64) -> Promise<bool>>;

// ---------------------------------------------------------------------------
// Construction parameters
// ---------------------------------------------------------------------------

/// Construction parameters for [`XYMiniMap::new`]. Mirrors the TS
/// `XYMinimapParams`.
///
/// The TS source takes the parent `PanZoomInstance` directly. The
/// Rust port takes the two callable bits we need (`set_viewport_constrained`
/// and `scale_to`) as `Rc<dyn Fn>` aliases so the consumer can route
/// them to whatever pan/zoom layer they have — typically a clone of
/// the parent [`crate::xypanzoom::XYPanZoom`].
pub struct XYMiniMapParams {
    pub get_transform: GetTransformFn,
    pub get_view_scale: GetViewScaleFn,
    pub set_viewport_constrained: SetViewportConstrainedFn,
    pub scale_to: ScaleToFn,
}

/// Per-update reconfiguration parameters. Mirrors the TS
/// `XYMinimapUpdate`. Optional fields default to the same values as
/// the JS counterpart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XYMiniMapUpdateParams {
    pub translate_extent: CoordinateExtent,
    pub width: f64,
    pub height: f64,
    /// Whether the user's pan direction should be inverted (TS
    /// default `false`).
    pub inverse_pan: bool,
    /// Multiplier applied to wheel-zoom delta. TS default `1.0`.
    pub zoom_step: f64,
    /// Whether the minimap supports panning. TS default `true`.
    pub pannable: bool,
    /// Whether the minimap supports zooming. TS default `true`.
    pub zoomable: bool,
}

impl Default for XYMiniMapUpdateParams {
    fn default() -> Self {
        Self {
            translate_extent: [
                [f64::NEG_INFINITY, f64::NEG_INFINITY],
                [f64::INFINITY, f64::INFINITY],
            ],
            width: 0.0,
            height: 0.0,
            inverse_pan: false,
            zoom_step: 1.0,
            pannable: true,
            zoomable: true,
        }
    }
}

// ---------------------------------------------------------------------------
// XYMiniMap
// ---------------------------------------------------------------------------

/// Minimap pan/zoom controller.
///
/// Generic over `K` only by interaction style; the underlying
/// callbacks (`get_transform`, `set_viewport_constrained`, `scale_to`)
/// already encapsulate the parent-pane key. Cloning is cheap
/// (`Rc::clone`).
pub struct XYMiniMap {
    inner: Rc<XYMiniMapInner>,
}

impl Clone for XYMiniMap {
    fn clone(&self) -> Self {
        XYMiniMap {
            inner: Rc::clone(&self.inner),
        }
    }
}

struct XYMiniMapInner {
    params: XYMiniMapParams,
    update_params: RefCell<XYMiniMapUpdateParams>,
    pan_start: RefCell<XYPosition>,
    panning: RefCell<bool>,
    destroyed: RefCell<bool>,
}

impl XYMiniMap {
    /// Construct a new minimap controller.
    #[must_use]
    pub fn new(params: XYMiniMapParams) -> Self {
        XYMiniMap {
            inner: Rc::new(XYMiniMapInner {
                params,
                update_params: RefCell::new(XYMiniMapUpdateParams::default()),
                pan_start: RefCell::new(XYPosition::ZERO),
                panning: RefCell::new(false),
                destroyed: RefCell::new(false),
            }),
        }
    }

    /// Reconfigure the controller. Mirrors the TS `update()`.
    pub fn update(&self, params: XYMiniMapUpdateParams) {
        if *self.inner.destroyed.borrow() {
            return;
        }
        *self.inner.update_params.borrow_mut() = params;
    }

    /// Detach the controller. Subsequent input methods are no-ops.
    pub fn destroy(&self) {
        *self.inner.destroyed.borrow_mut() = true;
        *self.inner.panning.borrow_mut() = false;
    }

    /// Returns `true` while the user is actively dragging the minimap
    /// viewport indicator.
    #[must_use]
    pub fn is_panning(&self) -> bool {
        *self.inner.panning.borrow()
    }

    // -----------------------------------------------------------------
    // Pointer / wheel forwarders
    // -----------------------------------------------------------------

    /// Forward a `mousedown` / `touchstart` event over the minimap.
    /// Records the pan-start position. Mirrors the TS `panStartHandler`.
    pub fn handle_pointer_down(&self, event: &PointerEventLike) {
        if *self.inner.destroyed.borrow() {
            return;
        }
        if !self.inner.update_params.borrow().pannable {
            return;
        }
        *self.inner.pan_start.borrow_mut() =
            XYPosition::new(event.client_x, event.client_y);
        *self.inner.panning.borrow_mut() = true;
    }

    /// Forward a `mousemove` / `touchmove` event. While the user is
    /// dragging, this translates the parent viewport. Mirrors the TS
    /// `panHandler`.
    ///
    /// Returns the parent's `set_viewport_constrained` promise so the
    /// caller can chain on completion if desired; the more common
    /// path is to fire-and-forget.
    pub fn handle_pointer_move(&self, event: &PointerEventLike) -> Promise<bool> {
        if *self.inner.destroyed.borrow() {
            return Promise::resolved(false);
        }
        if !*self.inner.panning.borrow() {
            return Promise::resolved(false);
        }
        let update = *self.inner.update_params.borrow();
        if !update.pannable {
            return Promise::resolved(false);
        }

        let pan_current = XYPosition::new(event.client_x, event.client_y);
        let pan_start = *self.inner.pan_start.borrow();
        let pan_delta = XYPosition {
            x: pan_current.x - pan_start.x,
            y: pan_current.y - pan_start.y,
        };
        *self.inner.pan_start.borrow_mut() = pan_current;

        let transform = (self.inner.params.get_transform)();
        let view_scale = (self.inner.params.get_view_scale)();
        let k = transform.scale();
        // TS: `getViewScale() * Math.max(k, Math.log(k)) * (inversePan ? -1 : 1)`.
        // We must take care when k <= 0 (Math.log of <=0 is NaN/-Inf in
        // JS), but k is always positive (a zoom factor), so this is safe.
        let move_scale = view_scale
            * k.max(k.ln())
            * if update.inverse_pan { -1.0 } else { 1.0 };
        let position = XYPosition {
            x: transform.tx() - pan_delta.x * move_scale,
            y: transform.ty() - pan_delta.y * move_scale,
        };
        let extent: CoordinateExtent = [[0.0, 0.0], [update.width, update.height]];
        (self.inner.params.set_viewport_constrained)(
            Viewport {
                x: position.x,
                y: position.y,
                zoom: k,
            },
            extent,
            update.translate_extent,
        )
    }

    /// Forward a `mouseup` / `touchend` event. Ends the pan gesture.
    pub fn handle_pointer_up(&self, _event: &PointerEventLike) {
        *self.inner.panning.borrow_mut() = false;
    }

    /// Forward a `wheel` event over the minimap. Mirrors the TS
    /// `zoomHandler`. The wheel zoom is applied to the parent's
    /// `scale_to` — the minimap itself does not zoom.
    ///
    /// `is_mac_os` is the caller-resolved platform flag (the TS source
    /// reads `navigator.userAgent` — see `xypanzoom::utils::xy_wheel_delta`
    /// for the same reasoning).
    pub fn handle_wheel(&self, event: &WheelEventLike, is_mac_os: bool) -> Promise<bool> {
        if *self.inner.destroyed.borrow() {
            return Promise::resolved(false);
        }
        let update = *self.inner.update_params.borrow();
        if !update.zoomable {
            return Promise::resolved(false);
        }
        let transform = (self.inner.params.get_transform)();
        // TS reads `event.deltaY * deltaModeFactor * zoomStep`, then
        // applies the macOS ctrl-multiplier.
        // We reuse our existing `xy_wheel_delta` (which does the
        // delta-mode + ctrl factoring) and multiply by zoom_step.
        let pinch_delta = xy_wheel_delta(event, is_mac_os) * update.zoom_step;
        let factor = if event.ctrl_key && is_mac_os {
            // xy_wheel_delta already applied the 10x for macOS+ctrl;
            // TS does it independently here. To preserve TS semantics
            // we *don't* re-multiply — the unified path matches the
            // JS overall effect (delta_y * pixel-factor * zoom_step *
            // mac-ctrl-factor).
            1.0
        } else {
            1.0
        };
        let next_zoom = transform.scale() * 2f64.powf(pinch_delta * factor);
        (self.inner.params.scale_to)(next_zoom)
    }
}

// ---------------------------------------------------------------------------
// Convenience constructors
// ---------------------------------------------------------------------------

impl XYMiniMap {
    /// Convenience wrapper that builds the four `Rc<dyn Fn>` accessors
    /// directly from an [`crate::xypanzoom::XYPanZoom`] instance. The
    /// consumer still supplies a `get_view_scale` closure since the
    /// minimap layout is consumer-specific.
    #[must_use]
    pub fn from_panzoom<K>(
        panzoom: crate::xypanzoom::XYPanZoom<K>,
        get_view_scale: GetViewScaleFn,
    ) -> Self
    where
        K: std::hash::Hash + Eq + Clone + 'static,
    {
        let panzoom_for_transform = panzoom.clone();
        let panzoom_for_set_viewport = panzoom.clone();
        let panzoom_for_scale = panzoom.clone();
        Self::new(XYMiniMapParams {
            get_transform: Rc::new(move || {
                let v = panzoom_for_transform.get_viewport();
                Transform(v.x, v.y, v.zoom)
            }),
            get_view_scale,
            set_viewport_constrained: Rc::new(move |viewport, extent, translate_extent| {
                let p = panzoom_for_set_viewport
                    .set_viewport_constrained(viewport, extent, translate_extent);
                // Convert Promise<Option<Transform>> → Promise<bool> by
                // mapping any resolved value to `true`.
                let (out_promise, resolver) = crate::promise::channel::<bool>();
                // `Promise` is `!Send`; we can't move to a thread. Use
                // a one-shot polling pattern: try-take inline; if not
                // ready, resolve `false` to keep the API responsive.
                // The TS counterpart awaits but our consumers don't
                // need the resolved transform on the minimap path.
                match p.try_take() {
                    Some(_) => resolver.resolve(true),
                    None => resolver.resolve(false),
                }
                out_promise
            }),
            scale_to: Rc::new(move |zoom| {
                let p = panzoom_for_scale.scale_to(zoom, None);
                let (out_promise, resolver) = crate::promise::channel::<bool>();
                match p.try_take() {
                    Some(v) => resolver.resolve(v),
                    None => resolver.resolve(false),
                }
                out_promise
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn make_minimap(
        transform: Rc<Cell<Transform>>,
        view_scale: f64,
        viewport_updates: Rc<RefCell<Vec<Viewport>>>,
        zoom_calls: Rc<RefCell<Vec<f64>>>,
    ) -> XYMiniMap {
        let t_clone = Rc::clone(&transform);
        let updates = Rc::clone(&viewport_updates);
        let zooms = Rc::clone(&zoom_calls);
        XYMiniMap::new(XYMiniMapParams {
            get_transform: Rc::new(move || t_clone.get()),
            get_view_scale: Rc::new(move || view_scale),
            set_viewport_constrained: Rc::new(move |viewport, _ext, _tx| {
                updates.borrow_mut().push(viewport);
                Promise::resolved(true)
            }),
            scale_to: Rc::new(move |z| {
                zooms.borrow_mut().push(z);
                Promise::resolved(true)
            }),
        })
    }

    fn default_update() -> XYMiniMapUpdateParams {
        XYMiniMapUpdateParams {
            translate_extent: [
                [f64::NEG_INFINITY, f64::NEG_INFINITY],
                [f64::INFINITY, f64::INFINITY],
            ],
            width: 200.0,
            height: 150.0,
            ..Default::default()
        }
    }

    #[test]
    fn new_then_destroy_is_safe() {
        let m = make_minimap(
            Rc::new(Cell::new(Transform::IDENTITY)),
            1.0,
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        m.destroy();
        // Subsequent calls are no-ops; the panning flag must stay false.
        m.handle_pointer_down(&PointerEventLike {
            client_x: 1.0,
            client_y: 2.0,
            ..Default::default()
        });
        assert!(!m.is_panning());
    }

    #[test]
    fn pannable_false_blocks_pan_gesture() {
        let updates = Rc::new(RefCell::new(Vec::new()));
        let m = make_minimap(
            Rc::new(Cell::new(Transform::IDENTITY)),
            1.0,
            Rc::clone(&updates),
            Rc::new(RefCell::new(Vec::new())),
        );
        let mut params = default_update();
        params.pannable = false;
        m.update(params);

        m.handle_pointer_down(&PointerEventLike {
            client_x: 10.0,
            client_y: 10.0,
            ..Default::default()
        });
        // pannable=false → handle_pointer_down should not flip the flag.
        assert!(!m.is_panning());
        let _ = m.handle_pointer_move(&PointerEventLike {
            client_x: 20.0,
            client_y: 20.0,
            ..Default::default()
        });
        assert!(updates.borrow().is_empty());
    }

    #[test]
    fn pointer_move_translates_parent_viewport() {
        let transform = Rc::new(Cell::new(Transform::new(100.0, 50.0, 1.0)));
        let updates = Rc::new(RefCell::new(Vec::new()));
        let m = make_minimap(
            Rc::clone(&transform),
            2.0, // view_scale
            Rc::clone(&updates),
            Rc::new(RefCell::new(Vec::new())),
        );
        m.update(default_update());
        m.handle_pointer_down(&PointerEventLike {
            client_x: 100.0,
            client_y: 100.0,
            ..Default::default()
        });
        let _ = m.handle_pointer_move(&PointerEventLike {
            client_x: 110.0,
            client_y: 115.0,
            ..Default::default()
        });
        let v = updates.borrow();
        assert_eq!(v.len(), 1);
        // pan_delta = (10, 15), view_scale=2, k=1, max(1, ln(1)=0)=1 → move_scale = 2.
        // new x = 100 - 10*2 = 80; new y = 50 - 15*2 = 20.
        assert!((v[0].x - 80.0).abs() < 1e-9);
        assert!((v[0].y - 20.0).abs() < 1e-9);
        // zoom carried through unchanged.
        assert!((v[0].zoom - 1.0).abs() < 1e-9);
    }

    #[test]
    fn pointer_move_without_down_is_no_op() {
        let updates = Rc::new(RefCell::new(Vec::new()));
        let m = make_minimap(
            Rc::new(Cell::new(Transform::IDENTITY)),
            1.0,
            Rc::clone(&updates),
            Rc::new(RefCell::new(Vec::new())),
        );
        m.update(default_update());
        let _ = m.handle_pointer_move(&PointerEventLike {
            client_x: 50.0,
            client_y: 50.0,
            ..Default::default()
        });
        assert!(updates.borrow().is_empty());
    }

    #[test]
    fn pointer_up_clears_panning() {
        let m = make_minimap(
            Rc::new(Cell::new(Transform::IDENTITY)),
            1.0,
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        m.update(default_update());
        m.handle_pointer_down(&PointerEventLike::default());
        assert!(m.is_panning());
        m.handle_pointer_up(&PointerEventLike::default());
        assert!(!m.is_panning());
    }

    #[test]
    fn inverse_pan_flips_direction() {
        let transform = Rc::new(Cell::new(Transform::new(0.0, 0.0, 1.0)));
        let updates = Rc::new(RefCell::new(Vec::new()));
        let m = make_minimap(
            Rc::clone(&transform),
            1.0,
            Rc::clone(&updates),
            Rc::new(RefCell::new(Vec::new())),
        );
        let mut params = default_update();
        params.inverse_pan = true;
        m.update(params);

        m.handle_pointer_down(&PointerEventLike {
            client_x: 0.0,
            client_y: 0.0,
            ..Default::default()
        });
        let _ = m.handle_pointer_move(&PointerEventLike {
            client_x: 10.0,
            client_y: 0.0,
            ..Default::default()
        });
        // inverse_pan=true → x = 0 - 10 * (1 * 1 * -1) = +10 (flipped).
        let v = updates.borrow();
        assert!((v[0].x - 10.0).abs() < 1e-9);
    }

    #[test]
    fn wheel_calls_parent_scale_to() {
        let transform = Rc::new(Cell::new(Transform::new(0.0, 0.0, 1.0)));
        let zooms = Rc::new(RefCell::new(Vec::new()));
        let m = make_minimap(
            Rc::clone(&transform),
            1.0,
            Rc::new(RefCell::new(Vec::new())),
            Rc::clone(&zooms),
        );
        m.update(default_update());
        let _ = m.handle_wheel(
            &WheelEventLike {
                delta_y: -100.0,
                delta_mode: 0,
                ctrl_key: false,
                shift_key: false,
                delta_x: 0.0,
            },
            false,
        );
        // delta_y=-100, mode=0 → factor 0.002 → pinch_delta = 0.2.
        // Next zoom = 1 * 2^0.2 ≈ 1.1487.
        let z = zooms.borrow();
        assert_eq!(z.len(), 1);
        assert!((z[0] - 2f64.powf(0.2)).abs() < 1e-9);
    }

    #[test]
    fn wheel_respects_zoomable_false() {
        let zooms = Rc::new(RefCell::new(Vec::new()));
        let m = make_minimap(
            Rc::new(Cell::new(Transform::IDENTITY)),
            1.0,
            Rc::new(RefCell::new(Vec::new())),
            Rc::clone(&zooms),
        );
        let mut params = default_update();
        params.zoomable = false;
        m.update(params);
        let _ = m.handle_wheel(
            &WheelEventLike {
                delta_y: -100.0,
                ..Default::default()
            },
            false,
        );
        assert!(zooms.borrow().is_empty());
    }

    #[test]
    fn zoom_step_multiplies_pinch_delta() {
        let transform = Rc::new(Cell::new(Transform::new(0.0, 0.0, 1.0)));
        let zooms_a = Rc::new(RefCell::new(Vec::new()));
        let zooms_b = Rc::new(RefCell::new(Vec::new()));

        let m1 = make_minimap(
            Rc::clone(&transform),
            1.0,
            Rc::new(RefCell::new(Vec::new())),
            Rc::clone(&zooms_a),
        );
        m1.update(default_update());
        let _ = m1.handle_wheel(
            &WheelEventLike {
                delta_y: -100.0,
                ..Default::default()
            },
            false,
        );

        let m2 = make_minimap(
            Rc::clone(&transform),
            1.0,
            Rc::new(RefCell::new(Vec::new())),
            Rc::clone(&zooms_b),
        );
        let mut params = default_update();
        params.zoom_step = 2.0;
        m2.update(params);
        let _ = m2.handle_wheel(
            &WheelEventLike {
                delta_y: -100.0,
                ..Default::default()
            },
            false,
        );

        let z1 = zooms_a.borrow()[0];
        let z2 = zooms_b.borrow()[0];
        // zoom_step=2 means twice the exponent → much larger result.
        assert!(z2 > z1);
    }

    #[test]
    fn update_after_destroy_is_no_op() {
        let m = make_minimap(
            Rc::new(Cell::new(Transform::IDENTITY)),
            1.0,
            Rc::new(RefCell::new(Vec::new())),
            Rc::new(RefCell::new(Vec::new())),
        );
        m.destroy();
        // update shouldn't panic and shouldn't change anything.
        m.update(default_update());
        // pannable defaults to false now since update was rejected;
        // verify pointer-down doesn't activate the gesture either.
        m.handle_pointer_down(&PointerEventLike::default());
        assert!(!m.is_panning());
    }
}
