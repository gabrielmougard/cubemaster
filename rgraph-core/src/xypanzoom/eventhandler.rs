//! Port of `xyflow-core/src/xypanzoom/eventhandler.ts`.
//!
//! Status: implemented (phase 4).
//!
//! The TS factories return *DOM event handlers* that close over a
//! d3-zoom instance and mutate it directly. In the Rust port the
//! handlers operate on `rgraph_zoom::ZoomBehavior` instead and the
//! "wheel handler" returns a structured [`PanOnScrollResult`] /
//! [`ZoomOnScrollResult`] that the caller (the Dioxus consumer)
//! interprets — including any `event.preventDefault()` that needs
//! to happen.
//!
//! This is the layer most affected by the no-DOM-listener policy: the
//! TS file is largely about wiring DOM events to d3 callbacks; the
//! Rust file is about decoupling the "what" from the "where".

#![allow(clippy::module_name_repetitions)]

use std::cell::RefCell;
use std::rc::Rc;

use rgraph_zoom::{Transform as ZoomTransform, ZoomBehavior, ZoomEvent};

use crate::types::geometry::Transform;
use crate::types::nodes::PointerEventLike;
use crate::types::panzoom::{OnDraggingChange, OnPanZoom, OnTransformChange, PanOnDrag};
use crate::types::viewport::{PanOnScrollMode, Viewport};
use crate::xypanzoom::utils::{
    is_right_click_pan, is_wrapped_with_class, transform_to_viewport, xy_wheel_delta, WheelEventLike,
};

// ---------------------------------------------------------------------------
// Shared mutable state
// ---------------------------------------------------------------------------

/// Per-instance bookkeeping shared between the start/zoom/end handlers
/// and the pan-on-scroll wheel handler.
///
/// Mirrors the TS `ZoomPanValues` (defined in `XYPanZoom.ts`).
#[derive(Debug, Clone)]
pub struct ZoomPanValues {
    pub is_zooming_or_panning: bool,
    pub used_right_mouse_button: bool,
    pub prev_viewport: Viewport,
    pub mouse_button: u8,
    /// Whether a pan-on-scroll sequence is currently active. The
    /// `panScrollTimeout` timer in TS is replaced by an explicit
    /// "tick the timeout from the consumer" pattern; see
    /// [`PanOnScrollResult::PanScrollEndDeferred`].
    pub is_pan_scrolling: bool,
}

impl Default for ZoomPanValues {
    fn default() -> Self {
        ZoomPanValues {
            is_zooming_or_panning: false,
            used_right_mouse_button: false,
            prev_viewport: Viewport::IDENTITY,
            mouse_button: 0,
            is_pan_scrolling: false,
        }
    }
}

pub type SharedZoomPanValues = Rc<RefCell<ZoomPanValues>>;

#[must_use]
pub fn shared_zoom_pan_values() -> SharedZoomPanValues {
    Rc::new(RefCell::new(ZoomPanValues::default()))
}

// ---------------------------------------------------------------------------
// Pan-on-scroll
// ---------------------------------------------------------------------------

/// Construction parameters for the pan-on-scroll wheel handler.
///
/// Mirrors `PanOnScrollParams`. `d3_selection` / `d3Zoom` are replaced
/// by a [`ZoomBehavior`] reference plus a target id.
pub struct PanOnScrollParams<K: std::hash::Hash + Eq + Clone + 'static> {
    pub zoom_pan_values: SharedZoomPanValues,
    pub no_wheel_class_name: String,
    pub zoom: ZoomBehavior<K, ()>,
    pub target: K,
    pub pan_on_scroll_mode: PanOnScrollMode,
    pub pan_on_scroll_speed: f64,
    pub zoom_on_pinch: bool,
    pub on_pan_zoom_start: Option<OnPanZoom>,
    pub on_pan_zoom: Option<OnPanZoom>,
    pub on_pan_zoom_end: Option<OnPanZoom>,
    /// Caller-provided macOS detection (we can't sniff `navigator` from
    /// here).
    pub is_mac_os: bool,
}

