//! WiFi station (STA) connection management.
//!
//! The WiFi subsystem is normally off. It is activated either:
//! 1. Automatically on boot if stored WiFi credentials exist in NVS.
//! 2. On demand when the companion app sends credentials via BLE.
//!
//! Once connected, the cube:
//! 1. Obtains an IP via DHCP.
//! 2. Notifies the BLE client of the assigned IP via the status characteristic.
//! 3. (Future) Starts an mDNS responder and HTTP server.

use defmt::info;
use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Timer, with_timeout};
use esp_radio::wifi::{Config, WifiController, sta::StationConfig};
use heapless::String;

/// Events from the WiFi subsystem -> supervisor.
#[derive(Debug, Clone)]
pub enum WifiEvent {
    /// WiFi connected, DHCP obtained.
    Connected { ip: String<16> },
    /// WiFi disconnected (timeout, AP gone, etc.).
    Disconnected,
    /// WiFi connection failed.
    Failed { reason: String<32> },
}

/// Commands from the supervisor -> WiFi subsystem.
#[derive(Debug, Clone)]
pub enum WifiCommand {
    /// Connect to a network with these credentials.
    Connect { ssid: String<32>, password: String<64> },
    /// Disconnect and power down WiFi.
    Disconnect,
    /// Auto-connect using stored credentials from NVS (sent on boot).
    AutoConnect { ssid: String<32>, password: String<64> },
}

pub const WIFI_EVENT_CHANNEL_SIZE: usize = 4;
pub const WIFI_CMD_CHANNEL_SIZE: usize = 4;

pub type WifiEventChannel = Channel<CriticalSectionRawMutex, WifiEvent, WIFI_EVENT_CHANNEL_SIZE>;
pub type WifiCmdChannel = Channel<CriticalSectionRawMutex, WifiCommand, WIFI_CMD_CHANNEL_SIZE>;

/// WiFi connection state for the task.
#[derive(Debug, Clone, PartialEq)]
enum WifiState {
    /// Radio is off, waiting for a connect command.
    Off,
    /// Attempting to connect to an AP.
    Connecting,
    /// Connected with an assigned IP.
    Connected { ip: String<16> },
}

