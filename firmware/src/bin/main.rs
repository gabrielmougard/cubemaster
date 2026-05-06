#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;

use bt_hci::controller::ExternalController;
use esp_radio::ble::controller::BleConnector;
use trouble_host::prelude::*;

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};

use defmt::info;

use esp_backtrace as _;
use esp_println as _;

use cubemaster_firmware::ble_gatt::{
    BleCommand, BleEvent, BLE_CMD_CHANNEL_SIZE, BLE_EVENT_CHANNEL_SIZE,
};
use cubemaster_firmware::config::ConfigStore;
use cubemaster_firmware::wifi::{WifiCommand, WifiEvent, WIFI_CMD_CHANNEL_SIZE, WIFI_EVENT_CHANNEL_SIZE};

extern crate alloc;

static BLE_EVENT_CH: Channel<CriticalSectionRawMutex, BleEvent, BLE_EVENT_CHANNEL_SIZE> = Channel::new();
static BLE_CMD_CH: Channel<CriticalSectionRawMutex, BleCommand, BLE_CMD_CHANNEL_SIZE> = Channel::new();
static WIFI_EVENT_CH: Channel<CriticalSectionRawMutex, WifiEvent, WIFI_EVENT_CHANNEL_SIZE> = Channel::new();
static WIFI_CMD_CH: Channel<CriticalSectionRawMutex, WifiCommand, WIFI_CMD_CHANNEL_SIZE> = Channel::new();

const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 1;

// GATT Server definition
// Minimal GATT server so the cube responds to ATT requests (service discovery).
// This will be expanded with real services (Pairing, Control, WiFi) later.
#[gatt_server]
struct CubeGattServer {
    cube_info: CubeInfoService,
}

/// A minimal "Cube Info" service that exposes the cube's name and status.
#[gatt_service(uuid = "c0bea577-0000-4000-8000-00000000f001")]
struct CubeInfoService {
    /// Cube friendly name (read + write to rename).
    #[characteristic(uuid = "c0bea577-0000-4000-8000-00000000f004", read, write)]
    cube_name: [u8; 32],

    /// Connection status byte (0=idle, 1=paired). Read + Notify.
    #[characteristic(uuid = "c0bea577-0000-4000-8000-00000000ffe2", read, notify)]
    status: u8,

    /// WiFi SSID (write, max 32 bytes).
    #[characteristic(uuid = "c0bea577-0000-4000-8000-00000000f011", write)]
    wifi_ssid: [u8; 32],

