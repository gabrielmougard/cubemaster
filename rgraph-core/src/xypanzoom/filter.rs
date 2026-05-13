//! Port of `xyflow-core/src/xypanzoom/filter.ts`.
//!
//! Status: implemented (phase 4).
//!
//! TS reads DOM directly (`event.target.closest`, `event.preventDefault`).
//! In the Rust port we model the input event as a structured
//! [`FilterEvent`] populated by the Dioxus consumer; the `target.closest`
//! walk is replaced by a pre-collected list of ancestor class names.

#![allow(clippy::module_name_repetitions)]

use crate::types::panzoom::PanOnDrag;
use crate::xypanzoom::utils::is_wrapped_with_class;

/// Source-event shape consumed by [`create_filter`]. The Dioxus
/// consumer fills this in from each browser event.
///
/// Note: TS additionally reads `event.touches?.length`. Because the
/// touch case calls `event.preventDefault()` synchronously inside
/// the filter (line 77 of `filter.ts`), the Rust port lifts that
/// side-effect out: filter just *returns false* for that case and a
/// separate boolean in [`FilterDecision::PreventDefault`] tells the
/// caller to invoke `preventDefault()` on its own event.
#[derive(Debug, Clone)]
pub struct FilterEvent<'a> {
    pub kind: FilterEventKind,
    /// `MouseEvent.button`. 0 = primary (left), 1 = middle, 2 = right.
    pub button: u8,
    pub ctrl_key: bool,
    /// For touchstart: number of active touches at this moment.
    pub touches_len: u32,
    /// Class names of every ancestor of the event target. Replaces
    /// the TS `event.target.closest('.foo')` lookup.
    pub ancestor_classes: &'a [String],
}

/// Discriminator over the relevant browser-event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterEventKind {
    Wheel,
    MouseDown,
    TouchStart,
    /// Anything else (kept for future extension).
    Other,
}

/// Decision returned by the filter closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterDecision {
    /// Allow the event through to the zoom engine.
    Accept,
    /// Reject the event without any side effect.
    Reject,
    /// Reject the event AND tell the caller to call
    /// `event.preventDefault()` (only used for the touchstart
    /// pinch-prevention path).
    RejectAndPreventDefault,
}

/// Inputs accepted by [`create_filter`]. Mirrors `FilterParams` in
/// `filter.ts`. `pan_on_drag` is the same enum we use everywhere.
#[derive(Debug, Clone)]
pub struct CreateFilterParams {
    pub zoom_activation_key_pressed: bool,
    pub zoom_on_scroll: bool,
    pub zoom_on_pinch: bool,
    pub pan_on_drag: PanOnDrag,
    pub pan_on_scroll: bool,
    pub zoom_on_double_click: bool,
    pub user_selection_active: bool,
    pub no_wheel_class_name: String,
    pub no_pan_class_name: String,
    pub lib: String,
    pub connection_in_progress: bool,
}

