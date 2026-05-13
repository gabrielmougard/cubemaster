//! Port of `xyflow-core/src/constants.ts`.
//!
//! Status: implemented (phase 1).
//!
//! This module exposes the error catalogue, the infinite coordinate
//! extent, the keyboard-selection key list and the default aria-label
//! configuration used across the crate.

#![allow(clippy::module_name_repetitions)]

use crate::types::geometry::CoordinateExtent;
use crate::types::handles::HandleType;

/// `[[-∞, -∞], [+∞, +∞]]` — used as the default extent for unbounded
/// graphs and viewports.
pub const INFINITE_EXTENT: CoordinateExtent = [
    [f64::NEG_INFINITY, f64::NEG_INFINITY],
    [f64::INFINITY, f64::INFINITY],
];

/// Key codes that activate / deactivate keyboard selection of nodes
/// and edges.
///
/// Mirrors the TS `elementSelectionKeys = ['Enter', ' ', 'Escape']`.
pub const ELEMENT_SELECTION_KEYS: &[&str] = &["Enter", " ", "Escape"];

// ---------------------------------------------------------------------------
// Error catalogue
// ---------------------------------------------------------------------------

/// Identifier appended to the docs URL on every formatted error message
/// (e.g. `https://reactflow.dev/error#001`).
const ERR_DOCS_BASE: &str = "https://reactflow.dev/error#";

#[inline]
fn err_link(id: &str) -> String {
    format!("{ERR_DOCS_BASE}{id}")
}

/// Equivalent of TS `errorMessages.error001`.
#[must_use]
pub fn error_001() -> String {
    format!(
        "[React Flow]: Seems like you have not used zustand provider as an ancestor. Help: {}",
        err_link("001")
    )
}

/// Equivalent of TS `errorMessages.error002`.
#[must_use]
pub fn error_002() -> String {
    "It looks like you've created a new nodeTypes or edgeTypes object. \
     If this wasn't on purpose please define the nodeTypes/edgeTypes \
     outside of the component or memoize them."
        .to_string()
}