    /// WiFi password (write, max 32 bytes, we truncate long passwords for MVP).
    #[characteristic(uuid = "c0bea577-0000-4000-8000-00000000f012", write)]
    wifi_pass: [u8; 32],
}

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    // Initialize WiFi controller (powered down, ready for on-demand)
    let (_wifi_controller, interfaces) =
        esp_radio::wifi::new(peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    // Read the real MAC address from the WiFi radio
    let mac: [u8; 6] = interfaces.station.mac_address();
    info!("Hardware MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);

    // Load / generate persistent configuration
    let config_store = ConfigStore::load_or_generate(&mac);
    let adv_name = config_store.config.adv_name();
    info!("Cube name: {}", config_store.config.name.as_str());
    info!("BLE adv name: {}", adv_name.as_str());
    info!("Device ID: {}", config_store.config.device_id.as_str());

    // Initialize BLE controller
    let transport = BleConnector::new(peripherals.BT, Default::default()).unwrap();
    let controller = ExternalController::<_, 20>::new(transport);

    // Spawn auxiliary tasks
    spawner.spawn(wifi_task().unwrap());
    spawner.spawn(supervisor_task().unwrap());

    info!("Starting BLE stack...");

    // Run BLE (uses join pattern: runner + peripheral in same context)
    run_ble(controller, adv_name.as_str(), config_store.config.name.as_str()).await;

    loop {
        Timer::after(Duration::from_secs(60)).await;
    }
}

/// Run the BLE stack with a GATT server.
async fn run_ble<C: Controller>(controller: C, adv_name: &str, cube_name: &str) {
    // Use the device's BLE address (random static).
    let address = Address::random([0x1c, 0xdb, 0xd4, 0x41, 0x1e, 0x29]);
    info!("BLE address = {:?}", address);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let Host {
        mut peripheral,
        runner,
        ..
    } = stack.build();

    // Initialize GATT server
    let server = CubeGattServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: adv_name,
        appearance: &appearance::UNKNOWN,
    }))
    .unwrap();

    // Set the cube name in the readable characteristic.
    let mut name_buf = [0u8; 32];
    let name_bytes = cube_name.as_bytes();
    let len = name_bytes.len().min(32);
    name_buf[..len].copy_from_slice(&name_bytes[..len]);
    let _ = server.set(&server.cube_info.cube_name, &name_buf);
    let _ = server.set(&server.cube_info.status, &0u8);

    // Mutable advertising name buffer. Gets updated on rename and used for
    // the next advertising cycle after disconnect.
    let mut current_adv_name = [0u8; 32];
    let adv_bytes = adv_name.as_bytes();
    let adv_len = adv_bytes.len().min(32);
    current_adv_name[..adv_len].copy_from_slice(&adv_bytes[..adv_len]);
    let mut current_adv_len = adv_len;

    info!("GATT server initialized, starting advertising + runner");

    // The runner must execute concurrently with our application logic.
    let _ = join(ble_runner(runner), async {
        loop {
            // Build the adv name from current state
            let adv_name_for_cycle: heapless::String<32> = {
                let mut s = heapless::String::new();
                let _ = s.push_str(
                    core::str::from_utf8(&current_adv_name[..current_adv_len]).unwrap_or(adv_name)
                );
                s
            };

            match advertise_raw(adv_name_for_cycle.as_bytes(), &mut peripheral, &server).await {
                Ok(conn) => {
                    info!("[ble] connection established");
                    BLE_EVENT_CH.sender().send(BleEvent::Connected).await;

                    // Handle GATT events until disconnect.
                    gatt_events_task(&server, &conn).await;

                    // After disconnect, check if the cube was renamed during
                    // this connection by reading back the cube_name characteristic.
                    if let Ok(updated_name) = server.get(&server.cube_info.cube_name) {
                        let end = updated_name.iter().position(|&b| b == 0).unwrap_or(updated_name.len());
                        if end > 0 {
                            // Update the advertising name for the next cycle
                            current_adv_name = [0u8; 32];
                            current_adv_name[..end].copy_from_slice(&updated_name[..end]);
                            current_adv_len = end;
                            if let Ok(new_name) = core::str::from_utf8(&updated_name[..end]) {
                                info!("[ble] next advertising name: {}", new_name);
                            }
                        }
                    }

                    info!("[ble] connection closed, re-advertising...");
                    BLE_EVENT_CH.sender().send(BleEvent::Disconnected).await;
                }
                Err(_e) => {
                    info!("[ble] advertise error, retrying...");
                    Timer::after(Duration::from_secs(1)).await;
                }
            }
        }
    })
    .await;
}

/// Background BLE host runner.
async fn ble_runner<C: Controller, P: PacketPool>(mut runner: Runner<'_, C, P>) {
    loop {
        if let Err(_e) = runner.run().await {
            info!("[ble_runner] error, restarting...");
        }
    }
}

/// 16-bit service UUID used in advertising to identify CubeMaster devices.
/// The companion uses this to filter scan results regardless of device name.
const CUBEMASTER_ADV_UUID: [u8; 2] = [0xFF, 0xCB]; // 0xCBFF little-endian

/// Start advertising with given name bytes and wait for a GATT connection.
async fn advertise_raw<'values, 'server, C: Controller>(
    name_bytes: &[u8],
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server CubeGattServer<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut adv_data_buf = [0u8; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[CUBEMASTER_ADV_UUID]),
            AdStructure::CompleteLocalName(name_bytes),
        ],
        &mut adv_data_buf,
    )?;

    if let Ok(name_str) = core::str::from_utf8(name_bytes) {
        info!("[adv] starting (name={})", name_str);
    }

    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &adv_data_buf[..len],
                scan_data: &[],
            },
        )
        .await?;

    info!("[adv] advertising active, waiting for central...");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] central connected with GATT");
    Ok(conn)
}

