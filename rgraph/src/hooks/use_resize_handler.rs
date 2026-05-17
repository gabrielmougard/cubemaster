//! Port of `xyflow-react/src/hooks/useResizeHandler.ts`.
//!
//! Status: Phase 4 — implemented.
//!
//! ## What this hook does
//!
//! Watches the wrapper element's bounding rect and writes its size
//! into [`crate::store::RGraphStore`]'s `width` / `height` signals.
//! The TS source uses a `ResizeObserver` plus a window-level `resize`
//! listener; we mirror that with our [`crate::dom::resize_observer`]
//! shim.
//!
//! Usage from a component:
//!
//! ```ignore
//! fn ZoomPane() -> Element {
//!     let on_mounted = use_resize_handler::<(), ()>("rgraph-wrapper");
//!     rsx! {
//!         div {
//!             id: "rgraph-wrapper",
//!             onmounted: on_mounted,
//!             /* ... */
//!         }
//!     }
//! }
//! ```
//!
//! The supplied id must match the `id` attribute on the wrapper
//! `<div>` so the JS shim can resolve it via
//! `document.querySelector('#…')`.

#![allow(clippy::module_name_repetitions)]

use dioxus::events::MountedData;
use dioxus::prelude::{spawn, use_hook, Callback, Event, ReadableExt, WritableExt};

use crate::context::use_rgraph_store;
use crate::dom::resize_observer;
use crate::store::RGraphStore;

/// Returns an `onmounted` handler that wires the wrapper element to
/// the resize-observer shim. Mirrors TS `useResizeHandler(domNodeRef)`.
///
/// The `wrapper_id` is the HTML `id` of the element being measured;
/// it doubles as the [`resize_observer`] key.
#[must_use]
pub fn use_resize_handler<N, E>(wrapper_id: &str) -> Callback<Event<MountedData>>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    let id = wrapper_id.to_string();

    // Install the JS shim once per webview lifetime.
    use_hook(|| {
        resize_observer::install_once();
    });

    Callback::new(move |evt: Event<MountedData>| {
        let id = id.clone();
        let store = store;
        spawn(async move {
            // First measurement via `MountedData` — fast, doesn't
            // depend on the JS shim at all.
            if let Ok(rect) = evt.get_client_rect().await {
                let w = rect.size.width;
                let h = rect.size.height;
                if let Some((width, height)) = sane_size(w, h) {
                    write_size(store, width, height);
                }
                // Cache the full bbox (including origin) so
                // `screen_to_flow_position` / `flow_to_screen_position`
                // can subtract the wrapper's document offset.
                write_bbox(
                    store,
                    crate::dom::PaneBounds {
                        x: rect.origin.x,
                        y: rect.origin.y,
                        width: rect.size.width,
                        height: rect.size.height,
                    },
                );
            }

            // Then attach the shim so subsequent resizes feed back
            // through `__rgraph_sizes`. The id-based selector targets
            // the same element we just measured.
            let selector = format!("#{id}");
            let _attached = resize_observer::observe(&selector, &id).await;

            // Note: subsequent dimension polls need to be triggered by
            // a per-frame loop (e.g. inside the host's render closure).
            // For now the host should call `poll_resize_handler` from
            // a `use_future` if it expects continuous updates. Phase 5
            // will swap this for a push-based stream once the JS shim
            // gains a `dioxus.send` channel.
        });
    })
}

/// Manually fetch the latest size from the resize-observer shim and
/// write it into the store. Hosts call this from a polling loop or
/// after an event that may have changed the wrapper's size.
pub async fn poll_resize_handler<N, E>(store: RGraphStore<N, E>, wrapper_id: &str) -> bool
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let Some(size) = resize_observer::get_size(wrapper_id).await else {
        return false;
    };
    if let Some((w, h)) = sane_size(size.width, size.height) {
        write_size(store, w, h);
        true
    } else {
        false
    }
}

fn sane_size(w: f64, h: f64) -> Option<(f64, f64)> {
    if w.is_finite() && h.is_finite() && w >= 0.0 && h >= 0.0 {
        // The TS source falls back to 500×500 when the measured size
        // is zero (which would otherwise cancel the fit-view logic).
        let width = if w == 0.0 { 500.0 } else { w };
        let height = if h == 0.0 { 500.0 } else { h };
        Some((width, height))
    } else {
        None
    }
}

fn write_size<N, E>(store: RGraphStore<N, E>, width: f64, height: f64)
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    if (*store.width.peek() - width).abs() > f64::EPSILON {
        store.width.clone().set(width);
    }
    if (*store.height.peek() - height).abs() > f64::EPSILON {
        store.height.clone().set(height);
    }
}

fn write_bbox<N, E>(store: RGraphStore<N, E>, bbox: crate::dom::PaneBounds)
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    if *store.dom_bbox.peek() != bbox {
        store.dom_bbox.clone().set(bbox);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sane_size_fills_zero() {
        assert_eq!(sane_size(0.0, 0.0), Some((500.0, 500.0)));
        assert_eq!(sane_size(800.0, 0.0), Some((800.0, 500.0)));
        assert_eq!(sane_size(0.0, 600.0), Some((500.0, 600.0)));
    }

    #[test]
    fn sane_size_rejects_invalid() {
        assert!(sane_size(f64::NAN, 100.0).is_none());
        assert!(sane_size(100.0, -1.0).is_none());
        assert!(sane_size(f64::INFINITY, 100.0).is_none());
    }
}