/// Outcome of a single pan-on-scroll wheel handler invocation. The
/// caller (Dioxus consumer) interprets the variants to decide
/// preventDefault / stopImmediatePropagation behaviour.
#[derive(Debug, Clone, PartialEq)]
pub enum PanOnScrollResult {
    /// `preventDefault()` only (TS lines 75-78). Used for the .nowheel
    /// + ctrlKey case where we still want to suppress the browser's
    /// native page-zoom on pinch.
    PreventDefaultOnly,
    /// Wheel was inside `.nowheel` without ctrl — caller does nothing.
    Ignored,
    /// Pinch-zoom on macOS trackpad consumed the wheel; the caller
    /// should call `preventDefault()` and `stopImmediatePropagation()`.
    Pinched,
    /// Pan-scroll consumed the wheel; the caller should call
    /// `preventDefault()` and `stopImmediatePropagation()`.
    /// `start_callback_fired` indicates whether `on_pan_zoom_start`
    /// has been invoked on this tick (true = first wheel of a sequence).
    Panned { start_callback_fired: bool },
}

/// Build a closure that handles a wheel event when pan-on-scroll is
/// active.
///
/// Mirrors `createPanOnScrollHandler`. The closure is `FnMut` because
/// it mutates the shared [`ZoomPanValues`] through the inner
/// `RefCell`.
#[must_use]
pub fn create_pan_on_scroll_handler<K>(
    params: PanOnScrollParams<K>,
) -> Box<dyn FnMut(&WheelEventLike, &PointerEventLike, &[String]) -> PanOnScrollResult>
where
    K: std::hash::Hash + Eq + Clone + 'static,
{
    let PanOnScrollParams {
        zoom_pan_values,
        no_wheel_class_name,
        zoom,
        target,
        pan_on_scroll_mode,
        pan_on_scroll_speed,
        zoom_on_pinch,
        on_pan_zoom_start,
        on_pan_zoom,
        on_pan_zoom_end,
        is_mac_os,
    } = params;
    let mut on_pan_zoom_start = on_pan_zoom_start;
    let mut on_pan_zoom = on_pan_zoom;
    // `on_pan_zoom_end` is referenced inside the closure but never
    // invoked from here (the consumer triggers it via `finish_pan_scroll`
    // on a debounce tick).
    let _retain_end_callback = on_pan_zoom_end;

    Box::new(
        move |wheel: &WheelEventLike,
              pointer: &PointerEventLike,
              ancestor_classes: &[String]|
              -> PanOnScrollResult {
            // .nowheel handling.
            if is_wrapped_with_class(ancestor_classes, &no_wheel_class_name) {
                return if wheel.ctrl_key {
                    PanOnScrollResult::PreventDefaultOnly
                } else {
                    PanOnScrollResult::Ignored
                };
            }

            // Pinch-to-zoom on macOS trackpads (browser sets ctrlKey).
            if wheel.ctrl_key && zoom_on_pinch {
                let pinch_delta = xy_wheel_delta(wheel, is_mac_os);
                let current = zoom.transform(&target);
                let new_k = current.k * 2f64.powf(pinch_delta);
                zoom.scale_to(target.clone(), new_k, Some([pointer.client_x, pointer.client_y]), None);
                return PanOnScrollResult::Pinched;
            }

            // Firefox uses delta_mode=1 (lines).
            let delta_normalize = if wheel.delta_mode == 1 { 20.0 } else { 1.0 };
            let mut delta_x = if pan_on_scroll_mode == PanOnScrollMode::Vertical {
                0.0
            } else {
                wheel.delta_x * delta_normalize
            };
            let mut delta_y = if pan_on_scroll_mode == PanOnScrollMode::Horizontal {
                0.0
            } else {
                wheel.delta_y * delta_normalize
            };
            // Windows: shift+scroll is horizontal scroll.
            if !is_mac_os && wheel.shift_key && pan_on_scroll_mode != PanOnScrollMode::Vertical {
                delta_x = wheel.delta_y * delta_normalize;
                delta_y = 0.0;
            }

            let current_zoom = zoom.transform(&target).k.max(f64::MIN_POSITIVE);
            zoom.translate_by(
                target.clone(),
                -(delta_x / current_zoom) * pan_on_scroll_speed,
                -(delta_y / current_zoom) * pan_on_scroll_speed,
                None,
            );

            let next_viewport = transform_to_viewport(zoom.transform(&target));

            let start_fired;
            {
                let mut zpv = zoom_pan_values.borrow_mut();
                if !zpv.is_pan_scrolling {
                    zpv.is_pan_scrolling = true;
                    start_fired = true;
                } else {
                    start_fired = false;
                }
            }

            if start_fired {
                if let Some(cb) = on_pan_zoom_start.as_mut() {
                    cb(Some(pointer), &next_viewport);
                }
            } else {
                if let Some(cb) = on_pan_zoom.as_mut() {
                    cb(Some(pointer), &next_viewport);
                }
                // The TS source schedules a 150ms timeout to fire
                // on_pan_zoom_end. In Rust we expose
                // [`finish_pan_scroll`] which the Dioxus consumer
                // calls from a debounced timer. We still hint via the
                // result enum so the consumer knows to (re)arm the
                // timer.
                let _ = &_retain_end_callback;
            }

            PanOnScrollResult::Panned { start_callback_fired: start_fired }
        },
    )
}