/// Equivalent of TS `errorMessages.error003(nodeType)`.
#[must_use]
pub fn error_003(node_type: &str) -> String {
    format!(r#"Node type "{node_type}" not found. Using fallback type "default"."#)
}

/// Equivalent of TS `errorMessages.error004`.
#[must_use]
pub fn error_004() -> String {
    "The React Flow parent container needs a width and a height to render the graph.".to_string()
}

/// Equivalent of TS `errorMessages.error005`.
#[must_use]
pub fn error_005() -> String {
    "Only child nodes can use a parent extent.".to_string()
}

/// Equivalent of TS `errorMessages.error006`.
#[must_use]
pub fn error_006() -> String {
    "Can't create edge. An edge needs a source and a target.".to_string()
}

/// Equivalent of TS `errorMessages.error007(id)`.
#[must_use]
pub fn error_007(id: &str) -> String {
    format!("The old edge with id={id} does not exist.")
}

/// Equivalent of TS `errorMessages.error008`.
///
/// `handle_type` selects which of `source_handle` / `target_handle` is
/// reported.
#[must_use]
pub fn error_008(
    handle_type: HandleType,
    id: &str,
    source_handle: Option<&str>,
    target_handle: Option<&str>,
) -> String {
    let handle_label = match handle_type {
        HandleType::Source => "source",
        HandleType::Target => "target",
    };
    let handle_id = match handle_type {
        HandleType::Source => source_handle,
        HandleType::Target => target_handle,
    }
    .unwrap_or("");
    format!(r#"Couldn't create edge for {handle_label} handle id: "{handle_id}", edge id: {id}."#)
}

/// Equivalent of TS `errorMessages.error009(type)`.
#[must_use]
pub fn error_009(marker_type: &str) -> String {
    format!(r#"Marker type "{marker_type}" doesn't exist."#)
}

/// Equivalent of TS `errorMessages.error010`.
#[must_use]
pub fn error_010() -> String {
    "Handle: No node id found. Make sure to only use a Handle inside a custom Node.".to_string()
}

/// Equivalent of TS `errorMessages.error011(edgeType)`.
#[must_use]
pub fn error_011(edge_type: &str) -> String {
    format!(r#"Edge type "{edge_type}" not found. Using fallback type "default"."#)
}

/// Equivalent of TS `errorMessages.error012(id)`.
#[must_use]
pub fn error_012(id: &str) -> String {
    format!(
        r#"Node with id "{id}" does not exist, it may have been removed. \
This can happen when a node is deleted before the "onNodeClick" handler is called."#
    )
}

/// Equivalent of TS `errorMessages.error013(lib)`.
///
/// `lib` defaults to `"react"` in the TS source. Pass `"dioxus"` from
/// the consumer crate.
#[must_use]
pub fn error_013(lib: Option<&str>) -> String {
    let lib = lib.unwrap_or("react");
    format!(
        r#"It seems that you haven't loaded the styles. Please import \
'@xyflow/{lib}/dist/style.css' or base.css to make sure everything is working properly."#
    )
}

/// Equivalent of TS `errorMessages.error014`.
#[must_use]
pub fn error_014() -> String {
    "useNodeConnections: No node ID found. Call useNodeConnections inside a custom Node or provide a node ID."
        .to_string()
}

/// Equivalent of TS `errorMessages.error015`.
#[must_use]
pub fn error_015() -> String {
    "It seems that you are trying to drag a node that is not initialized. Please use onNodesChange as explained in the docs."
        .to_string()
}

// ---------------------------------------------------------------------------
// Default aria-label configuration
// ---------------------------------------------------------------------------

/// Aria-label catalogue.
///
/// All fields are owned `String`s so callers can override individual
/// entries via the `..Default::default()` pattern.
///
/// Mirrors the TS `defaultAriaLabelConfig` and `AriaLabelConfig`. The
/// dynamic aria-live message accepting `{ direction, x, y }` is
/// modelled as a free function [`aria_live_message`] rather than a
/// boxed callback so the struct stays `Clone`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AriaLabelConfig {
    pub node_a11y_description_default: String,
    pub node_a11y_description_keyboard_disabled: String,
    pub edge_a11y_description_default: String,
    pub controls_aria_label: String,
    pub controls_zoom_in_aria_label: String,
    pub controls_zoom_out_aria_label: String,
    pub controls_fit_view_aria_label: String,
    pub controls_interactive_aria_label: String,
    pub minimap_aria_label: String,
    pub handle_aria_label: String,
}

impl AriaLabelConfig {
    /// Default English aria-label config (mirrors TS
    /// `defaultAriaLabelConfig`).
    #[must_use]
    pub fn default_en() -> Self {
        AriaLabelConfig {
            node_a11y_description_default:
                "Press enter or space to select a node. Press delete to remove it and escape to cancel.".into(),
            node_a11y_description_keyboard_disabled:
                "Press enter or space to select a node. You can then use the arrow keys to move the node around. \
                 Press delete to remove it and escape to cancel."
                    .into(),
            edge_a11y_description_default:
                "Press enter or space to select an edge. You can then press delete to remove it or escape to cancel."
                    .into(),
            controls_aria_label: "Control Panel".into(),
            controls_zoom_in_aria_label: "Zoom In".into(),
            controls_zoom_out_aria_label: "Zoom Out".into(),
            controls_fit_view_aria_label: "Fit View".into(),
            controls_interactive_aria_label: "Toggle Interactivity".into(),
            minimap_aria_label: "Mini Map".into(),
            handle_aria_label: "Handle".into(),
        }
    }
}

impl Default for AriaLabelConfig {
    fn default() -> Self {
        Self::default_en()
    }
}

/// Format the aria-live announcement spoken when a selected node is
/// moved with the arrow keys.
///
/// Equivalent of TS `defaultAriaLabelConfig['node.a11yDescription.ariaLiveMessage']`.
#[must_use]
pub fn aria_live_message(direction: &str, x: f64, y: f64) -> String {
    format!("Moved selected node {direction}. New position, x: {x}, y: {y}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infinite_extent_is_signed_infinities() {
        assert!(INFINITE_EXTENT[0][0].is_infinite() && INFINITE_EXTENT[0][0].is_sign_negative());
        assert!(INFINITE_EXTENT[1][1].is_infinite() && INFINITE_EXTENT[1][1].is_sign_positive());
    }

    #[test]
    fn element_selection_keys_match_ts() {
        assert_eq!(ELEMENT_SELECTION_KEYS, &["Enter", " ", "Escape"]);
    }

    #[test]
    fn errors_format_their_arguments() {
        assert!(error_003("foo").contains("\"foo\""));
        assert!(error_007("e1").contains("e1"));
        assert!(error_008(HandleType::Source, "e1", Some("h1"), None).contains("source"));
        assert!(error_008(HandleType::Target, "e1", None, Some("h2")).contains("\"h2\""));
        assert!(error_011("custom").contains("\"custom\""));
        assert!(error_012("n1").contains("n1"));
        assert!(error_013(Some("dioxus")).contains("@xyflow/dioxus"));
        assert!(error_013(None).contains("@xyflow/react"));
    }

    #[test]
    fn aria_live_format() {
        let s = aria_live_message("up", 1.5, -2.0);
        assert!(s.contains("up"));
        assert!(s.contains("1.5"));
        assert!(s.contains("-2"));
    }

    #[test]
    fn aria_label_config_default_has_all_strings_populated() {
        let c = AriaLabelConfig::default();
        assert!(!c.controls_aria_label.is_empty());
        assert!(!c.minimap_aria_label.is_empty());
        assert!(!c.handle_aria_label.is_empty());
    }
}
