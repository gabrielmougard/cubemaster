use dioxus::prelude::*;
use crate::components::connection_header::ConnectionHeader;
use crate::components::icons::*;
use crate::components::sidebar::{NavItem, Sidebar};
use crate::state::use_app_state;

/// UUIDs matching the firmware's GATT characteristics.
const CUBE_NAME_CHAR_UUID: &str = "c0bea577-0000-4000-8000-00000000f004";
const WIFI_SSID_CHAR_UUID: &str = "c0bea577-0000-4000-8000-00000000f011";
const WIFI_PASS_CHAR_UUID: &str = "c0bea577-0000-4000-8000-00000000f012";
/// UUID for the status characteristic (read/notify) to monitor WiFi IP.
const STATUS_CHAR_UUID: &str = "c0bea577-0000-4000-8000-00000000ffe2";

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
    let mut is_scanning_wifi = use_signal(|| false);

    let is_connected = app_state.read().is_connected();
    let connected_name = app_state
        .read()
        .connected_cube
        .as_ref()
        .map(|c| c.friendly_name.clone())
        .unwrap_or_default();
    let wifi_ip = app_state.read().cube_wifi_ip.clone();
    let wifi_connected_ssid = app_state.read().cube_wifi_ssid.clone();

    // Scan host WiFi networks on mount
    let _scan_wifi = use_effect(move || {
        spawn(async move {
            is_scanning_wifi.set(true);
            match scan_host_wifi_networks().await {
                Ok(networks) => wifi_networks.set(networks),
                Err(e) => tracing::warn!("Failed to scan WiFi networks: {}", e),
            }
            is_scanning_wifi.set(false);
        });
    });

    let on_rescan_wifi = move |_| {
        spawn(async move {
            is_scanning_wifi.set(true);
            match scan_host_wifi_networks().await {
                Ok(networks) => wifi_networks.set(networks),
                Err(e) => tracing::warn!("Failed to rescan WiFi networks: {}", e),
            }

            is_scanning_wifi.set(false);
        });
    };

    let on_disconnect = move |_| {
        spawn(async move {
            tracing::info!("Disconnect button clicked");
            if let Err(e) = crate::views::discover::disconnect_ble().await {
                tracing::warn!("BLE disconnect error: {}", e);
            }

            app_state.write().connected_cube = None;
            app_state.write().cube_wifi_ip = None;
            app_state.write().cube_wifi_ssid = None;
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
            // Clear any stale WiFi state before starting a new provisioning attempt.
            app_state.write().cube_wifi_ip = None;
            app_state.write().cube_wifi_ssid = None;

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

            action_status.set(ActionStatus::InProgress("Credentials sent, waiting for cube to connect...".into()));

            // Poll the status characteristic until we get a WiFi IP or timeout.
            // The cube takes ~2-5 seconds to associate + get DHCP.
            // We poll every 1 second for up to 20 seconds.
            let poll_interval = std::time::Duration::from_secs(1);
            let max_attempts = 20;
            let mut ip_found: Option<String> = None;

            for attempt in 1..=max_attempts {
                tokio::time::sleep(poll_interval).await;

                match read_ble_characteristic(STATUS_CHAR_UUID).await {
                    Ok(data) => {
                        // Trim trailing null bytes from the fixed-size characteristic.
                        let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
                        if let Ok(status_str) = std::str::from_utf8(&data[..end]) {
                            if let Some(ip) = parse_wifi_ip_from_status(status_str) {
                                tracing::info!(
                                    "WiFi IP obtained after {} poll(s): {}",
                                    attempt, ip
                                );
                                ip_found = Some(ip);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Poll attempt {}: read failed: {}", attempt, e);
                        // Don't break — transient BLE errors can happen during WiFi connect.
                    }
                }

                // Update the progress message with the attempt count.
                if attempt % 5 == 0 {
                    action_status.set(ActionStatus::InProgress(
                        format!("Waiting for cube WiFi connection... ({}s)", attempt)
                    ));
                }
            }

            // Evaluate the result.
            if let Some(ip) = ip_found {
                app_state.write().cube_wifi_ip = Some(ip.clone());
                app_state.write().cube_wifi_ssid = Some(ssid.clone());
                action_status.set(ActionStatus::Success(
                    format!("WiFi connected! IP: {}", ip)
                ));
            } else {
                // Timed out. No IP obtained means connection failed.
                // Do NOT set wifi state since the cube is not connected.
                action_status.set(ActionStatus::Error(
                    format!("WiFi connection failed for '{}'. Check password and try again.", ssid)
                ));
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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

                        // WiFi connection status indicator
                        if let Some(ref ip) = wifi_ip {
                            div { class: "wifi-status-card wifi-status-connected",
                                div { class: "wifi-status-icon",
                                    IconWifi { class: "wifi-status-svg".to_string() }
                                }
                                div { class: "wifi-status-info",
                                    span { class: "wifi-status-label", "Connected" }
                                    span { class: "wifi-status-detail",
                                        if let Some(ref ssid) = wifi_connected_ssid {
                                            "{ssid} — "
                                        }
                                        "{ip}"
                                    }
                                }
                                div { class: "wifi-status-dot" }
                            }
                        }

                        div { class: "form-fields",
                            div { class: "input-group",
                                div { class: "input-label-row",
                                    label { class: "input-label", "SSID" }
                                    button {
                                        class: "btn btn-ghost btn-xs",
                                        disabled: *is_scanning_wifi.read(),
                                        onclick: on_rescan_wifi,
                                        if *is_scanning_wifi.read() {
                                            span { class: "spin", "↻" }
                                        } else {
                                            "↻ Rescan"
                                        }
                                    }
                                }
                                div { class: "wifi-select-wrapper",
                                    select {
                                        class: "input wifi-select",
                                        disabled: !is_connected,
                                        value: "{wifi_ssid}",
                                        onchange: move |e| wifi_ssid.set(e.value()),
                                        option { value: "", disabled: true, selected: wifi_ssid.read().is_empty(),
                                            if *is_scanning_wifi.read() {
                                                "Scanning networks..."
                                            } else if wifi_networks.read().is_empty() {
                                                "No networks found"
                                            } else {
                                                "Select a network..."
                                            }
                                        }
                                        for network in wifi_networks.read().iter() {
                                            option {
                                                value: "{network}",
                                                selected: *wifi_ssid.read() == *network,
                                                "{network}"
                                            }
                                        }
                                    }

                                    if *is_scanning_wifi.read() {
                                        div { class: "wifi-scanning-indicator",
                                            span { class: "spin", "↻" }
                                        }
                                    }
                                }

                                if wifi_networks.read().is_empty() && !*is_scanning_wifi.read() {
                                    span { class: "input-hint",
                                        "No networks found. Click Rescan or ensure WiFi is enabled on this machine."
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
                                disabled: !is_connected || wifi_ssid.read().is_empty(),
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

/// Read a value from a BLE characteristic on the currently connected cube.
async fn read_ble_characteristic(uuid_str: &str) -> Result<Vec<u8>, String> {
    use btleplug::api::{Central, Peripheral as _};
    use cubemaster_shared::ble::ADV_NAME_PREFIX;
    use uuid::Uuid;

    let adapter = crate::ble::scanner::get_adapter()
        .await
        .map_err(|e| e.to_string())?;

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| format!("Peripherals: {e}"))?;

    // Find a connected CubeMaster peripheral
    let mut target = None;
    for p in &peripherals {
        if p.is_connected().await.unwrap_or(false) {
            target = Some(p.clone());
            break;
        }
    }

    // If none connected, try to find one by name
    if target.is_none() {
        for p in &peripherals {
            if let Some(props) = p.properties().await.unwrap_or(None) {
                if let Some(ref name) = props.local_name {
                    if name.starts_with(ADV_NAME_PREFIX) {
                        p.connect().await.map_err(|e| format!("Reconnect: {e}"))?;
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                        target = Some(p.clone());
                        break;
                    }
                }
            }
        }
    }

    let peripheral = target
        .ok_or_else(|| "No CubeMaster device found".to_string())?;

    peripheral
        .discover_services()
        .await
        .map_err(|e| format!("Service discovery: {e}"))?;

    let target_uuid = Uuid::parse_str(uuid_str)
        .map_err(|e| format!("Invalid UUID: {e}"))?;

    let characteristics = peripheral.characteristics();
    let char = characteristics
        .iter()
        .find(|c| c.uuid == target_uuid)
        .ok_or_else(|| format!("Characteristic {} not found", uuid_str))?;

    let data = peripheral
        .read(char)
        .await
        .map_err(|e| format!("BLE read: {e}"))?;

    tracing::info!("Read {} bytes from {}", data.len(), uuid_str);
    Ok(data)
}

/// Parse the WiFi IP address from a JSON status payload.
///
/// Expected format: `{"wifi":"connected","ip":"192.168.1.100"}`
fn parse_wifi_ip_from_status(status: &str) -> Option<String> {
    // Simple JSON parsing without pulling in serde for this single field.
    // Look for `"ip":"<value>"` pattern.
    let ip_key = "\"ip\":\"";
    if let Some(start) = status.find(ip_key) {
        let value_start = start + ip_key.len();
        if let Some(end) = status[value_start..].find('"') {
            let ip = &status[value_start..value_start + end];
            if !ip.is_empty() {
                return Some(ip.to_string());
            }
        }
    }

    None
}
