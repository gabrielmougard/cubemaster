//! Port of `xyflow-react/src/hooks/useOnInitHandler.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::{use_effect, use_hook};

use crate::hooks::use_rgraph::{use_rgraph, RGraphHandle};
use crate::types::general::OnInit;

/// Fires `on_init` exactly once after the viewport is initialised.
/// Mirrors TS `useOnInitHandler(onInit)`.
///
/// The TS source uses `useRef<boolean>(false)` to guarantee a single
/// fire; we replicate that with an `Rc<Cell<bool>>` captured by the
/// effect closure.
pub fn use_on_init_handler<N, E>(on_init: Option<OnInit<RGraphHandle<N, E>>>)
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let handle = use_rgraph::<N, E>();
    let initialized = use_hook(|| Rc::new(Cell::new(false)));

    use_effect(move || {
        if !initialized.get()
            && handle.viewport_initialized()
            && let Some(cb) = on_init
        {
            cb.call(handle);
            initialized.set(true);
        }
    });
}