/// Build a closure that decides whether a given browser event should
/// reach the underlying [`rgraph_zoom::ZoomBehavior`].
///
/// Mirrors the TS `createFilter` byte-for-byte; differences are noted
/// in the doc comments where they arise.
#[must_use]
pub fn create_filter(
    params: CreateFilterParams,
) -> Box<dyn Fn(&FilterEvent<'_>) -> FilterDecision + Send + Sync> {
    Box::new(move |event: &FilterEvent<'_>| -> FilterDecision {
        let zoom_scroll = params.zoom_activation_key_pressed || params.zoom_on_scroll;
        let pinch_zoom = params.zoom_on_pinch && event.ctrl_key;
        let is_wheel = event.kind == FilterEventKind::Wheel;

        // Allow middle-click pan on a node/edge so node-bound DnD apps
        // can still drag the pane via middle button.
        if event.button == 1
            && event.kind == FilterEventKind::MouseDown
            && (is_wrapped_with_class(
                event.ancestor_classes,
                &format!("{}-flow__node", params.lib),
            ) || is_wrapped_with_class(
                event.ancestor_classes,
                &format!("{}-flow__edge", params.lib),
            ))
        {
            return FilterDecision::Accept;
        }

        // If all interactions are disabled, prevent all zoom events.
        let any_pan_or_zoom = !matches!(params.pan_on_drag, PanOnDrag::Off)
            || zoom_scroll
            || params.pan_on_scroll
            || params.zoom_on_double_click
            || params.zoom_on_pinch;
        if !any_pan_or_zoom {
            return FilterDecision::Reject;
        }

        // During a selection prevent all other interactions.
        if params.user_selection_active {
            return FilterDecision::Reject;
        }

        // Disable pinch-zoom while a connection is being drawn.
        if params.connection_in_progress && !is_wheel {
            return FilterDecision::Reject;
        }

        // .nowheel suppresses zoom-on-wheel.
        if is_wheel
            && is_wrapped_with_class(event.ancestor_classes, &params.no_wheel_class_name)
        {
            return FilterDecision::Reject;
        }

        // .nopan suppresses panning.
        if is_wrapped_with_class(event.ancestor_classes, &params.no_pan_class_name)
            && (!is_wheel
                || (params.pan_on_scroll && is_wheel && !params.zoom_activation_key_pressed))
        {
            return FilterDecision::Reject;
        }

        if !params.zoom_on_pinch && event.ctrl_key && is_wheel {
            return FilterDecision::Reject;
        }

        if !params.zoom_on_pinch
            && event.kind == FilterEventKind::TouchStart
            && event.touches_len > 1
        {
            // TS: event.preventDefault(); return false;
            return FilterDecision::RejectAndPreventDefault;
        }

        // No scroll handling enabled — drop wheel events.
        if !zoom_scroll && !params.pan_on_scroll && !pinch_zoom && is_wheel {
            return FilterDecision::Reject;
        }

        // Pane is not movable and the event is mousedown/touchstart.
        if matches!(params.pan_on_drag, PanOnDrag::Off)
            && (event.kind == FilterEventKind::MouseDown
                || event.kind == FilterEventKind::TouchStart)
        {
            return FilterDecision::Reject;
        }

        // Pane is movable only with specific buttons — reject mismatched mousedowns.
        if let PanOnDrag::Buttons(allowed) = &params.pan_on_drag {
            if event.kind == FilterEventKind::MouseDown && !allowed.contains(&event.button) {
                return FilterDecision::Reject;
            }
        }

        // Default d3-zoom filter: allow wheel even with ctrl, and only
        // allow primary/middle/listed buttons elsewhere.
        let button_allowed = match &params.pan_on_drag {
            PanOnDrag::Buttons(allowed) => allowed.contains(&event.button),
            _ => event.button == 0 || event.button <= 1,
        };
        if (!event.ctrl_key || is_wheel) && button_allowed {
            FilterDecision::Accept
        } else {
            FilterDecision::Reject
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> CreateFilterParams {
        CreateFilterParams {
            zoom_activation_key_pressed: false,
            zoom_on_scroll: true,
            zoom_on_pinch: true,
            pan_on_drag: PanOnDrag::On,
            pan_on_scroll: false,
            zoom_on_double_click: true,
            user_selection_active: false,
            no_wheel_class_name: "nowheel".into(),
            no_pan_class_name: "nopan".into(),
            lib: "react".into(),
            connection_in_progress: false,
        }
    }

    fn evt(kind: FilterEventKind, button: u8, ancestors: &[String]) -> FilterEvent<'_> {
        FilterEvent {
            kind,
            button,
            ctrl_key: false,
            touches_len: 0,
            ancestor_classes: ancestors,
        }
    }

    #[test]
    fn defaults_accept_wheel_and_left_click() {
        let f = create_filter(defaults());
        assert_eq!(
            f(&evt(FilterEventKind::Wheel, 0, &[])),
            FilterDecision::Accept
        );
        assert_eq!(
            f(&evt(FilterEventKind::MouseDown, 0, &[])),
            FilterDecision::Accept
        );
    }

    #[test]
    fn user_selection_active_rejects_everything() {
        let mut p = defaults();
        p.user_selection_active = true;
        let f = create_filter(p);
        assert_eq!(
            f(&evt(FilterEventKind::MouseDown, 0, &[])),
            FilterDecision::Reject
        );
    }

    #[test]
    fn nowheel_class_blocks_wheel() {
        let f = create_filter(defaults());
        let ancestors = vec!["nowheel".into()];
        assert_eq!(
            f(&evt(FilterEventKind::Wheel, 0, &ancestors)),
            FilterDecision::Reject
        );
        // mousedown on the same element is still allowed.
        assert_eq!(
            f(&evt(FilterEventKind::MouseDown, 0, &ancestors)),
            FilterDecision::Accept
        );
    }

    #[test]
    fn nopan_blocks_mousedown() {
        let f = create_filter(defaults());
        let ancestors = vec!["nopan".into()];
        assert_eq!(
            f(&evt(FilterEventKind::MouseDown, 0, &ancestors)),
            FilterDecision::Reject
        );
    }

    #[test]
    fn pan_on_drag_off_blocks_mousedown() {
        let mut p = defaults();
        p.pan_on_drag = PanOnDrag::Off;
        // Need at least one zoom path available to bypass the
        // "any_pan_or_zoom" early-return.
        p.zoom_on_scroll = true;
        let f = create_filter(p);
        assert_eq!(
            f(&evt(FilterEventKind::MouseDown, 0, &[])),
            FilterDecision::Reject
        );
    }

    #[test]
    fn middle_click_on_node_passes() {
        let f = create_filter(defaults());
        let ancestors = vec!["react-flow__node".into()];
        assert_eq!(
            f(&evt(FilterEventKind::MouseDown, 1, &ancestors)),
            FilterDecision::Accept
        );
    }

    #[test]
    fn ctrl_pinch_touchstart_with_pinch_disabled_prevents_default() {
        let mut p = defaults();
        p.zoom_on_pinch = false;
        let f = create_filter(p);
        let mut e = evt(FilterEventKind::TouchStart, 0, &[]);
        e.touches_len = 2;
        assert_eq!(f(&e), FilterDecision::RejectAndPreventDefault);
    }

    #[test]
    fn pan_on_drag_buttons_filters_buttons() {
        let mut p = defaults();
        p.pan_on_drag = PanOnDrag::Buttons(vec![0, 2]);
        let f = create_filter(p);
        assert_eq!(
            f(&evt(FilterEventKind::MouseDown, 0, &[])),
            FilterDecision::Accept
        );
        assert_eq!(
            f(&evt(FilterEventKind::MouseDown, 1, &[])),
            FilterDecision::Reject
        );
        assert_eq!(
            f(&evt(FilterEventKind::MouseDown, 2, &[])),
            FilterDecision::Accept
        );
    }

    #[test]
    fn connection_in_progress_blocks_non_wheel() {
        let mut p = defaults();
        p.connection_in_progress = true;
        let f = create_filter(p);
        assert_eq!(
            f(&evt(FilterEventKind::MouseDown, 0, &[])),
            FilterDecision::Reject
        );
        // Wheel still allowed.
        assert_eq!(
            f(&evt(FilterEventKind::Wheel, 0, &[])),
            FilterDecision::Accept
        );
    }

    #[test]
    fn ctrl_wheel_with_pinch_disabled_rejected() {
        let mut p = defaults();
        p.zoom_on_pinch = false;
        let f = create_filter(p);
        let mut e = evt(FilterEventKind::Wheel, 0, &[]);
        e.ctrl_key = true;
        assert_eq!(f(&e), FilterDecision::Reject);
    }
}
