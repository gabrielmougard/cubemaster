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

use static_cell::StaticCell;
use embassy_sync::mutex::Mutex;

extern crate alloc;

static BLE_EVENT_CH: Channel<CriticalSectionRawMutex, BleEvent, BLE_EVENT_CHANNEL_SIZE> = Channel::new();
static BLE_CMD_CH: Channel<CriticalSectionRawMutex, BleCommand, BLE_CMD_CHANNEL_SIZE> = Channel::new();
static WIFI_EVENT_CH: Channel<CriticalSectionRawMutex, WifiEvent, WIFI_EVENT_CHANNEL_SIZE> = Channel::new();
static WIFI_CMD_CH: Channel<CriticalSectionRawMutex, WifiCommand, WIFI_CMD_CHANNEL_SIZE> = Channel::new();

/// Shared config store protected by a mutex for access from multiple tasks.
static CONFIG_STORE: StaticCell<Mutex<CriticalSectionRawMutex, ConfigStore>> = StaticCell::new();

/// Global reference to the config store mutex (set during init).
static mut CONFIG_REF: Option<&'static Mutex<CriticalSectionRawMutex, ConfigStore>> = None;

/// Shared WiFi status buffer. Updated by the supervisor when WiFi connects.
/// The GATT read handler returns this when the status characteristic is read.
/// Format: UTF-8 JSON, e.g. `{"w":1,"ip":"192.168.1.26"}`
///
/// Uses a critical-section mutex (blocking) so both the async supervisor and
/// the sync GATT read handler can access it safely.
static WIFI_STATUS: embassy_sync::blocking_mutex::Mutex<
    CriticalSectionRawMutex,
    core::cell::RefCell<[u8; 32]>,
> = embassy_sync::blocking_mutex::Mutex::new(core::cell::RefCell::new([0u8; 32]));
static WIFI_STATUS_LEN: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Shared WiFi SSID buffer. Set when credentials are received or loaded from NVS.
/// The GATT read handler returns this for the wifi_ssid characteristic.
static WIFI_SSID_BUF: embassy_sync::blocking_mutex::Mutex<
    CriticalSectionRawMutex,
    core::cell::RefCell<[u8; 32]>,
