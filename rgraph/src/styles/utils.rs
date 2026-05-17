//! Port of `xyflow-react/src/styles/utils.ts`.
//!
//! Shared inline-style snippets used by multiple components. In Dioxus we
//! return them as string slices to embed in `style="…"` attributes.

/// `containerStyle` — absolute-positioned fill (top/left/0; 100%×100%).
///
/// TS reference: `xyflow-react/src/styles/utils.ts:3`.
pub const CONTAINER_STYLE: &str = "position:absolute;width:100%;height:100%;top:0;left:0;";
