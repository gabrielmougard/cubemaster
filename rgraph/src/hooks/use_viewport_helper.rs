//! Port of `xyflow-react/src/hooks/useViewportHelper.ts`.
//!
//! Status: Phase 3 — partial implementation.
//!
//! The TS hook returns ten viewport-helper functions
//! (`zoomIn`, `zoomOut`, `zoomTo`, `getZoom`, `setViewport`,
//! `getViewport`, `setCenter`, `fitBounds`, `screenToFlowPosition`,
//! `flowToScreenPosition`). Each one delegates to the live
//! `panZoom` instance plus a few store reads.
//!
//! In our port the live `PanZoomInstance` is wired in **Phase 4** when
//! `<ZoomPane>` lands. Until then the mutating helpers (`zoom_in`,
//! `zoom_out`, `zoom_to`, `set_viewport`, `set_center`, `fit_bounds`)
//! return `false` synchronously when `pan_zoom` is `None`. The pure
//! getters and the screen↔flow conversions work right away because
//! they only depend on the store's `transform` and `width`/`height`
//! signals.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use rgraph_core::types::geometry::{Rect, Transform, XYPosition};
use rgraph_core::types::viewport::{
    FitBoundsOptions, Padding, SetCenterOptions, SnapGrid, Viewport, ViewportHelperFunctionOptions,
};
use rgraph_core::utils::general::{get_viewport_for_bounds, point_to_renderer_point, renderer_point_to_point};

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

/// Bundle of imperative viewport helpers, mirroring the TS
/// `ViewportHelperFunctions`.
///
/// All methods are non-`async` and return synchronously: the
/// underlying `Promise<bool>` returned by `rgraph_zoom` is awaited
/// in-place via [`rgraph_core::Promise::block_take`] inside the
/// store actions.
///
/// `Copy + Clone` are implemented manually for the same reason as on
/// [`crate::store::RGraphStore`]: avoid propagating `N: Copy / E: Copy`
/// bounds through the derive.
pub struct ViewportHelper<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    store: RGraphStore<N, E>,
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Copy for ViewportHelper<N, E> {}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Clone for ViewportHelper<N, E> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> ViewportHelper<N, E> {
    /// Zoom in by 1.2×. Mirrors TS `zoomIn`.
    pub fn zoom_in(&self, options: Option<ViewportHelperFunctionOptions>) -> bool {
        let Some(pan_zoom) = self.store.pan_zoom.peek().clone() else {
            return false;
        };
        let promise = pan_zoom
            .borrow_mut()
            .scale_by(1.2, options.map(into_pan_zoom_options));
        promise.block_take().unwrap_or(false)
    }

    /// Zoom out by 1/1.2×. Mirrors TS `zoomOut`.
    pub fn zoom_out(&self, options: Option<ViewportHelperFunctionOptions>) -> bool {
        let Some(pan_zoom) = self.store.pan_zoom.peek().clone() else {
            return false;
        };
        let promise = pan_zoom
            .borrow_mut()
            .scale_by(1.0 / 1.2, options.map(into_pan_zoom_options));
        promise.block_take().unwrap_or(false)
    }

    /// Zoom to an absolute zoom level. Mirrors TS `zoomTo`.
    pub fn zoom_to(&self, zoom_level: f64, options: Option<ViewportHelperFunctionOptions>) -> bool {
        let Some(pan_zoom) = self.store.pan_zoom.peek().clone() else {
            return false;
        };
        let promise = pan_zoom
            .borrow_mut()
            .scale_to(zoom_level, options.map(into_pan_zoom_options));
        promise.block_take().unwrap_or(false)
    }

    /// Read the current zoom level. Mirrors TS `getZoom`.
    pub fn get_zoom(&self) -> f64 {
        self.store.transform.peek().scale()
    }

    /// Set the viewport to a new value. Mirrors TS `setViewport`.
    /// Each component of the input is optional — `None` keeps the
    /// existing value (TS `viewport.x ?? tX`).
    pub fn set_viewport(
        &self,
        viewport: PartialViewport,
        options: Option<ViewportHelperFunctionOptions>,
    ) -> bool {
        let Some(pan_zoom) = self.store.pan_zoom.peek().clone() else {
            return false;
        };
        let current = *self.store.transform.peek();
        let next = Viewport {
            x: viewport.x.unwrap_or(current.tx()),
            y: viewport.y.unwrap_or(current.ty()),
            zoom: viewport.zoom.unwrap_or(current.scale()),
        };
        let promise = pan_zoom
            .borrow_mut()
            .set_viewport(next, options.map(into_pan_zoom_options));
        promise.block_take().unwrap_or(false)
    }

    /// Read the current viewport.
    pub fn get_viewport(&self) -> Viewport {
        let t = *self.store.transform.peek();
        Viewport {
            x: t.tx(),
            y: t.ty(),
            zoom: t.scale(),
        }
    }

    /// Center on a flow-space position. Mirrors TS `setCenter`.
    /// Delegates to [`RGraphStore::set_center`].
    pub fn set_center(&self, x: f64, y: f64, options: Option<SetCenterOptions>) -> bool {
        self.store.set_center(x, y, options)
    }

    /// Fit the viewport to a rectangle. Mirrors TS `fitBounds`.
    pub fn fit_bounds(&self, bounds: Rect, options: Option<FitBoundsOptions>) -> bool {
        let Some(pan_zoom) = self.store.pan_zoom.peek().clone() else {
            return false;
        };
        let width = *self.store.width.peek();
        let height = *self.store.height.peek();
        let min_zoom = *self.store.min_zoom.peek();
        let max_zoom = *self.store.max_zoom.peek();

        let padding = options
            .as_ref()
            .and_then(|o| o.padding.map(Padding::factor))
            .unwrap_or_else(|| Padding::factor(0.1));

        let viewport =
            get_viewport_for_bounds(bounds, width, height, min_zoom, max_zoom, padding);

        let transform_options = options.map(|o| {
            rgraph_core::types::panzoom::PanZoomTransformOptions {
                duration: o.base.duration,
                ease: o.base.ease,
                interpolate: o.base.interpolate,
            }
        });
        let promise = pan_zoom.borrow_mut().set_viewport(viewport, transform_options);
        promise.block_take().unwrap_or(false)
    }

    /// Translate a screen-space point to flow-space.
    ///
    /// Mirrors TS `screenToFlowPosition`. The TS source reads the
    /// wrapper element's `getBoundingClientRect` to subtract the
    /// dom-node origin; our port stores the wrapper id in
    /// `dom_node_id` and Phase 4 will install a per-frame cache of
    /// its `(domX, domY)` so this hook can subtract it. For Phase 3
    /// we treat the wrapper origin as `(0, 0)`, which is correct when
    /// the wrapper is at the document origin (the common case in
    /// embedded test setups).
    pub fn screen_to_flow_position(&self, p: XYPosition, options: ScreenToFlowOptions) -> XYPosition {
        let transform = *self.store.transform.peek();
        let snap_grid = options.snap_grid.unwrap_or(*self.store.snap_grid.peek());
        let snap_to_grid = options.snap_to_grid.unwrap_or(*self.store.snap_to_grid.peek());
        // TODO(rgraph/phase4): subtract `getBoundingClientRect()` of
        // the wrapper before converting — see TS lines 84–98.
        point_to_renderer_point(p, transform, snap_to_grid, snap_grid)
    }

    /// Translate a flow-space point to screen-space.
    pub fn flow_to_screen_position(&self, p: XYPosition) -> XYPosition {
        let transform = *self.store.transform.peek();
        // TODO(rgraph/phase4): add `getBoundingClientRect()` of the
        // wrapper after converting — see TS lines 104–118.
        renderer_point_to_point(p, transform)
    }
}