> = embassy_sync::blocking_mutex::Mutex::new(core::cell::RefCell::new([0u8; 32]));

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

    /// Connection/WiFi status (JSON). Read + Notify. Up to 32 bytes.
    /// Format: {"w":1,"ip":"x.x.x.x"} when WiFi connected, or all zeros.
    #[characteristic(uuid = "c0bea577-0000-4000-8000-00000000ffe2", read, notify)]
    status: [u8; 32],

    /// WiFi SSID (read + write, max 32 bytes).
    #[characteristic(uuid = "c0bea577-0000-4000-8000-00000000f011", read, write)]
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

    // Initialize WiFi controller and embassy-net stack
    let (wifi_controller, interfaces) =
        esp_radio::wifi::new(peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    // Read the real MAC address from the WiFi radio
    let mac: [u8; 6] = interfaces.station.mac_address();
    info!("Hardware MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);

    // Create the embassy-net stack for the station interface (DHCP client).
    let net_config = embassy_net::Config::dhcpv4(Default::default());
    static NET_RESOURCES: StaticCell<embassy_net::StackResources<3>> = StaticCell::new();
    let net_resources = NET_RESOURCES.init(embassy_net::StackResources::new());
    let seed = (mac[4] as u64) << 8 | mac[5] as u64; // Pseudo-random seed from MAC
    let (stack, net_runner) = embassy_net::new(interfaces.station, net_config, net_resources, seed);

    // Store WifiController in a static cell so the wifi task can own it.
    static WIFI_CTRL: StaticCell<esp_radio::wifi::WifiController<'static>> = StaticCell::new();
    let wifi_ctrl_ref = WIFI_CTRL.init(wifi_controller);

    // Load / generate persistent configuration
    let config_store = ConfigStore::load_or_generate(peripherals.FLASH, &mac).await;
    let adv_name = config_store.config.adv_name();
    let cube_name: heapless::String<64> = config_store.config.name.clone();
    info!("Cube name: {}", cube_name.as_str());
    info!("BLE adv name: {}", adv_name.as_str());
    info!("Device ID: {}", config_store.config.device_id.as_str());

    // Check for stored WiFi credentials to auto-connect on boot.
    let auto_connect_creds = if config_store.config.has_wifi_creds() {
        info!("Stored WiFi credentials found — will auto-connect");
        // Populate the shared SSID buffer so the GATT characteristic is readable.
        let ssid_bytes = config_store.config.wifi_ssid.as_bytes();
        WIFI_SSID_BUF.lock(|cell| {
            let mut buf = cell.borrow_mut();
            let len = ssid_bytes.len().min(32);
            buf[..len].copy_from_slice(&ssid_bytes[..len]);
        });
        Some((
            config_store.config.wifi_ssid.clone(),
            config_store.config.wifi_password.clone(),
        ))
    } else {
        info!("No stored WiFi credentials");
        None
    };

    // Store the config in a static cell for shared task access.
    let _config_mutex = CONFIG_STORE.init(Mutex::new(config_store));
    // SAFETY: Single-threaded init before tasks are spawned.
    unsafe { CONFIG_REF = Some(_config_mutex); }

    // Initialize BLE controller
    let transport = BleConnector::new(peripherals.BT, Default::default()).unwrap();
    let controller = ExternalController::<_, 20>::new(transport);

    // Spawn auxiliary tasks
    spawner.spawn(net_task(net_runner).expect("net_task token"));
    spawner.spawn(wifi_task(wifi_ctrl_ref, stack).expect("wifi_task token"));
    spawner.spawn(supervisor_task().expect("supervisor_task token"));

    // If we have stored WiFi creds, send auto-connect command to WiFi task.
    if let Some((ssid, password)) = auto_connect_creds {
        info!("Sending auto-connect command to WiFi task");
        WIFI_CMD_CH
            .sender()
            .send(WifiCommand::AutoConnect { ssid, password })
            .await;
    }

    info!("Starting BLE stack...");

    // Run BLE (uses join pattern: runner + peripheral in same context)
    run_ble(controller, adv_name.as_str(), cube_name.as_str()).await;

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
    let _ = server.set(&server.cube_info.status, &[0u8; 32]);

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
    // Temporary buffer for the WiFi SSID, written first, then combined with
    // the password when wifi_pass is written.
    let mut pending_wifi_ssid: Option<heapless::String<32>> = None;

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
                            // Update the status characteristic with the current WiFi status
                            // before the read response is sent.
                            let len = WIFI_STATUS_LEN.load(core::sync::atomic::Ordering::Relaxed);
                            if len > 0 {
                                let mut buf = [0u8; 32];
                                WIFI_STATUS.lock(|cell| {
                                    let data = cell.borrow();
                                    buf[..len].copy_from_slice(&data[..len]);
                                });
                                let _ = server.set(&server.cube_info.status, &buf);
                            }

                            info!("[gatt] read: status (len={})", len);
                        } else if read_event.handle() == server.cube_info.wifi_ssid.handle {
                            // Return the current WiFi SSID from the shared buffer.
                            let mut buf = [0u8; 32];
                            WIFI_SSID_BUF.lock(|cell| {
                                let data = cell.borrow();
                                buf.copy_from_slice(&*data);
                            });
                            let _ = server.set(&server.cube_info.wifi_ssid, &buf);
                            info!("[gatt] read: wifi_ssid");
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
                                info!("[gatt] WiFi SSID received: {}", ssid);
                                let mut ssid_str = heapless::String::<32>::new();
                                let _ = ssid_str.push_str(ssid);
                                pending_wifi_ssid = Some(ssid_str);
                            }
                        } else if write_event.handle() == server.cube_info.wifi_pass.handle {
                            let data = write_event.data();
                            let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
                            if let Ok(pass) = core::str::from_utf8(&data[..end]) {
                                info!("[gatt] WiFi password received ({} chars)", pass.len());

                                // Combine with the pending SSID and trigger WiFi connection.
                                if let Some(ssid) = pending_wifi_ssid.take() {
                                    info!("[gatt] WiFi credentials complete — SSID='{}', triggering connect", ssid.as_str());
                                    let mut password = heapless::String::<64>::new();
                                    let _ = password.push_str(pass);
                                    BLE_EVENT_CH.sender().try_send(BleEvent::WifiCredentials {
                                        ssid,
                                        password,
                                    }).ok();
                                } else {
                                    info!("[gatt] WiFi password received but no SSID pending — ignoring");
                                }
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
    loop {
        Timer::after(Duration::from_secs(5)).await;
        let len = WIFI_STATUS_LEN.load(core::sync::atomic::Ordering::Relaxed);
        let mut buf = [0u8; 32];
        if len > 0 {
            WIFI_STATUS.lock(|cell| {
                let data = cell.borrow();
                buf[..len].copy_from_slice(&data[..len]);
            });
        }

        if server.cube_info.status.notify(conn, &buf).await.is_err() {
            info!("[notify] connection lost");
            break;
        }

        info!("[notify] sent status notify ({} bytes)", len);
    }
}

// Auxiliary embassy tasks

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, esp_radio::wifi::Interface<'static>>) {
    runner.run().await;
}

