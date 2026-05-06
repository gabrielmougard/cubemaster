use dioxus::prelude::*;
use crate::components::connection_header::ConnectionHeader;
use crate::components::icons::*;
use crate::components::sidebar::{NavItem, Sidebar};
use crate::state::use_app_state;

#[component]
pub fn DashboardView() -> Element {
    let app_state = use_app_state();
    let state = app_state.read();

    rsx! {
        div { class: "app-layout",
            Sidebar { active: NavItem::Dashboard }
            main { class: "main-content",
                ConnectionHeader {}

                if let Some(ref cube) = state.connected_cube {
                    ConnectedDashboard {
                        cube_name: cube.friendly_name.clone(),
                        device_id: cube.device_id.clone(),
                        rssi: cube.rssi,
                    }
                } else {
                    div { class: "empty-state",
                        IconCube { class: "empty-icon".to_string() }
                        h2 { class: "empty-title", "No Cube Connected" }
                        p { "Go to Discover to scan for nearby cubes and pair." }
                    }
                }
            }
        }
    }
}

#[component]
fn ConnectedDashboard(cube_name: String, device_id: String, rssi: Option<i16>) -> Element {
    let ble_value = rssi.map(|r| format!("{r} dBm")).unwrap_or("-".into());
    let ble_status = match rssi {
        Some(r) if r > -50 => "good",
        Some(r) if r > -70 => "warn",
        Some(_) => "bad",
        None => "off",
    };

    rsx! {
        div { class: "view-header",
            h1 { class: "view-title", "{cube_name}" }
        }

        div { class: "dashboard-grid",
            StatusCard {
                title: "BLE Signal".to_string(),
                value: ble_value,
                icon: "bluetooth".to_string(),
                status: ble_status.to_string(),
            }
            StatusCard {
                title: "Device Address".to_string(),
                value: device_id.clone(),
                icon: "cube".to_string(),
                status: "good".to_string(),
            }
            StatusCard {
                title: "WiFi".to_string(),
                value: "Not configured".to_string(),
                icon: "wifi".to_string(),
                status: "off".to_string(),
            }
            StatusCard {
                title: "Firmware".to_string(),
                value: "0.1.0".to_string(),
                icon: "cube".to_string(),
                status: "good".to_string(),
            }
            StatusCard {
                title: "GATT Services".to_string(),
                value: "Connected".to_string(),
                icon: "bluetooth".to_string(),
                status: "good".to_string(),
            }
            StatusCard {
                title: "Storage".to_string(),
                value: "N/A (no SD)".to_string(),
                icon: "cube".to_string(),
                status: "off".to_string(),
            }
        }

        div { class: "actions-section",
            h2 { class: "section-title", "Quick Actions" }
            div { class: "action-buttons",
                button { class: "btn btn-primary",
                    IconWifi { class: "btn-icon".to_string() }
                    span { "Setup WiFi" }
                }
                button { class: "btn btn-secondary",
                    IconRefresh { class: "btn-icon".to_string() }
                    span { "Refresh" }
                }
            }
        }
    }
}

#[component]
fn StatusCard(title: String, value: String, icon: String, status: String) -> Element {
    let card_class = format!("status-card status-{status}");
    let dot_class = format!("status-indicator status-dot-{status}");

    rsx! {
        div { class: "{card_class}",
            div { class: "status-card-icon",
                if icon == "bluetooth" {
                    IconBluetooth { class: "card-icon".to_string() }
                } else if icon == "wifi" {
                    IconWifi { class: "card-icon".to_string() }
                } else {
                    IconCube { class: "card-icon".to_string() }
                }
            }
            div { class: "status-card-content",
                span { class: "status-card-title", "{title}" }
                span { class: "status-card-value", "{value}" }
            }
            div { class: "{dot_class}" }
        }
    }
}
