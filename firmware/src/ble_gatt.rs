//! BLE GATT server implementation with `trouble-host`.
//!
//! Implements three GATT services exposed by the cube:
//! - **Pairing Service**: initial setup + session auth
//! - **Control Service**: command, status, config
//! - **WiFi Provisioning Service**: SSID/password delivery + WiFi status
//!
//! This module defines the GATT table, handles characteristic reads/writes,
//! and publishes notifications.  It communicates with the supervisor task via
//! an `embassy_sync::channel::Channel`.

use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Sender};
use heapless::String;

/// Events that the BLE GATT server sends to the supervisor.
#[derive(Debug, Clone)]
pub enum BleEvent {
    /// A new BLE client connected.
    Connected,
    /// BLE client disconnected.
    Disconnected,
    /// Pairing initiated — the cube should show the 6-digit code.
    PairingRequested { app_nonce: [u8; 32] },
    /// Pairing confirmed by the app with the 6-digit code.
    PairingConfirmed { code: [u8; 6], app_nonce: [u8; 32] },
    /// Session authenticated (app proved PSK knowledge).
    SessionAuthenticated,
    /// Play command received.
    CmdPlay { id: u8, face: u8, volume: u8 },
    /// Stop command received.
    CmdStop,
    /// Volume set command.
    CmdVolume { volume: u8 },
    /// WiFi credentials received — start connecting.
    WifiCredentials { ssid: String<32>, password: String<64> },
    /// Cube rename request.
    CmdRename { name: String<64> },
}

/// Commands that the supervisor sends back to the BLE task to trigger
/// notifications or update readable characteristics.
#[derive(Debug, Clone)]
pub enum BleCommand {
    /// Update the STATUS characteristic and notify subscribers.
    NotifyStatus { payload: [u8; 128], len: usize },
    /// Update the WiFi status characteristic and notify subscribers.
    NotifyWifiStatus { payload: [u8; 64], len: usize },
    /// Set the pairing info (6-digit code) for readback.
    SetPairingCode { code: [u8; 6] },
    /// Set the pairing result after successful confirm.
    SetPairingResult { payload: [u8; 128], len: usize },
}

/// Channel capacity for BLE events → supervisor.
pub const BLE_EVENT_CHANNEL_SIZE: usize = 8;
/// Channel capacity for supervisor → BLE commands.
pub const BLE_CMD_CHANNEL_SIZE: usize = 4;

pub type BleEventChannel = Channel<CriticalSectionRawMutex, BleEvent, BLE_EVENT_CHANNEL_SIZE>;
pub type BleCmdChannel = Channel<CriticalSectionRawMutex, BleCommand, BLE_CMD_CHANNEL_SIZE>;

/// The BLE GATT task.
///
/// This is the embassy task that owns the BLE stack and runs the GATT server
/// loop.  It advertises the cube, accepts connections, processes characteristic
/// writes, and pushes events to the supervisor.
///
/// # Parameters
/// - `stack`: The trouble-host BLE stack (host + controller split).
/// - `event_tx`: Channel sender to push events to the supervisor.
/// - `cmd_rx`: Channel receiver for commands from the supervisor.
/// - `adv_name`: The BLE advertising local name (e.g. "CubeMaster-AB12").
///
/// # Implementation Notes
///
/// The real GATT table registration using trouble-host macros requires
/// specific lifetime and type gymnastics that depend on the exact stack
/// configuration.  This file provides the architecture and message plumbing;
/// the actual `trouble_host::gatt!{}` macro invocation will be finalized
/// when we can do a full `cargo build` targeting the device.
pub struct BleGattServer {
    // Placeholder fields — will hold the actual trouble-host GATT handles.
    _private: (),
}

impl BleGattServer {
    /// Create the GATT server and register all services + characteristics.
    ///
    /// This sets up the attribute table but does not start advertising.
    pub fn new() -> Self {
        info!("BLE GATT server initialized (bootstrap stub)");
        Self { _private: () }
    }
}

/// Run the BLE advertising + connection event loop.
///
/// This function never returns. It is intended to be spawned as an embassy task.
///
/// Pseudocode for the actual implementation:
/// ```ignore
/// #[embassy_executor::task]
/// async fn ble_task(
///     stack: Stack,
///     event_tx: Sender<'static, CriticalSectionRawMutex, BleEvent, BLE_EVENT_CHANNEL_SIZE>,
///     cmd_rx: Receiver<'static, CriticalSectionRawMutex, BleCommand, BLE_CMD_CHANNEL_SIZE>,
/// ) {
///     let server = BleGattServer::new();
///     loop {
///         // Start advertising
///         let conn = advertise(&stack, &adv_name).await;
///         event_tx.send(BleEvent::Connected).await;
///         // Process GATT operations until disconnect
///         run_connection(&conn, &server, &event_tx, &cmd_rx).await;
///         event_tx.send(BleEvent::Disconnected).await;
///     }
/// }
/// ```
pub async fn ble_task_impl(
    event_tx: Sender<'static, CriticalSectionRawMutex, BleEvent, BLE_EVENT_CHANNEL_SIZE>,
) {
    info!("BLE task started (bootstrap — advertising stub)");

    // In the real implementation, this would:
    // 1. Build the GATT attribute table with trouble_host::gatt!{} macro.
    // 2. Start advertising with the cube's name.
    // 3. Accept connections and process reads/writes.
    // 4. Parse incoming writes into BleEvent variants.
    // 5. Send notifications when BleCommand messages arrive.

    // For now, just signal that we're "connected" so the supervisor can
    // exercise the channel plumbing.
    let _ = event_tx;

    loop {
        embassy_time::Timer::after(embassy_time::Duration::from_secs(5)).await;
        info!("BLE task heartbeat");
    }
}
