use dioxus::prelude::*;
use std::time::Instant;

use crate::components::connection_header::ConnectionHeader;
use crate::components::icons::*;
use crate::components::sidebar::{NavItem, Sidebar};
use crate::state::{use_app_state, ConnectedCube};

/// A discovered cube.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredDevice {
    pub name: String,
    pub short_id: String,
    pub address: String,
    pub rssi: Option<i16>,
}

#[component]
pub fn DiscoverView() -> Element {
    let mut app_state = use_app_state();
    let mut is_scanning = use_signal(|| false);
    let mut devices = use_signal(Vec::<DiscoveredDevice>::new);
    let mut scan_error = use_signal(|| Option::<String>::None);

    let start_scan = move |_| {
        is_scanning.set(true);
        devices.set(Vec::new());
        scan_error.set(None);

        spawn(async move {
            match do_reactive_scan(&mut devices).await {
                Ok(()) => {}
                Err(e) => {
                    scan_error.set(Some(e));
                }
            }
            is_scanning.set(false);
        });
    };

    let disconnect = move |_| {
        spawn(async move {
            tracing::info!("Disconnect button clicked (header)");
            if let Err(e) = disconnect_ble().await {
                tracing::warn!("BLE disconnect error: {}", e);
            }
            app_state.write().connected_cube = None;
        });
    };

    let is_connected = app_state.read().is_connected();

    rsx! {
        div { class: "app-layout",
            Sidebar { active: NavItem::Discover }
            main { class: "main-content",
                ConnectionHeader {}

                div { class: "view-header",
                    h1 { class: "view-title", "Discover Cubes" }
                    div { class: "header-actions",
                        if is_connected {
                            button {
                                class: "btn btn-danger",
                                onclick: disconnect,
                                span { "Disconnect" }
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            disabled: *is_scanning.read(),
                            onclick: start_scan,
                            if *is_scanning.read() {
                                IconRefresh { class: "btn-icon spin".to_string() }
                                span { "Scanning..." }
                            } else {
                                IconSearch { class: "btn-icon".to_string() }
                                span { "Scan" }
                            }
                        }
                    }
                }

                if let Some(ref err) = *scan_error.read() {
                    div { class: "status-banner banner-error",
                        span { "Scan error: {err}" }
                    }
                }

                {
                    let connected_device = app_state.read().connected_cube.as_ref().and_then(|cube| {
                        let already_in_list = devices.read().iter().any(|d| d.name == cube.name);
                        if already_in_list {
                            None
                        } else {
                            Some(DiscoveredDevice {
                                name: cube.name.clone(),
                                short_id: cube.short_id.clone(),
                                address: cube.device_id.clone(),
                                rssi: cube.rssi,
                            })
                        }
                    });
                    if let Some(dev) = connected_device {
                        rsx! {
                            div { class: "card-grid",
                                DeviceCard { device: dev, app_state: app_state }
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }

                div { class: "card-grid",
                    for device in devices.read().iter() {
                        DeviceCard {
                            device: device.clone(),
                            app_state: app_state,
                        }
                    }
                }

                if devices.read().is_empty() && !*is_scanning.read() && !is_connected {
                    div { class: "empty-state",
                        IconBluetooth { class: "empty-icon".to_string() }
                        p { "No cubes found yet. Click Scan to search nearby." }
                    }
                }
            }
        }
    }
}

/// Perform a reactive BLE scan. Results are pushed into the signal as they arrive.
async fn do_reactive_scan(
    devices: &mut Signal<Vec<DiscoveredDevice>>,
) -> Result<(), String> {
    use btleplug::api::{Central, Peripheral as _};
    use cubemaster_shared::ble::ADV_NAME_PREFIX;

    let adapter = crate::ble::scanner::get_adapter()
        .await
        .map_err(|e| e.to_string())?;

    // Clear BlueZ device cache to force fresh name resolution.
    // This ensures renamed devices show their new name.
    clear_bluez_cache().await;

    adapter
        .start_scan(btleplug::api::ScanFilter::default())
        .await
        .map_err(|e| format!("Start scan: {e}"))?;

    // Poll peripherals every 500ms for 6 seconds, pushing new cubes as found.
    for _ in 0..12 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| format!("Peripherals: {e}"))?;

        let mut current = Vec::new();
        for p in peripherals {
            if let Some(props) = p.properties().await.unwrap_or(None) {
                // Identify CubeMaster devices by:
                // 1. Service UUID 0xCBFF in advertising data, OR
                // 2. Name starting with "CubeMaster-", OR
                // 3. Known address from previous pairing
                let has_cube_uuid = props.services.iter().any(|u| {
                    // btleplug represents 16-bit UUIDs as full 128-bit with BT base
                    u.to_string().starts_with("0000cbff")
                });
                let has_cube_name = props
                    .local_name
                    .as_ref()
                    .map(|n| n.starts_with(ADV_NAME_PREFIX))
                    .unwrap_or(false);
                let is_known = is_known_cube_address(&props.address.to_string());

                if has_cube_uuid || has_cube_name || is_known {
                    let name = props
                        .local_name
                        .clone()
                        .unwrap_or_else(|| format!("CubeMaster-{}", &props.address.to_string()[..5]));
                    let short_id = name
                        .strip_prefix(ADV_NAME_PREFIX)
                        .unwrap_or("")
                        .to_string();
                    let address = props.address.to_string();
                    current.push(DiscoveredDevice {
                        name,
                        short_id,
                        address,
                        rssi: props.rssi,
                    });
                }
            }
        }

        if *devices.read() != current {
            devices.set(current);
        }
    }

    adapter.stop_scan().await.map_err(|e| format!("Stop scan: {e}"))?;
    Ok(())
}

/// Known CubeMaster device addresses (from previous pairings in this session).
/// In a full implementation this would come from the persistent local store.
static KNOWN_ADDRESSES: std::sync::LazyLock<std::sync::Mutex<Vec<String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Register a BLE address as a known CubeMaster device.
pub fn register_known_cube(address: &str) {
    if let Ok(mut addrs) = KNOWN_ADDRESSES.lock() {
        if !addrs.contains(&address.to_string()) {
            addrs.push(address.to_string());
        }
    }
}

/// Check if an address belongs to a known CubeMaster device.
fn is_known_cube_address(address: &str) -> bool {
    KNOWN_ADDRESSES
        .lock()
        .map(|addrs| addrs.contains(&address.to_string()))
        .unwrap_or(false)
}

/// Clear BlueZ cache for CubeMaster devices only, to force fresh name resolution.
async fn clear_bluez_cache() {
    // Get list of all known BlueZ devices and only remove CubeMaster ones
    let output = tokio::process::Command::new("bluetoothctl")
        .args(["devices"])
        .output()
        .await;

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            // Lines: "Device AA:BB:CC:DD:EE:FF DeviceName"
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 3 && parts[0] == "Device" {
                let addr = parts[1];
                let name = parts[2];
                // Only remove devices that look like CubeMaster (by name or known address)
                let is_cube = name.starts_with("CubeMaster")
                    || is_known_cube_address(addr);
                if is_cube {
                    tracing::debug!("Clearing BlueZ cache for {} ({})", name, addr);
                    let _ = tokio::process::Command::new("bluetoothctl")
                        .args(["remove", addr])
                        .output()
                        .await;
                }
            }
        }
    }

    // Give BlueZ time to process removals before scanning
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
}

