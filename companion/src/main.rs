#![cfg_attr(not(test), windows_subsystem = "windows")]

mod ble;
mod components;
mod state;
mod store;
mod views;
mod wifi;

use dioxus::prelude::*;
use views::dashboard::DashboardView;
use views::discover::DiscoverView;
use views::scenario::ScenarioView;
use views::settings::SettingsView;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("cubemaster_companion=debug,btleplug=info")
        .init();

    LaunchBuilder::desktop()
        .with_cfg(desktop_config())
        .launch(app);
}

fn app() -> Element {
    state::provide_app_state();

    // Unified background BLE monitor: detects disconnects and auto-reconnects.
    use_ble_lifecycle_monitor();

    rsx! {
        style { {include_str!("../assets/style.css")} }
        components::titlebar::Titlebar {}
        Router::<Route> {}
    }
}

/// Unified BLE lifecycle monitor.
///
/// Runs continuously in the background with two modes:
/// - **Connected**: Polls every 3s to detect device power-off / disconnect.
/// - **Disconnected**: If there's a known cube in history, scans every 5s to
///   auto-reconnect when the device becomes available again.
fn use_ble_lifecycle_monitor() {
    let mut app_state = state::use_app_state();

    use_effect(move || {
        spawn(async move {
            // Small delay to let the BLE adapter initialize.
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            loop {
                if app_state.read().is_connected() {
                    // CONNECTED MODE: monitor for disconnect
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                    let still_connected = check_ble_connected().await;
                    if !still_connected {
                        tracing::info!("BLE monitor: device disconnected");
                        let mut state = app_state.write();
                        state.connected_cube = None;
                        state.cube_wifi_ip = None;
                        state.cube_wifi_ssid = None;
                    }
                } else {
                    // DISCONNECTED MODE: try to auto-reconnect
                    let store = crate::store::local::LocalStore::load();
                    if store.cubes.is_empty() {
                        // No history. Just wait and check again later.
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }

                    let most_recent = store.cubes.iter()
                        .max_by_key(|c| c.last_connected)
                        .cloned();

                    let Some(cube_info) = most_recent else {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    };

                    let cube_name = cube_info.cube_name.clone();
                    let device_id = cube_info.ble_address.clone();
                    let short_id = cube_info.short_id.clone();

                    tracing::debug!("BLE monitor: scanning for '{}' (addr={})...", cube_name, device_id);

                    match try_auto_reconnect(&cube_name, &device_id).await {
                        Ok(rssi) => {
                            tracing::info!("BLE monitor: auto-connected to '{}'", cube_name);
                            app_state.write().connected_cube = Some(state::ConnectedCube {
                                name: cube_name.clone(),
                                device_id,
                                short_id,
                                friendly_name: cube_name,
                                rssi,
                                connected_at: std::time::Instant::now(),
                            });

                            // Read WiFi status from the cube.
                            if let Some((ip, ssid)) = views::discover::read_cube_wifi_status_pub().await {
                                app_state.write().cube_wifi_ip = Some(ip);
                                app_state.write().cube_wifi_ssid = Some(ssid);
                            }
                        }
                        Err(_) => {
                            // Device not found. Wait before retrying.
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            }
        });
    });
}

/// Returns true if a CubeMaster BLE peripheral is still connected.
async fn check_ble_connected() -> bool {
    use btleplug::api::{Central, Peripheral as _};

    let adapter = match crate::ble::scanner::get_adapter().await {
        Ok(a) => a,
        Err(_) => return false,
    };

    let peripherals = match adapter.peripherals().await {
        Ok(p) => p,
        Err(_) => return false,
    };

    for p in &peripherals {
        if p.is_connected().await.unwrap_or(false) {
            return true;
        }
    }

    false
}

/// Try to reconnect to a previously known cube by scanning for it.
/// Matches by BLE address (most reliable) or by name/prefix.
/// Returns the RSSI on success.
async fn try_auto_reconnect(name: &str, ble_address: &str) -> Result<Option<i16>, String> {
    use btleplug::api::{Central, Peripheral as _, ScanFilter};
    use cubemaster_shared::ble::ADV_NAME_PREFIX;

    let adapter = crate::ble::scanner::get_adapter()
        .await
        .map_err(|e| e.to_string())?;

    // Start a BLE scan to find the device.
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| format!("scan start: {e}"))?;

    // Scan for up to 6 seconds.
    let scan_duration = std::time::Duration::from_secs(6);
    let scan_start = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_millis(500);

    let mut target = None;
    let mut found_rssi: Option<i16> = None;

    while scan_start.elapsed() < scan_duration {
        tokio::time::sleep(poll_interval).await;

        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| format!("peripherals: {e}"))?;

        for p in &peripherals {
            let addr = p.address().to_string();
            if let Some(props) = p.properties().await.unwrap_or(None) {
                // Match by BLE address (most reliable, survives name changes)
                let addr_match = !ble_address.is_empty() && addr == ble_address;

                // Match by name (exact or prefix)
                let name_match = props.local_name.as_deref().map_or(false, |n| {
                    n == name || n.starts_with(ADV_NAME_PREFIX)
                });

                if addr_match || name_match {
                    // Only accept if we have a fresh RSSI (proves we got an actual advertisement)
                    if props.rssi.is_some() {
                        found_rssi = props.rssi;
                        target = Some(p.clone());
                        break;
                    } else if target.is_none() {
                        // Accept without RSSI as fallback (cached device)
                        target = Some(p.clone());
                    }
                }
            }
        }

        // If we found a target with RSSI, we can stop scanning
        if target.is_some() && found_rssi.is_some() {
            break;
        }
    }

    let _ = adapter.stop_scan().await;

    let peripheral = target.ok_or_else(|| "Device not found during scan".to_string())?;

    peripheral
        .connect()
        .await
        .map_err(|e| format!("connect: {e}"))?;

    // Wait for GATT to stabilize
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    peripheral
        .discover_services()
        .await
        .map_err(|e| format!("service discovery: {e}"))?;

    tracing::info!("Auto-reconnect: BLE connected and services discovered (RSSI={:?})", found_rssi);
    Ok(found_rssi)
}

#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[route("/")]
    Discover,
    #[route("/dashboard")]
    Dashboard,
    #[route("/scenario")]
    Scenario,
    #[route("/settings")]
    Settings,
}

#[component]
fn Discover() -> Element {
    rsx! { DiscoverView {} }
}

#[component]
fn Dashboard() -> Element {
    rsx! { DashboardView {} }
}

#[component]
fn Scenario() -> Element {
    rsx! { ScenarioView {} }
}

#[component]
fn Settings() -> Element {
    rsx! { SettingsView {} }
}

fn desktop_config() -> dioxus::desktop::Config {
    dioxus::desktop::Config::default()
        .with_window(
            dioxus::desktop::WindowBuilder::new()
                .with_title("CubeMaster")
                .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
                .with_min_inner_size(dioxus::desktop::LogicalSize::new(800.0, 500.0))
                .with_decorations(false),
        )
}