/// WiFi + embassy-net task.
///
/// Owns the WifiController and has access to the embassy-net Stack.
/// Listens for WifiCommand messages and drives connection/disconnection.
pub async fn wifi_task_impl(
    event_tx: Sender<'static, CriticalSectionRawMutex, WifiEvent, WIFI_EVENT_CHANNEL_SIZE>,
    cmd_rx: Receiver<'static, CriticalSectionRawMutex, WifiCommand, WIFI_CMD_CHANNEL_SIZE>,
    controller: &'static mut WifiController<'static>,
    stack: Stack<'static>,
) {
    info!("WiFi task started — waiting for connect command");

    let mut state = WifiState::Off;

    loop {
        match state {
            WifiState::Off => {
                // Wait for a connect command.
                let cmd = cmd_rx.receive().await;
                match cmd {
                    WifiCommand::Connect { ssid, password } => {
                        info!("WiFi: connect requested — SSID={}", ssid.as_str());
                        state = WifiState::Connecting;
                        attempt_connection(
                            ssid.as_str(), password.as_str(),
                            controller, &stack, &event_tx, &mut state,
                        ).await;
                    }
                    WifiCommand::AutoConnect { ssid, password } => {
                        info!("WiFi: auto-connect on boot — SSID={}", ssid.as_str());
                        state = WifiState::Connecting;
                        attempt_connection(
                            ssid.as_str(), password.as_str(),
                            controller, &stack, &event_tx, &mut state,
                        ).await;
                    }
                    WifiCommand::Disconnect => {
                        // Already off, ignore.
                    }
                }
            }
            WifiState::Connecting => {
                // Should not reach here; attempt_connection handles transitions.
                Timer::after(Duration::from_millis(100)).await;
            }
            WifiState::Connected { ip: _ } => {
                // Connected — listen for disconnect command or connection loss.
                match cmd_rx.try_receive() {
                    Ok(WifiCommand::Disconnect) => {
                        info!("WiFi: disconnect requested");
                        let _ = controller.disconnect_async().await;
                        event_tx.send(WifiEvent::Disconnected).await;
                        state = WifiState::Off;
                    }
                    Ok(WifiCommand::Connect { ssid, password }) => {
                        // Reconnect with new credentials.
                        info!("WiFi: reconnect with new creds — SSID={}", ssid.as_str());
                        let _ = controller.disconnect_async().await;
                        event_tx.send(WifiEvent::Disconnected).await;
                        state = WifiState::Connecting;
                        attempt_connection(
                            ssid.as_str(), password.as_str(),
                            controller, &stack, &event_tx, &mut state,
                        ).await;
                    }
                    Ok(WifiCommand::AutoConnect { .. }) => {
                        // Already connected, ignore auto-connect.
                    }
                    Err(_) => {
                        // No command, stay connected. Check link is still up.
                        if !stack.is_link_up() {
                            info!("WiFi: link lost, transitioning to Off");
                            event_tx.send(WifiEvent::Disconnected).await;
                            state = WifiState::Off;
                        }
                        Timer::after(Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }
}

/// Attempt to connect to a WiFi network using the real esp-radio WiFi stack.
///
/// 1. Configures the WifiController with SSID/password (WPA2 Personal).
/// 2. Calls `connect_async()` to associate with the AP.
/// 3. Waits for DHCP to assign an IP (polls embassy-net stack, 15s timeout).
/// 4. On success: sends WifiEvent::Connected with IP, transitions to Connected.
/// 5. On failure: sends WifiEvent::Failed, transitions back to Off.
async fn attempt_connection(
    ssid: &str,
    password: &str,
    controller: &mut WifiController<'_>,
    stack: &Stack<'static>,
    event_tx: &Sender<'static, CriticalSectionRawMutex, WifiEvent, WIFI_EVENT_CHANNEL_SIZE>,
    state: &mut WifiState,
) {
    info!("WiFi: configuring STA for '{}'...", ssid);

    // 1. Set station configuration with SSID and password.
    let sta_config = Config::Station(
        StationConfig::default()
            .with_ssid(ssid)
            .with_password(password.into()),
    );

    if let Err(e) = controller.set_config(&sta_config) {
        info!("WiFi: set_config failed: {:?}", e);
        let mut reason = String::<32>::new();
        let _ = reason.push_str("config error");
        event_tx.send(WifiEvent::Failed { reason }).await;
        *state = WifiState::Off;
        return;
    }

    // 2. Connect to the AP (waits for association or failure).
    //    In esp_radio 0.18.0, the controller is already started by wifi::new(),
    //    so we go directly to connect_async (no separate start() call needed).
    info!("WiFi: connecting...");
    match controller.connect_async().await {
        Ok(info) => {
            info!("WiFi: associated with AP (channel={})", info.channel);
        }
        Err(e) => {
            info!("WiFi: connect_async failed: {:?}", e);
            let mut reason = String::<32>::new();
            let _ = reason.push_str("association failed");
            event_tx.send(WifiEvent::Failed { reason }).await;
            *state = WifiState::Off;
            return;
        }
    }

    // 3. Wait for DHCP to assign an IP address (poll the stack, timeout 15s).
    info!("WiFi: waiting for DHCP...");
    let dhcp_result = with_timeout(Duration::from_secs(15), async {
        loop {
            if let Some(config) = stack.config_v4() {
                return config.address.address();
            }
            Timer::after(Duration::from_millis(500)).await;
        }
    }).await;

    match dhcp_result {
        Ok(ip_addr) => {
            let ip_str = format_ipv4(ip_addr.octets());
            info!("WiFi: DHCP assigned IP={}", ip_str.as_str());
            *state = WifiState::Connected { ip: ip_str.clone() };
            event_tx.send(WifiEvent::Connected { ip: ip_str }).await;
        }
        Err(_) => {
            info!("WiFi: DHCP timeout — disconnecting");
            let _ = controller.disconnect_async().await;
            let mut reason = String::<32>::new();
            let _ = reason.push_str("DHCP timeout");
            event_tx.send(WifiEvent::Failed { reason }).await;
            *state = WifiState::Off;
        }
    }
}

/// Format an IPv4 address from smoltcp's 4-byte array into a heapless String.
pub fn format_ipv4(octets: [u8; 4]) -> String<16> {
    let mut s: String<16> = String::new();
    for (i, &o) in octets.iter().enumerate() {
        if i > 0 {
            let _ = s.push('.');
        }
        write_u8_decimal(&mut s, o);
    }
    s
}

fn write_u8_decimal(s: &mut String<16>, mut val: u8) {
    if val >= 100 {
        let _ = s.push((b'0' + val / 100) as char);
        val %= 100;
        let _ = s.push((b'0' + val / 10) as char);
        let _ = s.push((b'0' + val % 10) as char);
    } else if val >= 10 {
        let _ = s.push((b'0' + val / 10) as char);
        let _ = s.push((b'0' + val % 10) as char);
    } else {
        let _ = s.push((b'0' + val) as char);
    }
}
