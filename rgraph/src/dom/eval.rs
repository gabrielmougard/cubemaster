//! `dom::eval` — shared helpers around `dioxus::document::eval`.
//!
//! Status: Phase 4 — implemented.
//!
//! Tiny wrapper around `dioxus::document::eval` that provides:
//!
//! * [`eval_fire_and_forget`] — kick off a JS snippet without awaiting
//!   its result. Used for one-shot side-effects (e.g. `setPointerCapture`).
//! * [`eval_get`] — await a JS expression's value as a typed Rust
//!   struct.
//! * Snippet constants used by [`pointer`], [`wheel`], and
//!   [`resize_observer`] elsewhere in this module hierarchy.
//!
//! All snippets target the stock Dioxus desktop runtime (which exposes
//! a Wry/WebKit2GTK webview). The runtime exposes a `dioxus` global
//! with a `send` callback for JS→Rust messages, but for Phase 4 we
//! only ever return values via `await`-style `eval_get` calls.

#![allow(clippy::module_name_repetitions)]

use dioxus::document::{eval, Eval, EvalError};
use serde::de::DeserializeOwned;

/// JS that resolves to a `{ x, y, width, height }` object describing
/// the bounding client rect of the element matching the given
/// selector. Used by Phase 4 callers that have an element's id but
/// not a `MountedData` handle.
pub const GET_BOUNDING_CLIENT_RECT_JS: &str = r#"
(function(selector) {
  const el = document.querySelector(selector);
  if (!el) return null;
  const r = el.getBoundingClientRect();
  return { x: r.left, y: r.top, width: r.width, height: r.height };
})($SELECTOR$)
"#;

/// JS that returns `true` iff
/// `window.matchMedia('(prefers-color-scheme: dark)').matches` is true.
/// Used by [`crate::hooks::use_color_mode_class::use_color_mode_class`]
/// when the prop is `ColorMode::System`.
pub const PREFERS_DARK_JS: &str = r#"
(function() {
  if (typeof window === 'undefined' || !window.matchMedia) return false;
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
})()
"#;

/// JS that calls `element.setPointerCapture(pointerId)` on the first
/// element matching the supplied selector. Failures are silenced.
pub const SET_POINTER_CAPTURE_JS: &str = r#"
(function(selector, pid) {
  try {
    const el = document.querySelector(selector);
    if (el && el.setPointerCapture) el.setPointerCapture(pid);
  } catch (e) {}
})($SELECTOR$, $POINTER_ID$);
"#;

/// JS that calls `element.releasePointerCapture(pointerId)` on the
/// first element matching the supplied selector.
pub const RELEASE_POINTER_CAPTURE_JS: &str = r#"
(function(selector, pid) {
  try {
    const el = document.querySelector(selector);
    if (el && el.releasePointerCapture) el.releasePointerCapture(pid);
  } catch (e) {}
})($SELECTOR$, $POINTER_ID$);
"#;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Fire a JS snippet and immediately drop the resulting [`Eval`].
///
/// This is the right primitive for "set this attribute" / "call this
/// imperative API" snippets where we don't need the return value. The
/// underlying `Eval` still drives to completion in the webview;
/// dropping it here is exactly how Dioxus' own `document::eval`
/// callers use it for side-effect-only snippets.
pub fn eval_fire_and_forget(js: &str) {
    let _ = eval(js);
}

/// Run a JS snippet and await its result, parsed as `T`.
///
/// The snippet must end with a JS expression (not a statement) so the
/// webview's `eval` returns a value to the host. JSON-serialisable
/// values are deserialised through `serde`. Errors propagate the
/// underlying `EvalError`.
pub async fn eval_get<T: DeserializeOwned + 'static>(js: &str) -> Result<T, EvalError> {
    let e: Eval = eval(js);
    e.join::<T>().await
}

/// Substitute placeholder tokens (`$NAME$`) in a JS snippet template
/// with the supplied JSON-encoded values. Used to inline arguments
/// safely without a separate variable-binding pass.
///
/// # Example
/// ```ignore
/// use serde_json::json;
/// let js = format_snippet(SET_POINTER_CAPTURE_JS, &[
///     ("$SELECTOR$", &json!("#node-1")),
///     ("$POINTER_ID$", &json!(42)),
/// ]);
/// ```
#[must_use]
pub fn format_snippet(template: &str, params: &[(&str, &serde_json::Value)]) -> String {
    let mut out = template.to_string();
    for (name, value) in params {
        let json = value.to_string();
        out = out.replace(name, &json);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_snippet_replaces_tokens() {
        let s = format_snippet(
            SET_POINTER_CAPTURE_JS,
            &[
                ("$SELECTOR$", &json!("#node-1")),
                ("$POINTER_ID$", &json!(42)),
            ],
        );
        assert!(s.contains("\"#node-1\""));
        assert!(s.contains("42"));
        assert!(!s.contains("$SELECTOR$"));
    }

    #[test]
    fn format_snippet_handles_no_params() {
        let s = format_snippet(PREFERS_DARK_JS, &[]);
        assert_eq!(s, PREFERS_DARK_JS);
    }
}