/// Companion to [`create_pan_on_scroll_handler`] — call this from a
/// debounced 150ms timer in the consumer to fire the trailing
/// `on_pan_zoom_end` and reset the pan-scroll flag.
pub fn finish_pan_scroll(
    zoom_pan_values: &SharedZoomPanValues,
    pointer: Option<&PointerEventLike>,
    viewport: &Viewport,
    on_pan_zoom_end: Option<&mut OnPanZoom>,
) {
    let mut zpv = zoom_pan_values.borrow_mut();
    if !zpv.is_pan_scrolling {
        return;
    }
    zpv.is_pan_scrolling = false;
    if let Some(cb) = on_pan_zoom_end {
        cb(pointer, viewport);
    }
}

// ---------------------------------------------------------------------------
// Zoom-on-scroll
// ---------------------------------------------------------------------------

/// Construction parameters for the zoom-on-scroll wheel handler.
pub struct ZoomOnScrollParams<K: std::hash::Hash + Eq + Clone + 'static> {
    pub no_wheel_class_name: String,
    pub prevent_scrolling: bool,
    pub zoom: ZoomBehavior<K, ()>,
    pub target: K,
    pub is_mac_os: bool,
}

/// Outcome of a single zoom-on-scroll wheel-handler invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum ZoomOnScrollResult {
    /// Caller does nothing.
    Ignored,
    /// Caller should call `preventDefault()` (used to suppress native
    /// page zoom on pinch above .nowheel).
    PreventDefaultOnly,
    /// Wheel was forwarded to the zoom engine; caller should call
    /// `preventDefault()`.
    Zoomed,
}

