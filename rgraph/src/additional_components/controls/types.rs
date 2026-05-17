//! Port of `xyflow-react/src/additional-components/Controls/types.ts`.
//!
//! Status: Phase 8 — implemented.
//!
//! Notes on TS-to-Rust mapping:
//! * `ButtonHTMLAttributes<HTMLButtonElement>` is collapsed to a small
//!   set of common props (`title`, `aria_label`, `disabled`,
//!   `class_name`, `on_click`). Extending this is straightforward.
//! * `FitViewOptions` reuses [`rgraph_core::types::viewport::FitViewOptionsBase`].

#![allow(clippy::module_name_repetitions)]

use rgraph_core::types::viewport::{FitViewOptionsBase, PanelPosition};

/// Mirrors the TS `orientation: 'horizontal' | 'vertical'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlsOrientation {
    Horizontal,
    #[default]
    Vertical,
}

impl ControlsOrientation {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ControlsOrientation::Horizontal => "horizontal",
            ControlsOrientation::Vertical => "vertical",
        }
    }
}

/// Default position used by `<Controls>`: `PanelPosition::BottomLeft`.
#[must_use]
pub fn default_controls_position() -> PanelPosition {
    PanelPosition::BottomLeft
}

/// Fit-view options exposed to `<Controls>`.
pub type ControlsFitViewOptions = FitViewOptionsBase;
