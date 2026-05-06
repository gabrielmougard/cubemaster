//! Global connection status header bar.

use dioxus::prelude::*;
use crate::state::use_app_state;
use crate::components::icons::*;

/// A thin header bar that shows connection status.
/// - Green: paired to a cube (shows name)
/// - Yellow: not paired to any cube
#[component]
pub fn ConnectionHeader() -> Element {
    let app_state = use_app_state();
    let state = app_state.read();

    if let Some(ref cube) = state.connected_cube {
        let display_name = &cube.friendly_name;
        rsx! {
            div { class: "conn-header conn-header-connected",
                IconCube { class: "conn-header-icon".to_string() }
                span { class: "conn-header-text", "Connected to {display_name}" }
                span { class: "conn-header-dot" }
            }
        }
    } else {
        rsx! {
            div { class: "conn-header conn-header-disconnected",
                IconBluetooth { class: "conn-header-icon".to_string() }
                span { class: "conn-header-text", "No cube connected" }
            }
        }
    }
}