/// Optional-component viewport accepted by
/// [`ViewportHelper::set_viewport`]. Mirrors the TS
/// `Partial<Viewport>` argument.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PartialViewport {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub zoom: Option<f64>,
}

impl From<Viewport> for PartialViewport {
    fn from(v: Viewport) -> Self {
        PartialViewport {
            x: Some(v.x),
            y: Some(v.y),
            zoom: Some(v.zoom),
        }
    }
}

/// Options accepted by [`ViewportHelper::screen_to_flow_position`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ScreenToFlowOptions {
    pub snap_to_grid: Option<bool>,
    pub snap_grid: Option<SnapGrid>,
}

fn into_pan_zoom_options(
    o: ViewportHelperFunctionOptions,
) -> rgraph_core::types::panzoom::PanZoomTransformOptions {
    rgraph_core::types::panzoom::PanZoomTransformOptions {
        duration: o.duration,
        ease: o.ease,
        interpolate: o.interpolate,
    }
}

// `Transform` import is used inside `ViewportHelper::get_zoom` /
// `get_viewport` indirectly through `*self.store.transform.peek()`.
// Suppress dead-code warnings in case future refactors decouple them.
#[allow(dead_code)]
type _T = Transform;

/// Returns a [`ViewportHelper`] handle bound to the current store.
#[must_use]
pub fn use_viewport_helper<N, E>() -> ViewportHelper<N, E>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    ViewportHelper { store }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn get_viewport_returns_identity_by_default() {
        thread_local! { static SEEN: Cell<(f64, f64, f64)> = const { Cell::new((-1.0, -1.0, -1.0)) }; }

        #[component]
        fn Probe() -> Element {
            let h = use_viewport_helper::<(), ()>();
            let v = h.get_viewport();
            SEEN.with(|c| c.set((v.x, v.y, v.zoom)));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(SEEN.with(|c| c.get()), (0.0, 0.0, 1.0));
    }

    #[test]
    fn zoom_in_returns_false_when_no_pan_zoom() {
        thread_local! { static OK: Cell<bool> = const { Cell::new(true) }; }

        #[component]
        fn Probe() -> Element {
            let h = use_viewport_helper::<(), ()>();
            // No PanZoom mounted in Phase 3 → returns false.
            OK.with(|c| c.set(h.zoom_in(None)));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(!OK.with(|c| c.get()));
    }
}
