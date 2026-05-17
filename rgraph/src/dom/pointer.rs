//! `dom::pointer` — Dioxus `PointerData` → `rgraph_zoom::PointerInput` adapter.
//!
//! Status: Phase 4 — implemented.
//!
//! ## What this module does
//!
//! Translates a Dioxus [`PointerData`] event into the [`PointerInput`]
//! variants consumed by [`rgraph_zoom::ZoomBehavior`]. The conversion
//! is parameterised over the [`PointerEventKind`] (Down / Move / Up /
//! Cancel) because Dioxus dispatches each kind through a separate
//! handler.
//!
//! ## Pointer-capture
//!
//! `setPointerCapture` and `releasePointerCapture` are not exposed on
//! `PointerData`, so we reach into the webview through `dom::eval`
//! (see [`set_pointer_capture`] and [`release_pointer_capture`]). The
//! supplied selector is up to the caller — typically the data-attribute
//! query the host attaches to its wrapper `<div>`.

#![allow(clippy::module_name_repetitions)]

use dioxus::events::PointerData;
use dioxus::html::point_interaction::{InteractionLocation, ModifiersInteraction, PointerInteraction};
use dioxus::html::input_data::keyboard_types::Modifiers;
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::Event;
use serde_json::json;

use rgraph_drag::PointerId;
use rgraph_zoom::PointerInput;

use crate::dom::eval::{
    eval_fire_and_forget, format_snippet, RELEASE_POINTER_CAPTURE_JS, SET_POINTER_CAPTURE_JS,
};
use crate::dom::wheel::PaneBounds;

/// Which pointer-event handler dispatched the event. The translation
/// to [`PointerInput`] depends on the kind because the [`Down`] variant
/// carries `button + ctrl` while the others don't.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerEventKind {
    Down,
    Move,
    Up,
    Cancel,
}

/// Convert a Dioxus pointer event to the `rgraph_zoom::PointerInput`
/// variant matching the supplied [`PointerEventKind`]. Returns `None`
/// when the event lacks a usable pointer id (shouldn't happen for
/// pointer events emitted by Wry, but the API stays defensive).
///
/// `D` is the per-gesture datum type — usually `()` for the pane-level
/// pan/zoom and a node id for per-node drags. Phase 5's `use_drag`
/// will pass the node id through here; Phase 4 always uses `()`.
#[must_use]
pub fn from_dioxus<D>(
    event: &Event<PointerData>,
    kind: PointerEventKind,
    bounds: PaneBounds,
    datum: Option<D>,
) -> PointerInput<D> {
    let data: &PointerData = event;
    let id = pointer_id_from(data);
    let client = <PointerData as InteractionLocation>::client_coordinates(data);
    let x = client.x - bounds.x;
    let y = client.y - bounds.y;

    match kind {
        PointerEventKind::Down => {
            let button = match <PointerData as PointerInteraction>::trigger_button(data) {
                Some(MouseButton::Primary) => 0,
                Some(MouseButton::Auxiliary) => 1,
                Some(MouseButton::Secondary) => 2,
                Some(MouseButton::Fourth) => 3,
                Some(MouseButton::Fifth) => 4,
                _ => 0,
            };
            let mods = <PointerData as ModifiersInteraction>::modifiers(data);
            PointerInput::Down {
                id,
                x,
                y,
                button,
                ctrl: mods.contains(Modifiers::CONTROL),
                datum,
            }
        }
        PointerEventKind::Move => PointerInput::Move { id, x, y },
        PointerEventKind::Up => PointerInput::Up { id, x, y },
        PointerEventKind::Cancel => PointerInput::Cancel { id },
    }
}

/// Map a Dioxus pointer to one of the three [`PointerId`] flavours
/// recognised by `rgraph_drag` / `rgraph_zoom`:
///
/// * `'mouse'` → [`PointerId::Mouse`].
/// * `'touch'` → [`PointerId::Touch(id)`].
/// * everything else (`'pen'`, `'unknown'`) → [`PointerId::Pointer(id)`].
fn pointer_id_from(data: &PointerData) -> PointerId {
    let kind = data.pointer_type();
    let id_i32 = data.pointer_id();
    match kind.as_str() {
        "mouse" => PointerId::Mouse,
        "touch" => PointerId::Touch(id_i32 as u64),
        _ => PointerId::Pointer(id_i32 as u64),
    }
}

/// Imperatively attach a pointer to the element matching the supplied
/// selector via the webview's `setPointerCapture` API.
///
/// `pointer_id` is the raw `PointerEvent.pointerId` — for Phase 4 we
/// expose the i32 directly because pointer-capture is per-browser-id,
/// not per-`PointerId` enum.
pub fn set_pointer_capture(selector: &str, pointer_id: i32) {
    let js = format_snippet(
        SET_POINTER_CAPTURE_JS,
        &[
            ("$SELECTOR$", &json!(selector)),
            ("$POINTER_ID$", &json!(pointer_id)),
        ],
    );
    eval_fire_and_forget(&js);
}

/// Inverse of [`set_pointer_capture`].
pub fn release_pointer_capture(selector: &str, pointer_id: i32) {
    let js = format_snippet(
        RELEASE_POINTER_CAPTURE_JS,
        &[
            ("$SELECTOR$", &json!(selector)),
            ("$POINTER_ID$", &json!(pointer_id)),
        ],
    );
    eval_fire_and_forget(&js);
}
