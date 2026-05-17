//! Port of `xyflow-react/src/components/A11yDescriptions/index.tsx`.
//!
//! Status: Phase 4 — implemented.
//!
//! Renders three hidden `<div>`s used by screen readers:
//!
//! * `react-flow__node-desc-{rfId}` — describes node interactions.
//! * `react-flow__edge-desc-{rfId}` — describes edge interactions.
//! * `react-flow__aria-live-{rfId}` — assertive aria-live region for
//!   keyboard-driven movement announcements.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

/// Selector prefix for the node-description hidden div.
pub const ARIA_NODE_DESC_KEY: &str = "react-flow__node-desc";

/// Selector prefix for the edge-description hidden div.
pub const ARIA_EDGE_DESC_KEY: &str = "react-flow__edge-desc";

/// Selector prefix for the aria-live announcement div.
pub const ARIA_LIVE_MESSAGE: &str = "react-flow__aria-live";

const HIDDEN_STYLE: &str = "display:none;";
const ARIA_LIVE_STYLE: &str =
    "position:absolute;width:1px;height:1px;margin:-1px;border:0;padding:0;\
     overflow:hidden;clip:rect(0px,0px,0px,0px);clip-path:inset(100%);";

#[derive(Props, Clone, PartialEq)]
pub struct A11yDescriptionsProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    pub rf_id: String,
    pub disable_keyboard_a11y: bool,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn A11yDescriptions<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: A11yDescriptionsProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let aria_label_config = store.aria_label_config.read();

    let node_desc = if props.disable_keyboard_a11y {
        aria_label_config.node_a11y_description_default.clone()
    } else {
        aria_label_config
            .node_a11y_description_keyboard_disabled
            .clone()
    };
    let edge_desc = aria_label_config.edge_a11y_description_default.clone();

    let node_desc_id = format!("{}-{}", ARIA_NODE_DESC_KEY, props.rf_id);
    let edge_desc_id = format!("{}-{}", ARIA_EDGE_DESC_KEY, props.rf_id);

    rsx! {
        div { id: "{node_desc_id}", style: "{HIDDEN_STYLE}", "{node_desc}" }
        div { id: "{edge_desc_id}", style: "{HIDDEN_STYLE}", "{edge_desc}" }
        if !props.disable_keyboard_a11y {
            AriaLiveMessage::<N, E> { rf_id: props.rf_id.clone() }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct AriaLiveMessageProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    pub rf_id: String,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
fn AriaLiveMessage<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: AriaLiveMessageProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let msg = store.aria_live_message.read().clone();
    let id = format!("{}-{}", ARIA_LIVE_MESSAGE, props.rf_id);

    rsx! {
        div {
            id: "{id}",
            "aria-live": "assertive",
            "aria-atomic": "true",
            style: "{ARIA_LIVE_STYLE}",
            "{msg}"
        }
    }
}