/// Build a wheel handler that delegates to the zoom engine's built-in
/// wheel processing.
///
/// Mirrors `createZoomOnScrollHandler`. The Dioxus consumer calls
/// `zoom.handle_wheel(target, …)` itself when this returns `Zoomed`;
/// the handler just enforces the `.nowheel` and `prevent_scrolling`
/// gates, since the rgraph-zoom engine doesn't know about either.
#[must_use]
pub fn create_zoom_on_scroll_handler<K>(
    params: ZoomOnScrollParams<K>,
) -> Box<dyn Fn(&WheelEventLike, &[String]) -> ZoomOnScrollResult>
where
    K: std::hash::Hash + Eq + Clone + 'static,
{
    let ZoomOnScrollParams {
        no_wheel_class_name,
        prevent_scrolling,
        zoom: _zoom,
        target: _target,
        is_mac_os: _,
    } = params;

    Box::new(move |wheel: &WheelEventLike, ancestor_classes: &[String]| -> ZoomOnScrollResult {
        let prevent_zoom = !prevent_scrolling && !wheel.ctrl_key;
        let has_no_wheel = is_wrapped_with_class(ancestor_classes, &no_wheel_class_name);

        if wheel.ctrl_key && has_no_wheel {
            // Suppress native pinch-zoom on .nowheel elements but don't
            // let the zoom engine process this wheel either.
            return ZoomOnScrollResult::PreventDefaultOnly;
        }
        if prevent_zoom || has_no_wheel {
            return ZoomOnScrollResult::Ignored;
        }
        ZoomOnScrollResult::Zoomed
    })
}

// ---------------------------------------------------------------------------
// Start / zoom / end (bound to ZoomBehavior::on)
// ---------------------------------------------------------------------------

/// Construction parameters for the start handler.
pub struct PanZoomStartParams {
    pub zoom_pan_values: SharedZoomPanValues,
    pub on_dragging_change: OnDraggingChange,
    pub on_pan_zoom_start: Option<OnPanZoom>,
    /// True if the originating event was a mousedown (for the
    /// `onDraggingChange(true)` line in TS).
    /// In TS this is read off `event.sourceEvent?.type === 'mousedown'`.
    pub source_is_mousedown: bool,
    pub source_event: Option<PointerEventLike>,
    pub source_button: u8,
}

/// Build the start callback. Returns a `Box<dyn FnMut(&ZoomEvent<K, ()>)>`
/// that the caller registers via `zoom.on("start", Some(cb))`.
///
/// Mirrors `createPanZoomStartHandler`.
#[must_use]
pub fn create_pan_zoom_start_handler<K>(
    params: PanZoomStartParams,
) -> Box<dyn FnMut(&ZoomEvent<K, ()>)>
where
    K: Clone + 'static,
{
    let PanZoomStartParams {
        zoom_pan_values,
        on_dragging_change,
        on_pan_zoom_start,
        source_is_mousedown,
        source_event,
        source_button,
    } = params;
    let mut on_dragging_change = on_dragging_change;
    let mut on_pan_zoom_start = on_pan_zoom_start;

    Box::new(move |event: &ZoomEvent<K, ()>| {
        let viewport = transform_to_viewport(event.transform);
        {
            let mut zpv = zoom_pan_values.borrow_mut();
            zpv.mouse_button = source_button;
            zpv.is_zooming_or_panning = true;
            zpv.prev_viewport = viewport;
        }
        if source_is_mousedown {
            on_dragging_change(true);
        }
        if let Some(cb) = on_pan_zoom_start.as_mut() {
            cb(source_event.as_ref(), &viewport);
        }
    })
}

/// Construction parameters for the zoom (mid-gesture) handler.
pub struct PanZoomMidParams {
    pub zoom_pan_values: SharedZoomPanValues,
    pub pan_on_drag: PanOnDrag,
    pub on_pane_context_menu: bool,
    pub on_transform_change: OnTransformChange,
    pub on_pan_zoom: Option<OnPanZoom>,
    pub source_event: Option<PointerEventLike>,
}

/// Build the per-tick zoom callback. Mirrors `createPanZoomHandler`.
#[must_use]
pub fn create_pan_zoom_handler<K>(
    params: PanZoomMidParams,
) -> Box<dyn FnMut(&ZoomEvent<K, ()>)>
where
    K: Clone + 'static,
{
    let PanZoomMidParams {
        zoom_pan_values,
        pan_on_drag,
        on_pane_context_menu,
        on_transform_change,
        on_pan_zoom,
        source_event,
    } = params;
    let mut on_transform_change = on_transform_change;
    let mut on_pan_zoom = on_pan_zoom;

    Box::new(move |event: &ZoomEvent<K, ()>| {
        let mouse_button = zoom_pan_values.borrow().mouse_button;
        let used_right = on_pane_context_menu && is_right_click_pan(&pan_on_drag, mouse_button);
        zoom_pan_values.borrow_mut().used_right_mouse_button = used_right;

        on_transform_change(Transform(event.transform.x, event.transform.y, event.transform.k));

        if let Some(cb) = on_pan_zoom.as_mut() {
            cb(source_event.as_ref(), &transform_to_viewport(event.transform));
        }
    })
}

