//! Port of `xyflow-react/src/hooks/useColorModeClass.ts`.
//!
//! Status: Phase 4 — implemented.
//!
//! Returns the resolved [`ColorModeClass`] for the supplied
//! [`ColorMode`]. For `Light` / `Dark` the result is determined
//! synchronously; for `System` the hook spawns a one-shot async query
//! to `window.matchMedia('(prefers-color-scheme: dark)')` through
//! [`crate::dom::eval`] and updates a signal once the answer arrives.
//!
//! ## Limitations
//!
//! The TS source also subscribes to `mediaQuery.addEventListener('change', …)`
//! so the class flips when the user toggles their system theme at
//! runtime. Phase 4 does *not* install that listener — Dioxus desktop
//! lacks a stable JS→Rust send channel as of 0.7. Phase 5 will pipe
//! `media-query change` events through the resize-observer shim's
//! polling table.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::{spawn, use_hook, use_signal, ReadableExt, Signal, WritableExt};

use rgraph_core::types::viewport::{ColorMode, ColorModeClass};

use crate::dom::eval::{eval_get, PREFERS_DARK_JS};

/// Returns a [`Signal<ColorModeClass>`] that tracks the resolved class.
///
/// * `Light` / `Dark` → seeded synchronously, never changes.
/// * `System` → seeded `Light`, then asynchronously updated to match
///   `prefers-color-scheme: dark` once the webview answers.
///
/// Unlike the TS source, the returned value is a *signal* rather than
/// a plain `ColorModeClass` because the `System` resolution is async.
/// Callers reading via `.read()` get the live value with no extra
/// effort.
#[must_use]
pub fn use_color_mode_class(color_mode: ColorMode) -> Signal<ColorModeClass> {
    let initial = match color_mode {
        ColorMode::Light => ColorModeClass::Light,
        ColorMode::Dark => ColorModeClass::Dark,
        ColorMode::System => ColorModeClass::Light,
    };
    let class = use_signal(|| initial);

    use_hook(move || {
        if !matches!(color_mode, ColorMode::System) {
            return;
        }
        let mut sink = class;
        spawn(async move {
            if let Ok(is_dark) = eval_get::<bool>(PREFERS_DARK_JS).await {
                let next = if is_dark { ColorModeClass::Dark } else { ColorModeClass::Light };
                if *sink.peek() != next {
                    sink.set(next);
                }
            }
        });
    });

    class
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn light_seeds_synchronously() {
        thread_local! { static SEEN: Cell<u8> = const { Cell::new(0) }; }
        fn Root() -> Element {
            let s = use_color_mode_class(ColorMode::Light);
            let v = matches!(*s.read(), ColorModeClass::Light);
            SEEN.with(|c| c.set(if v { 1 } else { 2 }));
            rsx! { div {} }
        }
        let mut vdom = VirtualDom::new(Root);
        let _ = vdom.rebuild_to_vec();
        assert_eq!(SEEN.with(|c| c.get()), 1);
    }

    #[test]
    fn dark_seeds_synchronously() {
        thread_local! { static SEEN: Cell<u8> = const { Cell::new(0) }; }
        fn Root() -> Element {
            let s = use_color_mode_class(ColorMode::Dark);
            let v = matches!(*s.read(), ColorModeClass::Dark);
            SEEN.with(|c| c.set(if v { 1 } else { 2 }));
            rsx! { div {} }
        }
        let mut vdom = VirtualDom::new(Root);
        let _ = vdom.rebuild_to_vec();
        assert_eq!(SEEN.with(|c| c.get()), 1);
    }

    #[test]
    fn system_seeds_light_initially() {
        // Without a webview backing, the async eval never resolves
        // and we keep the seed value.
        thread_local! { static SEEN: Cell<u8> = const { Cell::new(0) }; }
        fn Root() -> Element {
            let s = use_color_mode_class(ColorMode::System);
            let v = matches!(*s.read(), ColorModeClass::Light);
            SEEN.with(|c| c.set(if v { 1 } else { 2 }));
            rsx! { div {} }
        }
        let mut vdom = VirtualDom::new(Root);
        let _ = vdom.rebuild_to_vec();
        assert_eq!(SEEN.with(|c| c.get()), 1);
    }
}