/// Process incoming GATT events (reads, writes) until disconnect.
async fn gatt_events_task(
    server: &CubeGattServer<'_>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
) {
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Read(read_event) => {
                        if read_event.handle() == server.cube_info.cube_name.handle {
                            info!("[gatt] read: cube_name");
                        } else if read_event.handle() == server.cube_info.status.handle {
                            info!("[gatt] read: status");
                        }
                    }
                    GattEvent::Write(write_event) => {
                        if write_event.handle() == server.cube_info.cube_name.handle {
                            let data = write_event.data();
                            let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
                            if let Ok(new_name) = core::str::from_utf8(&data[..end]) {
                                info!("[gatt] RENAME cube to: {}", new_name);
                                // Store the new name in the server's characteristic value
                                // so server.get() returns it after disconnect.
                                let mut padded = [0u8; 32];
                                padded[..end].copy_from_slice(&data[..end]);
                                let _ = server.set(&server.cube_info.cube_name, &padded);
                                BLE_EVENT_CH.sender().try_send(BleEvent::CmdRename {
                                    name: {
                                        let mut s = heapless::String::<64>::new();
                                        let _ = s.push_str(new_name);
                                        s
                                    },
                                }).ok();
                            }
                        } else if write_event.handle() == server.cube_info.wifi_ssid.handle {
                            let data = write_event.data();
                            let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
                            if let Ok(ssid) = core::str::from_utf8(&data[..end]) {
                                info!("[gatt] WiFi SSID set: {}", ssid);
                            }
                        } else if write_event.handle() == server.cube_info.wifi_pass.handle {
                            let data = write_event.data();
                            let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
                            if let Ok(pass) = core::str::from_utf8(&data[..end]) {
                                info!("[gatt] WiFi password received ({} chars)", pass.len());
                                // TODO: Combine stored SSID + this password and trigger WiFi connect
                                info!("[gatt] WiFi credentials complete — would connect now");
                            }
                        } else {
                            info!("[gatt] write to unknown handle");
                        }
                    }
                    _ => {}
                }
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(_e) => {
                        info!("[gatt] error sending reply");
                    }
                }
            }
            _ => {}
        }
    }
}

/// Periodically notify the status characteristic (demonstrates notify works).
/// Currently unused. TODO: Will be activated once companion subscribes to notifications.
#[allow(dead_code)]
async fn status_notify_task(
    server: &CubeGattServer<'_>,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
) {
    let mut tick: u8 = 0;
    loop {
        Timer::after(Duration::from_secs(5)).await;
        tick = tick.wrapping_add(1);
        if server.cube_info.status.notify(conn, &tick).await.is_err() {
            info!("[notify] connection lost");
            break;
        }

        info!("[notify] sent status tick={}", tick);
    }
}

// Auxiliary embassy tasks

#[embassy_executor::task]
async fn wifi_task() {
    cubemaster_firmware::wifi::wifi_task_impl(WIFI_EVENT_CH.sender()).await;
}

#[embassy_executor::task]
async fn supervisor_task() {
    info!("Supervisor task started");

    let ble_event_rx = BLE_EVENT_CH.receiver();
    let _ble_cmd_tx = BLE_CMD_CH.sender();
    let wifi_event_rx = WIFI_EVENT_CH.receiver();
    let wifi_cmd_tx = WIFI_CMD_CH.sender();

    loop {
        if let Ok(event) = ble_event_rx.try_receive() {
            match event {
                BleEvent::Connected => {
                    info!("Supervisor: BLE client connected");
                }
                BleEvent::Disconnected => {
                    info!("Supervisor: BLE client disconnected");
                }
                BleEvent::WifiCredentials { ssid, password } => {
                    info!("Supervisor: WiFi creds received, connecting...");
                    wifi_cmd_tx
                        .send(WifiCommand::Connect { ssid, password })
                        .await;
                }
                BleEvent::CmdPlay { id, face, volume } => {
                    info!("Supervisor: Play cmd — id={}, face={}, vol={}", id, face, volume);
                }
                BleEvent::CmdStop => {
                    info!("Supervisor: Stop cmd");
                }
                BleEvent::CmdVolume { volume } => {
                    info!("Supervisor: Volume set to {}", volume);
                }
                BleEvent::CmdRename { name } => {
                    info!("Supervisor: Rename to {}", name.as_str());
                }
                BleEvent::PairingRequested { .. } => {
                    info!("Supervisor: Pairing requested");
                }
                BleEvent::PairingConfirmed { .. } => {
                    info!("Supervisor: Pairing confirmed");
                }
                BleEvent::SessionAuthenticated => {
                    info!("Supervisor: Session authenticated");
                }
            }
        }

        if let Ok(event) = wifi_event_rx.try_receive() {
            match event {
                WifiEvent::Connected { ip } => {
                    info!("Supervisor: WiFi connected, IP={}", ip.as_str());
                }
                WifiEvent::Disconnected => {
                    info!("Supervisor: WiFi disconnected");
                }
                WifiEvent::Failed { reason } => {
                    info!("Supervisor: WiFi failed: {}", reason.as_str());
                }
            }
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}
