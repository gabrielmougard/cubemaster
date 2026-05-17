//! Port of `xyflow-react/src/container/GraphView/useStylesLoadedWarning.ts`.
//!
//! Status: Phase 7 — implemented.
//!
//! Dev-only warning: emits a one-shot `error013` through `on_error`
//! when the stylesheet hasn't been injected (detected via an
//! async query to `window.getComputedStyle(.react-flow__pane).zIndex`).
//!
//! In the Dioxus desktop port we run the check through `dom::eval`
//! once on first mount; the result is observed asynchronously and
//! the warning fires only when the computed z-index doesn't match the
//! expected `"1"`.

#![allow(clippy::module_name_repetitions)]

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::{spawn, use_hook, ReadableExt};

use crate::context::use_rgraph_store;
use crate::dom::eval::eval_get;
use crate::types::component_props::OnErrorArgs;

const PROBE_JS: &str = r#"
(function() {
  try {
    const pane = document.querySelector('.react-flow__pane');
    if (!pane) return null;
    return window.getComputedStyle(pane).zIndex;
  } catch (e) {
    return null;
  }
})()
"#;

pub fn use_styles_loaded_warning<N, E>()
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    if !cfg!(debug_assertions) {
        return;
    }

    let store = use_rgraph_store::<N, E>();
    let checked: Rc<Cell<bool>> = use_hook(|| Rc::new(Cell::new(false)));

    use_hook(move || {
        spawn(async move {
            if checked.get() {
                return;
            }
            let Ok(z) = eval_get::<Option<String>>(PROBE_JS).await else {
                checked.set(true);
                return;
            };
            checked.set(true);
            // The expected value is `"1"`. Anything else — including
            // `None` — means the stylesheet hasn't loaded.
            if z.as_deref() != Some("1")
                && let Some(handler) = *store.on_error.peek()
            {
                handler.call(OnErrorArgs {
                    id: "013".to_string(),
                    message: "[rgraph]: It looks like the rgraph styles haven't been loaded. \
                              Please import the base CSS via `rgraph::styles::BASE_CSS`."
                        .to_string(),
                });
            }
        });
    });
}
