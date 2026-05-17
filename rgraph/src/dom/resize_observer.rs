//! `dom::resize_observer` — JS-bridged `ResizeObserver` for measuring
//! the wrapper and per-node DOM elements.
//!
//! Status: Phase 4 — partially implemented.
//!
//! ## What this module does
//!
//! * [`install_once`] — installs the `ResizeObserver` JS shim into
//!   `window` exactly once per webview. The shim exposes
//!   `window.__rgraph_observe(selector, key)` and
//!   `window.__rgraph_unobserve(key)` as the imperative entry points.
//!   The shim writes the most-recent measurement for each key into a
//!   `window.__rgraph_sizes` map; the Rust side reads it back via
//!   [`get_size`]. This polling model fits Dioxus 0.7 desktop, which
//!   doesn't yet expose a stable JS→Rust send channel.
//!
//! * [`get_size`] — fetch the most-recent `(width, height)` for a
//!   given key (or `None` if the element hasn't been measured yet).
//!
//! ## Phase 5 plan
//!
//! Phase 5 will extend the shim with a `dioxus.send` call so the
//! observer pushes deltas into the Rust runtime instead of waiting
//! for a poll. Until then, [`crate::hooks::use_resize_handler`] uses
//! Dioxus' native `MountedData::get_client_rect()` for the wrapper
//! (which is awaited once on mount and after window-resize). Per-node
//! sizes are sourced from the user-supplied `node.measured` /
//! `width` / `height` until Phase 5 turns this on.

#![allow(clippy::module_name_repetitions)]

use serde::Deserialize;
use serde_json::json;

use crate::dom::eval::{eval_fire_and_forget, eval_get, format_snippet};

/// JS shim that defines `window.__rgraph_observe`,
/// `window.__rgraph_unobserve`, and `window.__rgraph_sizes`. Idempotent
/// — re-running it has no effect because the global names are checked
/// before installation.
const INSTALL_JS: &str = r#"
(function() {
  if (window.__rgraph_resize_installed) return;
  window.__rgraph_resize_installed = true;
  window.__rgraph_sizes = window.__rgraph_sizes || {};
  const targets = new Map();
  const observer = new ResizeObserver(entries => {
    for (const entry of entries) {
      const key = targets.get(entry.target);
      if (!key) continue;
      const r = entry.contentRect || entry.target.getBoundingClientRect();
      window.__rgraph_sizes[key] = { width: r.width, height: r.height };
    }
  });
  window.__rgraph_observe = function(selector, key) {
    const el = document.querySelector(selector);
    if (!el) return false;
    targets.set(el, key);
    observer.observe(el);
    // Seed the size table immediately so the first poll succeeds.
    const r = el.getBoundingClientRect();
    window.__rgraph_sizes[key] = { width: r.width, height: r.height };
    return true;
  };
  window.__rgraph_unobserve = function(key) {
    for (const [el, k] of targets.entries()) {
      if (k === key) { targets.delete(el); observer.unobserve(el); break; }
    }
    delete window.__rgraph_sizes[key];
  };
})();
"#;

/// JS that returns `window.__rgraph_sizes[key]` or `null`.
const GET_SIZE_JS: &str = r#"
(function(key) {
  const s = (window.__rgraph_sizes || {})[key];
  return s ? { width: s.width, height: s.height } : null;
})($KEY$)
"#;

/// JS that runs `window.__rgraph_observe(selector, key)` synchronously
/// and returns the boolean result.
const OBSERVE_JS: &str = r#"
(function(selector, key) {
  if (typeof window.__rgraph_observe !== 'function') return false;
  return window.__rgraph_observe(selector, key);
})($SELECTOR$, $KEY$)
"#;

/// JS that calls `window.__rgraph_unobserve(key)`.
const UNOBSERVE_JS: &str = r#"
(function(key) {
  if (typeof window.__rgraph_unobserve === 'function') {
    window.__rgraph_unobserve(key);
  }
})($KEY$);
"#;

/// `(width, height)` snapshot returned by [`get_size`].
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct ObservedSize {
    pub width: f64,
    pub height: f64,
}

/// Install the JS shim (`window.__rgraph_*`) into the webview once.
/// Subsequent calls are cheap no-ops thanks to the
/// `__rgraph_resize_installed` guard.
pub fn install_once() {
    eval_fire_and_forget(INSTALL_JS);
}

/// Start observing the element matching `selector` under the supplied
/// `key`. Returns a future that resolves to `true` if the element was
/// found and observation began.
pub async fn observe(selector: &str, key: &str) -> bool {
    let js = format_snippet(
        OBSERVE_JS,
        &[
            ("$SELECTOR$", &json!(selector)),
            ("$KEY$", &json!(key)),
        ],
    );
    eval_get::<bool>(&js).await.unwrap_or(false)
}

/// Stop observing the element registered under `key`.
pub fn unobserve(key: &str) {
    let js = format_snippet(UNOBSERVE_JS, &[("$KEY$", &json!(key))]);
    eval_fire_and_forget(&js);
}

/// Fetch the most-recent observed `(width, height)` for `key`. Returns
/// `None` if the key is unknown (e.g. observation hasn't started or
/// the element has been unobserved).
pub async fn get_size(key: &str) -> Option<ObservedSize> {
    let js = format_snippet(GET_SIZE_JS, &[("$KEY$", &json!(key))]);
    eval_get::<Option<ObservedSize>>(&js).await.unwrap_or(None)
}
