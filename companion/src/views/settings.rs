use dioxus::prelude::*;
use crate::components::connection_header::ConnectionHeader;
use crate::components::icons::*;
use crate::components::sidebar::{NavItem, Sidebar};
use crate::state::use_app_state;

/// UUIDs matching the firmware's GATT characteristics.
const CUBE_NAME_CHAR_UUID: &str = "c0bea577-0000-4000-8000-00000000f004";
const WIFI_SSID_CHAR_UUID: &str = "c0bea577-0000-4000-8000-00000000f011";
const WIFI_PASS_CHAR_UUID: &str = "c0bea577-0000-4000-8000-00000000f012";

#[derive(Debug, Clone, PartialEq)]
enum ActionStatus {
    Idle,
    InProgress(String),
    Success(String),
    Error(String),
}

#[component]
pub fn SettingsView() -> Element {
    let mut app_state = use_app_state();
    let mut wifi_ssid = use_signal(String::new);
    let mut wifi_password = use_signal(String::new);
    let mut cube_name_input = use_signal(String::new);
    let mut show_password = use_signal(|| false);
    let mut action_status = use_signal(|| ActionStatus::Idle);
    let mut wifi_networks = use_signal(Vec::<String>::new);

    let is_connected = app_state.read().is_connected();
    let connected_name = app_state
        .read()
        .connected_cube
        .as_ref()
        .map(|c| c.friendly_name.clone())
        .unwrap_or_default();

    // Scan host WiFi networks on mount
    let _scan_wifi = use_effect(move || {
        spawn(async move {
            match scan_host_wifi_networks().await {
                Ok(networks) => wifi_networks.set(networks),
                Err(e) => tracing::warn!("Failed to scan WiFi networks: {}", e),
            }
        });
    });

    let on_disconnect = move |_| {
        spawn(async move {
            tracing::info!("Disconnect button clicked");
            if let Err(e) = crate::views::discover::disconnect_ble().await {
                tracing::warn!("BLE disconnect error: {}", e);
            }
            app_state.write().connected_cube = None;
        });
    };

    let on_rename = move |_| {
        let new_name = cube_name_input.read().clone();
        if new_name.is_empty() {
            action_status.set(ActionStatus::Error("Name cannot be empty".into()));
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                action_status.set(ActionStatus::Idle);
            });
            return;
        }
        action_status.set(ActionStatus::InProgress("Renaming...".into()));

        spawn(async move {
            match write_ble_characteristic(CUBE_NAME_CHAR_UUID, new_name.as_bytes()).await {
                Ok(()) => {
                    action_status.set(ActionStatus::Success(format!("Renamed to '{}'", new_name)));
                    // Update the friendly name in global app state
                    if let Some(ref mut cube) = app_state.write().connected_cube {
                        cube.friendly_name = new_name.clone();
                    }
                }
                Err(e) => {
                    action_status.set(ActionStatus::Error(format!("Rename failed: {e}")));
                }
            }
            // Auto-dismiss after 4 seconds
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            action_status.set(ActionStatus::Idle);
        });
    };

    let on_send_wifi = move |_| {
        let ssid = wifi_ssid.read().clone();
        let password = wifi_password.read().clone();
        if ssid.is_empty() {
            action_status.set(ActionStatus::Error("SSID cannot be empty".into()));
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                action_status.set(ActionStatus::Idle);
            });
            return;
        }
        action_status.set(ActionStatus::InProgress("Sending WiFi credentials...".into()));

        spawn(async move {
            // Write SSID first, then password
            if let Err(e) = write_ble_characteristic(WIFI_SSID_CHAR_UUID, ssid.as_bytes()).await {
                action_status.set(ActionStatus::Error(format!("Failed to send SSID: {e}")));
                tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                action_status.set(ActionStatus::Idle);
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            if let Err(e) = write_ble_characteristic(WIFI_PASS_CHAR_UUID, password.as_bytes()).await {
                action_status.set(ActionStatus::Error(format!("Failed to send password: {e}")));
                tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                action_status.set(ActionStatus::Idle);
                return;
            }
            action_status.set(ActionStatus::Success(format!("WiFi credentials sent for '{}'", ssid)));
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            action_status.set(ActionStatus::Idle);
        });
    };

    rsx! {
        div { class: "app-layout",
            Sidebar { active: NavItem::Settings }
            main { class: "main-content",
                ConnectionHeader {}

                div { class: "view-header",
                    h1 { class: "view-title", "Settings" }
                }

                // Action status banner
                {
                    let status_banner = match &*action_status.read() {
                        ActionStatus::InProgress(msg) => {
                            let m = msg.clone();
                            rsx! { div { class: "status-banner banner-connecting", "{m}" } }
                        }
                        ActionStatus::Success(msg) => {
                            let m = msg.clone();
                            rsx! { div { class: "status-banner banner-success", "{m}" } }
                        }
                        ActionStatus::Error(msg) => {
                            let m = msg.clone();
                            rsx! { div { class: "status-banner banner-error", "{m}" } }
                        }
                        ActionStatus::Idle => rsx! {},
                    };
                    status_banner
                }

                div { class: "settings-sections",
                    if is_connected {
                        section { class: "settings-section",
                            h2 { class: "section-title",
                                IconCube { class: "section-icon".to_string() }
                                span { "Connected Cube" }
                            }
                            div { class: "paired-item",
                                div { class: "paired-item-info",
                                    IconCube { class: "paired-icon".to_string() }
                                    div { class: "paired-details",
                                        span { class: "paired-name", "{connected_name}" }
                                        span { class: "paired-id", "Active BLE connection" }
                                    }
                                }
                                div { class: "paired-item-meta",
                                    button {
                                        class: "btn btn-danger btn-sm",
                                        onclick: on_disconnect,
                                        "Disconnect"
                                    }
                                }
                            }
                        }
                    }

                    section { class: "settings-section",
                        h2 { class: "section-title",
                            IconCube { class: "section-icon".to_string() }
                            span { "Cube Name" }
                        }
                        p { class: "section-desc",
                            "Rename your cube. Requires an active BLE connection."
                        }
                        div { class: "input-group input-row",
                            input {
                                r#type: "text",
                                class: "input",
                                placeholder: "Enter new cube name...",
                                disabled: !is_connected,
                                value: "{cube_name_input}",
                                oninput: move |e| cube_name_input.set(e.value()),
                            }
                            button {
                                class: "btn btn-primary",
                                disabled: !is_connected,
                                onclick: on_rename,
                                "Rename"
                            }
                        }
                    }

                    section { class: "settings-section",
                        h2 { class: "section-title",
                            IconWifi { class: "section-icon".to_string() }
                            span { "WiFi Configuration" }
                        }
                        p { class: "section-desc",
                            "Send WiFi credentials to the cube for bulk file transfers. Requires an active BLE connection."
                        }
                        div { class: "form-fields",
                            div { class: "input-group",
                                label { class: "input-label", "SSID" }
                                select {
                                    class: "input",
                                    disabled: !is_connected,
                                    value: "{wifi_ssid}",
                                    onchange: move |e| wifi_ssid.set(e.value()),
                                    option { value: "", "Select a network..." }
                                    for network in wifi_networks.read().iter() {
                                        option { value: "{network}", "{network}" }
                                    }
                                }
                            }
                            div { class: "input-group",
                                label { class: "input-label", "Password" }
                                div { class: "password-input-row",
                                    input {
                                        r#type: if *show_password.read() { "text" } else { "password" },
                                        class: "input",
                                        placeholder: "WiFi password",
                                        disabled: !is_connected,
                                        value: "{wifi_password}",
                                        oninput: move |e| wifi_password.set(e.value()),
                                    }
                                    button {
                                        class: "btn btn-ghost",
                                        onclick: move |_| show_password.toggle(),
                                        if *show_password.read() { "Hide" } else { "Show" }
                                    }
                                }
                            }
                            button {
                                class: "btn btn-primary",
                                disabled: !is_connected,
                                onclick: on_send_wifi,
                                IconWifi { class: "btn-icon".to_string() }
                                span { "Send to Cube" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Write a value to a BLE characteristic on the currently connected cube.
/// If the connection was lost, attempts to reconnect first.
async fn write_ble_characteristic(uuid_str: &str, data: &[u8]) -> Result<(), String> {
    use btleplug::api::{Central, Peripheral as _, WriteType};
    use cubemaster_shared::ble::ADV_NAME_PREFIX;
    use uuid::Uuid;

    let adapter = crate::ble::scanner::get_adapter()
        .await
        .map_err(|e| e.to_string())?;

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| format!("Peripherals: {e}"))?;

    // Find a CubeMaster peripheral — prefer one already connected
    let mut target = None;
    for p in &peripherals {
        if p.is_connected().await.unwrap_or(false) {
            target = Some(p.clone());
            break;
        }
    }

    // If none connected, try to find and reconnect to the CubeMaster device
    if target.is_none() {
        tracing::info!("No connected device found, attempting reconnect...");
        for p in &peripherals {
            if let Some(props) = p.properties().await.unwrap_or(None) {
                if let Some(ref name) = props.local_name {
                    if name.starts_with(ADV_NAME_PREFIX) {
                        tracing::info!("Reconnecting to {}...", name);
                        p.connect().await.map_err(|e| format!("Reconnect: {e}"))?;
                        // Wait for connection to stabilize + firmware GATT server to be ready
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                        target = Some(p.clone());
                        break;
                    }
                }
            }
        }
    }

    let peripheral = target
        .ok_or_else(|| "No CubeMaster device found. Try scanning again.".to_string())?;

    // Ensure services are discovered
    peripheral
        .discover_services()
        .await
        .map_err(|e| format!("Service discovery: {e}"))?;

    // Find the characteristic by UUID
    let target_uuid = Uuid::parse_str(uuid_str)
        .map_err(|e| format!("Invalid UUID: {e}"))?;

    let characteristics = peripheral.characteristics();
    tracing::debug!(
        "Found {} characteristics on device",
        characteristics.len()
    );

    let char = characteristics
        .iter()
        .find(|c| c.uuid == target_uuid)
        .ok_or_else(|| {
            let available: Vec<String> = characteristics.iter().map(|c| c.uuid.to_string()).collect();
            format!(
                "Characteristic {} not found. Available: {:?}",
                uuid_str, available
            )
        })?;

    // Write the data
    peripheral
        .write(char, data, WriteType::WithResponse)
        .await
        .map_err(|e| format!("BLE write: {e}"))?;

    tracing::info!("Wrote {} bytes to {}", data.len(), uuid_str);
    Ok(())
}

/// Scan available WiFi networks on the host machine.
async fn scan_host_wifi_networks() -> Result<Vec<String>, String> {
    // Use nmcli on Linux to list available WiFi networks.
    let output = tokio::process::Command::new("nmcli")
        .args(["-t", "-f", "SSID", "device", "wifi", "list"])
        .output()
        .await
        .map_err(|e| format!("nmcli: {e}"))?;

    if !output.status.success() {
        return Err("nmcli failed".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut networks: Vec<String> = stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // Deduplicate and sort
    networks.sort();
    networks.dedup();

    Ok(networks)
}