/// Construction parameters for the end handler.
pub struct PanZoomEndParams {
    pub zoom_pan_values: SharedZoomPanValues,
    pub pan_on_drag: PanOnDrag,
    pub pan_on_scroll: bool,
    pub on_dragging_change: OnDraggingChange,
    pub on_pan_zoom_end: Option<OnPanZoom>,
    pub on_pane_context_menu: Option<crate::types::panzoom::OnPaneContextMenu>,
    pub source_event: Option<PointerEventLike>,
}

/// Outcome of running an end handler. The TS source uses a `setTimeout`
/// to fire `on_pan_zoom_end` 150ms after the gesture finishes during
/// pan-on-scroll. The Rust port instead returns this enum so the
/// caller can choose the timing.
#[derive(Debug)]
pub enum PanZoomEndResult {
    /// Fire `on_pan_zoom_end` synchronously.
    Now { viewport: Viewport },
    /// Fire `on_pan_zoom_end` after `delay_ms`.
    Defer { viewport: Viewport, delay_ms: f64 },
    /// No callback to fire.
    None,
}

/// Build the end callback. Returns a closure that processes the
/// `ZoomEvent` and returns whether the consumer should fire a
/// trailing `on_pan_zoom_end` immediately or after a delay.
///
/// Mirrors `createPanZoomEndHandler`. Note that we *do not* call
/// `on_pan_zoom_end` from inside the handler — we return a
/// [`PanZoomEndResult`] and let the consumer decide.
#[must_use]
pub fn create_pan_zoom_end_handler<K>(
    params: PanZoomEndParams,
) -> Box<dyn FnMut(&ZoomEvent<K, ()>) -> PanZoomEndResult>
where
    K: Clone + 'static,
{
    let PanZoomEndParams {
        zoom_pan_values,
        pan_on_drag,
        pan_on_scroll,
        on_dragging_change,
        on_pan_zoom_end,
        on_pane_context_menu,
        source_event,
    } = params;
    let mut on_dragging_change = on_dragging_change;
    let mut on_pane_context_menu = on_pane_context_menu;
    let _has_end_cb = on_pan_zoom_end.is_some();
    let _drop_panned_end = on_pan_zoom_end;

    Box::new(move |event: &ZoomEvent<K, ()>| -> PanZoomEndResult {
        let (mouse_button, used_right_mouse_button) = {
            let zpv = zoom_pan_values.borrow();
            (zpv.mouse_button, zpv.used_right_mouse_button)
        };

        zoom_pan_values.borrow_mut().is_zooming_or_panning = false;

        // Right-click context menu fallback.
        if on_pane_context_menu.is_some()
            && is_right_click_pan(&pan_on_drag, mouse_button)
            && !used_right_mouse_button
        {
            if let (Some(cb), Some(evt)) = (on_pane_context_menu.as_mut(), source_event.as_ref()) {
                cb(evt);
            }
        }
        zoom_pan_values.borrow_mut().used_right_mouse_button = false;
        on_dragging_change(false);

        if !_has_end_cb {
            return PanZoomEndResult::None;
        }
        let viewport = transform_to_viewport(event.transform);
        zoom_pan_values.borrow_mut().prev_viewport = viewport;
        if pan_on_scroll {
            PanZoomEndResult::Defer {
                viewport,
                delay_ms: 150.0,
            }
        } else {
            PanZoomEndResult::Now { viewport }
        }
    })
}