#[embassy_executor::task]
async fn wifi_task(
    controller: &'static mut esp_radio::wifi::WifiController<'static>,
    stack: embassy_net::Stack<'static>,
) {
    cubemaster_firmware::wifi::wifi_task_impl(
        WIFI_EVENT_CH.sender(),
        WIFI_CMD_CH.receiver(),
        controller,
        stack,
    ).await;
}

#[embassy_executor::task]
async fn supervisor_task() {
    info!("Supervisor task started");

    let ble_event_rx = BLE_EVENT_CH.receiver();
    let _ble_cmd_tx = BLE_CMD_CH.sender();
    let wifi_event_rx = WIFI_EVENT_CH.receiver();
    let wifi_cmd_tx = WIFI_CMD_CH.sender();

    // Track the current WiFi IP for status reporting.
    let mut _wifi_ip: Option<heapless::String<16>> = None;

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
                    info!("Supervisor: WiFi creds received — SSID='{}', persisting and connecting...", ssid.as_str());

                    // Clear the WiFi status immediately so the companion won't read stale data.
                    WIFI_STATUS_LEN.store(0, core::sync::atomic::Ordering::Relaxed);
                    WIFI_STATUS.lock(|cell| {
                        let mut buf = cell.borrow_mut();
                        for b in buf.iter_mut() { *b = 0; }
                    });

                    // Update the shared SSID buffer for GATT readback.
                    WIFI_SSID_BUF.lock(|cell| {
                        let mut buf = cell.borrow_mut();
                        for b in buf.iter_mut() { *b = 0; }
                        let bytes = ssid.as_bytes();
                        let len = bytes.len().min(32);
                        buf[..len].copy_from_slice(&bytes[..len]);
                    });

                    // Persist credentials to NVS flash via the shared config store.
                    // SAFETY: CONFIG_REF is set during init before tasks are spawned.
                    if let Some(store_mutex) = unsafe { CONFIG_REF } {
                        let mut store = store_mutex.lock().await;
                        store.set_wifi_creds(ssid.as_str(), password.as_str()).await;
                        info!("Supervisor: WiFi credentials persisted to flash");
                    }

                    // Send connect command to WiFi task.
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
                    // Persist renamed cube name to NVS.
                    // SAFETY: CONFIG_REF is set during init before tasks are spawned.
                    if let Some(store_mutex) = unsafe { CONFIG_REF } {
                        let mut store = store_mutex.lock().await;
                        store.set_name(name.as_str()).await;
                        info!("Supervisor: Cube name persisted to flash");
                    }
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
                    info!("Supervisor: WiFi connected! IP={}", ip.as_str());
                    _wifi_ip = Some(ip.clone());

                    // Write the WiFi status JSON into the shared static buffer.
                    // The GATT read handler will pick this up on next status read.
                    let status_msg = format_wifi_status_json(ip.as_str());
                    let len = status_msg.len().min(32);
                    WIFI_STATUS.lock(|cell| {
                        let mut buf = cell.borrow_mut();
                        buf[..len].copy_from_slice(&status_msg.as_bytes()[..len]);
                        for b in &mut buf[len..] { *b = 0; }
                    });
                    WIFI_STATUS_LEN.store(len, core::sync::atomic::Ordering::Relaxed);
                    info!("Supervisor: WiFi status updated ({} bytes)", len);
                }
                WifiEvent::Disconnected => {
                    info!("Supervisor: WiFi disconnected");
                    _wifi_ip = None;
                    WIFI_STATUS_LEN.store(0, core::sync::atomic::Ordering::Relaxed);
                }
                WifiEvent::Failed { reason } => {
                    info!("Supervisor: WiFi failed: {}", reason.as_str());
                    _wifi_ip = None;
                    WIFI_STATUS_LEN.store(0, core::sync::atomic::Ordering::Relaxed);
                }
            }
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}

/// Format a compact WiFi status JSON for the BLE status characteristic.
/// Must fit in 32 bytes. Format: {"w":1,"ip":"x.x.x.x"}
fn format_wifi_status_json(ip: &str) -> heapless::String<32> {
    let mut s = heapless::String::<32>::new();
    let _ = s.push_str("{\"w\":1,\"ip\":\"");
    let _ = s.push_str(ip);
    let _ = s.push_str("\"}");
    s
}
