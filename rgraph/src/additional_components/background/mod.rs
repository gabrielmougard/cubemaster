//! Port of `xyflow-react/src/additional-components/Background/`.
//!
//! Status: Phase 8 — implemented.

pub mod background;
pub mod patterns;
pub mod types;

pub use background::{Background, BackgroundProps};
pub use patterns::{DotPattern, DotPatternProps, LinePattern, LinePatternProps};
pub use types::{BackgroundGap, BackgroundOffset, BackgroundVariant};
