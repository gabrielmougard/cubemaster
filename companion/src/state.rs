//! Global application state shared across views via Dioxus context.

use dioxus::prelude::*;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectedCube {
    /// The advertised BLE name of the cube (e.g. "CubeMaster-1E28").
    pub name: String,
    /// The unique device identifier (e.g. BLE MAC address).
    pub device_id: String,
    /// A short human-readable ID derived from the device.
    pub short_id: String,
    /// User-assigned friendly name (if renamed, otherwise same as `name`).
    pub friendly_name: String,
    /// Last observed signal strength (if available).
    pub rssi: Option<i16>,
    /// When the connection was established.
    pub connected_at: Instant,
}

/// Top-level reactive application state.
#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    /// The currently paired/connected cube, or `None` if disconnected.
    pub connected_cube: Option<ConnectedCube>,
    /// Whether a BLE scan is currently in progress.
    pub is_scanning: bool,
    /// WiFi IP address of the cube (if connected to WiFi).
    /// Set after successful WiFi provisioning when the cube reports its IP.
    pub cube_wifi_ip: Option<String>,
    /// The SSID the cube is connected to (if any).
    pub cube_wifi_ssid: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connected_cube: None,
            is_scanning: false,
            cube_wifi_ip: None,
            cube_wifi_ssid: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected_cube.is_some()
    }

    pub fn is_wifi_connected(&self) -> bool {
        self.cube_wifi_ip.is_some()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_app_state() {
    use_context_provider(|| Signal::new(AppState::new()));
}

pub fn use_app_state() -> Signal<AppState> {
    use_context::<Signal<AppState>>()
}