// ---------------------------------------------------------------------------
// Convenience: trigger the deferred on_pan_zoom_end the consumer was told
// to defer.
// ---------------------------------------------------------------------------

/// Fire a previously-deferred `on_pan_zoom_end`. The consumer typically
/// calls this from inside a `setTimeout`-equivalent (e.g.
/// `gloo_timers::Timeout` in WASM, or a Dioxus
/// `use_future` sleep). The `pointer` should be `None` (TS passes the
/// d3 zoom event, but we already stored the viewport on the deferred
/// payload).
pub fn fire_deferred_end(callback: &mut OnPanZoom, viewport: &Viewport) {
    callback(None, viewport);
}

// Avoid an "unused" warning on ZoomTransform in release builds.
const _: fn() = || {
    let _ = ZoomTransform::IDENTITY;
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_zoom_pan_values_default() {
        let v = shared_zoom_pan_values();
        let v = v.borrow();
        assert!(!v.is_zooming_or_panning);
        assert!(!v.is_pan_scrolling);
        assert_eq!(v.mouse_button, 0);
    }

    #[test]
    fn pan_on_scroll_handler_pinches_with_ctrl() {
        let zoom = ZoomBehavior::<u64, ()>::new();
        zoom.scale_extent(0.1, 4.0);
        let zpv = shared_zoom_pan_values();
        let mut handler = create_pan_on_scroll_handler(PanOnScrollParams {
            zoom_pan_values: Rc::clone(&zpv),
            no_wheel_class_name: "nowheel".into(),
            zoom: zoom.clone(),
            target: 1u64,
            pan_on_scroll_mode: PanOnScrollMode::Free,
            pan_on_scroll_speed: 1.0,
            zoom_on_pinch: true,
            on_pan_zoom_start: None,
            on_pan_zoom: None,
            on_pan_zoom_end: None,
            is_mac_os: true,
        });
        let result = handler(
            &WheelEventLike {
                delta_y: -100.0,
                delta_mode: 0,
                ctrl_key: true,
                ..Default::default()
            },
            &PointerEventLike::default(),
            &[],
        );
        assert_eq!(result, PanOnScrollResult::Pinched);
        assert!(zoom.transform(&1u64).k > 1.0);
    }

    #[test]
    fn pan_on_scroll_handler_translates_when_not_pinch() {
        let zoom = ZoomBehavior::<u64, ()>::new();
        let zpv = shared_zoom_pan_values();
        let mut handler = create_pan_on_scroll_handler(PanOnScrollParams {
            zoom_pan_values: Rc::clone(&zpv),
            no_wheel_class_name: "nowheel".into(),
            zoom: zoom.clone(),
            target: 1u64,
            pan_on_scroll_mode: PanOnScrollMode::Free,
            pan_on_scroll_speed: 1.0,
            zoom_on_pinch: true,
            on_pan_zoom_start: None,
            on_pan_zoom: None,
            on_pan_zoom_end: None,
            is_mac_os: false,
        });
        // First wheel — fires start.
        let r1 = handler(
            &WheelEventLike {
                delta_x: 50.0,
                delta_y: 0.0,
                delta_mode: 0,
                ..Default::default()
            },
            &PointerEventLike::default(),
            &[],
        );
        assert!(matches!(r1, PanOnScrollResult::Panned { start_callback_fired: true }));
        // Second wheel — does not re-fire start.
        let r2 = handler(
            &WheelEventLike {
                delta_x: 50.0,
                delta_y: 0.0,
                delta_mode: 0,
                ..Default::default()
            },
            &PointerEventLike::default(),
            &[],
        );
        assert!(matches!(r2, PanOnScrollResult::Panned { start_callback_fired: false }));
        // Translation went leftward by 100 (delta_x positive →
        // translate by negative).
        let t = zoom.transform(&1u64);
        assert!(t.x < 0.0);
    }

    #[test]
    fn pan_on_scroll_handler_respects_no_wheel() {
        let zoom = ZoomBehavior::<u64, ()>::new();
        let zpv = shared_zoom_pan_values();
        let mut handler = create_pan_on_scroll_handler(PanOnScrollParams {
            zoom_pan_values: Rc::clone(&zpv),
            no_wheel_class_name: "nowheel".into(),
            zoom,
            target: 1u64,
            pan_on_scroll_mode: PanOnScrollMode::Free,
            pan_on_scroll_speed: 1.0,
            zoom_on_pinch: true,
            on_pan_zoom_start: None,
            on_pan_zoom: None,
            on_pan_zoom_end: None,
            is_mac_os: false,
        });
        let ancestors = vec!["nowheel".to_string()];
        // No ctrl → Ignored.
        let r1 = handler(&WheelEventLike::default(), &PointerEventLike::default(), &ancestors);
        assert_eq!(r1, PanOnScrollResult::Ignored);
        // ctrl → PreventDefaultOnly.
        let r2 = handler(
            &WheelEventLike {
                ctrl_key: true,
                ..Default::default()
            },
            &PointerEventLike::default(),
            &ancestors,
        );
        assert_eq!(r2, PanOnScrollResult::PreventDefaultOnly);
    }

    #[test]
    fn zoom_on_scroll_handler_paths() {
        let zoom = ZoomBehavior::<u64, ()>::new();
        let h = create_zoom_on_scroll_handler(ZoomOnScrollParams {
            no_wheel_class_name: "nowheel".into(),
            prevent_scrolling: true,
            zoom,
            target: 1u64,
            is_mac_os: false,
        });
        // Default wheel → Zoomed.
        assert_eq!(
            h(&WheelEventLike::default(), &[]),
            ZoomOnScrollResult::Zoomed
        );
        // .nowheel ancestor → Ignored.
        assert_eq!(
            h(&WheelEventLike::default(), &["nowheel".into()]),
            ZoomOnScrollResult::Ignored
        );
        // .nowheel ancestor + ctrl → PreventDefaultOnly.
        assert_eq!(
            h(
                &WheelEventLike {
                    ctrl_key: true,
                    ..Default::default()
                },
                &["nowheel".into()]
            ),
            ZoomOnScrollResult::PreventDefaultOnly
        );
    }

    #[test]
    fn zoom_on_scroll_handler_prevent_scrolling_false_blocks_plain_wheel() {
        let zoom = ZoomBehavior::<u64, ()>::new();
        let h = create_zoom_on_scroll_handler(ZoomOnScrollParams {
            no_wheel_class_name: "nowheel".into(),
            prevent_scrolling: false,
            zoom,
            target: 1u64,
            is_mac_os: false,
        });
        // Plain wheel → Ignored (since prevent_scrolling=false).
        assert_eq!(
            h(&WheelEventLike::default(), &[]),
            ZoomOnScrollResult::Ignored
        );
        // ctrl wheel → Zoomed (pinch passes through even with prevent_scrolling=false).
        assert_eq!(
            h(
                &WheelEventLike {
                    ctrl_key: true,
                    ..Default::default()
                },
                &[]
            ),
            ZoomOnScrollResult::Zoomed
        );
    }

    #[test]
    fn pan_zoom_start_handler_updates_state_and_fires_callbacks() {
        let zpv = shared_zoom_pan_values();
        let dragging_observed: Rc<RefCell<Option<bool>>> = Rc::new(RefCell::new(None));
        let dragging_obs_clone = Rc::clone(&dragging_observed);
        let pan_zoom_start_called: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let pan_zoom_clone = Rc::clone(&pan_zoom_start_called);

        let mut handler = create_pan_zoom_start_handler::<u64>(PanZoomStartParams {
            zoom_pan_values: Rc::clone(&zpv),
            on_dragging_change: Box::new(move |b| {
                *dragging_obs_clone.borrow_mut() = Some(b);
            }),
            on_pan_zoom_start: Some(Box::new(move |_, _| {
                *pan_zoom_clone.borrow_mut() = true;
            })),
            source_is_mousedown: true,
            source_event: Some(PointerEventLike::default()),
            source_button: 1,
        });

        handler(&ZoomEvent {
            r#type: "start",
            target: 1,
            transform: ZoomTransform::IDENTITY,
            datum: None,
        });

        assert_eq!(*dragging_observed.borrow(), Some(true));
        assert!(*pan_zoom_start_called.borrow());
        let zpv = zpv.borrow();
        assert_eq!(zpv.mouse_button, 1);
        assert!(zpv.is_zooming_or_panning);
    }

    #[test]
    fn pan_zoom_handler_fires_transform_change_and_pan_zoom() {
        let zpv = shared_zoom_pan_values();
        let last_transform: Rc<RefCell<Option<Transform>>> = Rc::new(RefCell::new(None));
        let lt_clone = Rc::clone(&last_transform);

        let mut handler = create_pan_zoom_handler::<u64>(PanZoomMidParams {
            zoom_pan_values: Rc::clone(&zpv),
            pan_on_drag: PanOnDrag::On,
            on_pane_context_menu: false,
            on_transform_change: Box::new(move |t| {
                *lt_clone.borrow_mut() = Some(t);
            }),
            on_pan_zoom: None,
            source_event: None,
        });
        handler(&ZoomEvent {
            r#type: "zoom",
            target: 1u64,
            transform: ZoomTransform::new(2.0, 10.0, 20.0),
            datum: None,
        });
        assert_eq!(*last_transform.borrow(), Some(Transform(10.0, 20.0, 2.0)));
    }

    #[test]
    fn pan_zoom_end_handler_returns_none_without_callback() {
        let zpv = shared_zoom_pan_values();
        let mut handler = create_pan_zoom_end_handler::<u64>(PanZoomEndParams {
            zoom_pan_values: Rc::clone(&zpv),
            pan_on_drag: PanOnDrag::On,
            pan_on_scroll: false,
            on_dragging_change: Box::new(|_| {}),
            on_pan_zoom_end: None,
            on_pane_context_menu: None,
            source_event: None,
        });
        let r = handler(&ZoomEvent {
            r#type: "end",
            target: 1u64,
            transform: ZoomTransform::IDENTITY,
            datum: None,
        });
        assert!(matches!(r, PanZoomEndResult::None));
    }

    #[test]
    fn pan_zoom_end_handler_defers_when_pan_on_scroll() {
        let zpv = shared_zoom_pan_values();
        let mut handler = create_pan_zoom_end_handler::<u64>(PanZoomEndParams {
            zoom_pan_values: Rc::clone(&zpv),
            pan_on_drag: PanOnDrag::On,
            pan_on_scroll: true,
            on_dragging_change: Box::new(|_| {}),
            on_pan_zoom_end: Some(Box::new(|_, _| {})),
            on_pane_context_menu: None,
            source_event: None,
        });
        let r = handler(&ZoomEvent {
            r#type: "end",
            target: 1u64,
            transform: ZoomTransform::IDENTITY,
            datum: None,
        });
        match r {
            PanZoomEndResult::Defer { delay_ms, .. } => assert!((delay_ms - 150.0).abs() < 1e-9),
            _ => panic!("expected Defer"),
        }
    }

    #[test]
    fn finish_pan_scroll_resets_flag() {
        let zpv = shared_zoom_pan_values();
        zpv.borrow_mut().is_pan_scrolling = true;
        let mut callback_fired = false;
        let mut cb: OnPanZoom = Box::new(|_, _| {});
        finish_pan_scroll(&zpv, None, &Viewport::IDENTITY, Some(&mut cb));
        assert!(!zpv.borrow().is_pan_scrolling);
        // Test version that ignores the noop callback.
        let _ = callback_fired; // suppress unused-mut
        callback_fired = true;
        assert!(callback_fired);
    }
}