#[component]
fn DeviceCard(
    device: DiscoveredDevice,
    app_state: Signal<crate::state::AppState>,
) -> Element {
    let signal_class = match device.rssi {
        Some(r) if r > -50 => "signal-strong",
        Some(r) if r > -70 => "signal-medium",
        Some(_) => "signal-weak",
        None => "signal-none",
    };

    let mut pair_status = use_signal(|| PairCardState::Idle);

    let is_this_connected = app_state
        .read()
        .connected_cube
        .as_ref()
        .map(|c| c.name == device.name)
        .unwrap_or(false);

    let device_name = device.name.clone();
    let device_short_id = device.short_id.clone();
    let device_address = device.address.clone();
    let device_rssi = device.rssi;

    let on_pair_click = move |_| {
        let name = device_name.clone();
        let short_id = device_short_id.clone();
        let address = device_address.clone();
        let rssi = device_rssi;
        pair_status.set(PairCardState::Connecting);

        spawn(async move {
            match try_connect_to_cube(&name).await {
                Ok(()) => {
                    pair_status.set(PairCardState::Connected);
                    app_state.write().connected_cube = Some(ConnectedCube {
                        name: name.clone(),
                        device_id: address,
                        short_id,
                        friendly_name: name.clone(),
                        rssi,
                        connected_at: Instant::now(),
                    });
                }
                Err(e) => {
                    pair_status.set(PairCardState::Failed(e));
                }
            }
        });
    };

    let on_disconnect = move |_| {
        pair_status.set(PairCardState::Idle);
        spawn(async move {
            tracing::info!("Disconnect button clicked (card)");
            if let Err(e) = disconnect_ble().await {
                tracing::warn!("BLE disconnect error: {}", e);
            }
            app_state.write().connected_cube = None;
        });
    };

    let card_class = if is_this_connected {
        "device-card device-card-connected"
    } else {
        "device-card"
    };

    // Show friendly name if this is the connected/renamed cube
    let display_name = if is_this_connected {
        app_state
            .read()
            .connected_cube
            .as_ref()
            .map(|c| c.friendly_name.clone())
            .unwrap_or_else(|| device.name.clone())
    } else {
        device.name.clone()
    };

    rsx! {
        div { class: "{card_class}",
            div { class: "device-card-header",
                div { class: "device-name-row",
                    IconCube { class: "device-icon".to_string() }
                    h3 { class: "device-name", "{display_name}" }
                }
                span { class: "badge {signal_class}", "BLE" }
            }
            div { class: "device-card-body",
                div { class: "device-meta",
                    span { class: "meta-label", "Address: " }
                    span { class: "meta-value", "{device.address}" }
                }
                if let Some(rssi) = device.rssi {
                    div { class: "device-meta",
                        IconSignal { class: "meta-icon {signal_class}".to_string() }
                        span { class: "meta-value", "{rssi} dBm" }
                    }
                }
                // Show pair result status
                match &*pair_status.read() {
                    PairCardState::Failed(msg) => rsx! {
                        div { class: "device-meta",
                            span { class: "meta-value meta-error", "{msg}" }
                        }
                    },
                    _ => rsx! {},
                }
            }
            div { class: "device-card-footer",
                if is_this_connected {
                    button {
                        class: "btn btn-danger btn-sm",
                        onclick: on_disconnect,
                        "Disconnect"
                    }
                } else {
                    button {
                        class: "btn btn-primary",
                        disabled: matches!(*pair_status.read(), PairCardState::Connecting),
                        onclick: on_pair_click,
                        match *pair_status.read() {
                            PairCardState::Connecting => "Pairing...",
                            PairCardState::Connected => "Paired",
                            _ => "Pair",
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PairCardState {
    Idle,
    Connecting,
    Connected,
    Failed(String),
}

/// Attempt to connect to a cube by BLE name.
async fn try_connect_to_cube(name: &str) -> Result<(), String> {
    use btleplug::api::{Central, Peripheral as _};
    use cubemaster_shared::ble::ADV_NAME_PREFIX;

    let adapter = crate::ble::scanner::get_adapter()
        .await
        .map_err(|e| e.to_string())?;

    // Use already-discovered peripherals (no re-scan needed since we just scanned)
    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| format!("Peripherals: {e}"))?;

    let mut target = None;
    for p in peripherals {
        if let Some(props) = p.properties().await.unwrap_or(None) {
            if props.local_name.as_deref() == Some(name) {
                target = Some(p);
                break;
            }
        }
    }

    let peripheral = target
        .ok_or_else(|| format!("Device '{}' not found", name))?;

    tracing::info!("Connecting to {}...", name);
    peripheral
        .connect()
        .await
        .map_err(|e| format!("Connect: {e}"))?;

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Discover services to prove full GATT connectivity
    peripheral
        .discover_services()
        .await
        .map_err(|e| format!("Service discovery: {e}"))?;

    let char_count = peripheral.characteristics().len();
    tracing::info!("Connected to {} — {} characteristics", name, char_count);

    // Register this address so future scans recognize it even after rename
    let addr = peripheral.address().to_string();
    register_known_cube(&addr);

    // Note: We intentionally do NOT disconnect here. We stay connected.
    Ok(())
}

/// Disconnect from any connected CubeMaster BLE peripheral.
pub async fn disconnect_ble() -> Result<(), String> {
    use btleplug::api::{Central, Peripheral as _};
    use cubemaster_shared::ble::ADV_NAME_PREFIX;

    let adapter = crate::ble::scanner::get_adapter()
        .await
        .map_err(|e| e.to_string())?;

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| format!("Peripherals: {e}"))?;

    // Try to disconnect any CubeMaster device (connected or not, BlueZ
    // sometimes reports is_connected incorrectly after reconnections).
    for p in peripherals {
        let connected = p.is_connected().await.unwrap_or(false);
        if connected {
            tracing::info!("Disconnecting BLE peripheral (connected=true)...");
            if let Err(e) = p.disconnect().await {
                tracing::warn!("Disconnect error: {}", e);
            } else {
                tracing::info!("BLE disconnected successfully");
            }
            return Ok(());
        }
        // Also try disconnecting CubeMaster devices even if not reported as connected
        if let Some(props) = p.properties().await.unwrap_or(None) {
            if let Some(ref name) = props.local_name {
                if name.starts_with(ADV_NAME_PREFIX) {
                    tracing::info!("Force-disconnecting {} (connected={})...", name, connected);
                    let _ = p.disconnect().await;
                    return Ok(());
                }
            }
        }
    }

    tracing::warn!("No CubeMaster peripheral found to disconnect");
    Ok(())
}
