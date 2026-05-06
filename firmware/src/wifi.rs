//! WiFi station (STA) connection management and mDNS announcement.
//!
//! The WiFi subsystem is normally off.  It is activated when the companion
//! app sends WiFi credentials via BLE.  Once connected, the cube:
//! 1. Obtains an IP via DHCP.
//! 2. Starts an mDNS responder advertising `_cubemaster._tcp.local.`.
//! 3. Starts the HTTP server (see `http_server` module).
//! 4. Notifies the BLE client of the assigned IP.

use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Sender};
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

/// Commands from the supervisor → WiFi subsystem.
#[derive(Debug, Clone)]
pub enum WifiCommand {
    /// Connect to a network with these credentials.
    Connect { ssid: String<32>, password: String<64> },
    /// Disconnect and power down WiFi.
    Disconnect,
}

pub const WIFI_EVENT_CHANNEL_SIZE: usize = 4;
pub const WIFI_CMD_CHANNEL_SIZE: usize = 4;

pub type WifiEventChannel = Channel<CriticalSectionRawMutex, WifiEvent, WIFI_EVENT_CHANNEL_SIZE>;
pub type WifiCmdChannel = Channel<CriticalSectionRawMutex, WifiCommand, WIFI_CMD_CHANNEL_SIZE>;

/// WiFi + mDNS embassy task.
///
/// Listens for WifiCommand messages and manages the WiFi state machine:
/// Off → Connecting → Connected (with mDNS + HTTP) → Idle timeout → Off
///
/// # State machine:
/// ```
/// ┌─────┐  Connect cmd   ┌────────────┐  DHCP OK   ┌───────────┐
/// │ Off │ ──────────────>│ Connecting │ ──────────>│ Connected │
/// └─────┘                └────────────┘            └───────────┘
///    ^                         │ fail                      │ timeout / cmd
///    └─────────────────────────┴──────────────────────────-┘
/// ```
pub async fn wifi_task_impl(
    event_tx: Sender<'static, CriticalSectionRawMutex, WifiEvent, WIFI_EVENT_CHANNEL_SIZE>,
) {
    info!("WiFi task started (bootstrap — connection stub)");

    // In the real implementation, this would:
    // 1. Wait for a WifiCommand::Connect.
    // 2. Call esp_radio::wifi scan + connect with the given SSID/password.
    // 3. Run DHCP via embassy-net.
    // 4. Start edge-mdns responder with cube name + device_id TXT records.
    // 5. Notify supervisor via WifiEvent::Connected { ip }.
    // 6. On idle timeout, send WifiEvent::Disconnected and power down radio.

    let _ = event_tx;

    loop {
        embassy_time::Timer::after(embassy_time::Duration::from_secs(10)).await;
        info!("WiFi task heartbeat (idle — no credentials yet)");
    }
}

/// Format an IPv4 address from 4 bytes into a heapless String.
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
