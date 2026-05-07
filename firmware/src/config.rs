//! Persistent configuration stored in NVS flash.
//!
//! Uses `sequential-storage` for wear-leveled key-value persistence on the
//! ESP32-S3's internal flash. The NVS partition occupies a dedicated region
//! (0x9000..0xF000, 24 KiB = 6 sectors) leaving firmware and OTA slots intact.
//!
//! Key IDs (u8) for the sequential-storage map:
//!   - 0x01 → cube friendly name (UTF-8, max 64 bytes)
//!   - 0x02 → 32-byte pairing pre-shared key
//!   - 0x03 → WiFi SSID (max 32 bytes)
//!   - 0x04 → WiFi password (max 64 bytes)
//!   - 0x05 → 4-char hex BLE advertising suffix

use defmt::info;
use heapless::String;

use embassy_embedded_hal::adapter::BlockingAsync;
use esp_storage::FlashStorage;
use sequential_storage::cache::NoCache;
use sequential_storage::map::{MapConfig, MapStorage};

/// NVS flash partition range: 0x9000..0xF000 (24 KiB, 6 sectors).
/// Must have at least 2 pages (sectors) for sequential-storage.
const NVS_FLASH_RANGE: core::ops::Range<u32> = 0x9000..0xF000;

/// Key IDs for the map store.
const KEY_NAME: u8 = 0x01;
const KEY_PSK: u8 = 0x02;
const KEY_WIFI_SSID: u8 = 0x03;
const KEY_WIFI_PASS: u8 = 0x04;
const KEY_ADV_ID: u8 = 0x05;

/// Type alias for the async flash adapter.
type AsyncFlash = BlockingAsync<FlashStorage<'static>>;

/// In-memory representation of cube configuration.
/// On boot we load from flash; on change we flush back.
pub struct CubeConfig {
    /// Cube user-assigned friendly name.
    pub name: String<64>,
    /// BLE advertising short ID (e.g. "AB12").
    pub adv_id: String<4>,
    /// 256-bit pairing pre-shared key.
    pub psk: [u8; 32],
    /// Stored WiFi SSID.
    pub wifi_ssid: String<32>,
    /// Stored WiFi password.
    pub wifi_password: String<64>,
    /// Device ID (6-byte MAC as lowercase hex).
    pub device_id: String<12>,
}

impl CubeConfig {
    /// Generate initial config from the device's MAC address.
    ///
    /// Called on first boot (or when NVS is empty / factory-reset).
    pub fn generate_from_mac(mac: &[u8; 6]) -> Self {
        // Generate friendly name deterministically from MAC.
        let (name_buf, name_len) = cubemaster_shared::naming::generate_name(mac);
        let name_str = core::str::from_utf8(&name_buf[..name_len]).unwrap_or("cube");
        let mut name = String::new();
        let _ = name.push_str(name_str);

        // Advertising ID: last 2 bytes of MAC as uppercase hex.
        let adv_id = format_adv_id(mac);

        // Device ID: full 6-byte MAC as lowercase hex.
        let device_id = format_device_id(mac);

        // PSK: derived from MAC (temporary. TODO: replaced by true RNG later).
        let mut psk = [0u8; 32];
        for i in 0..32 {
            psk[i] = mac[i % 6].wrapping_add(i as u8).wrapping_mul(0x6D);
        }

        info!("Generated cube config: name={}, adv_id={}", name_str, adv_id.as_str());

        Self {
            name,
            adv_id,
            psk,
            wifi_ssid: String::new(),
            wifi_password: String::new(),
            device_id,
        }
    }

    /// Returns true if WiFi credentials have been configured.
    pub fn has_wifi_creds(&self) -> bool {
        !self.wifi_ssid.is_empty()
    }

    /// Returns the BLE advertising name: "CubeMaster-XXXX".
    pub fn adv_name(&self) -> String<20> {
        let mut s: String<20> = String::new();
        let _ = s.push_str(cubemaster_shared::ble::ADV_NAME_PREFIX);
        let _ = s.push_str(self.adv_id.as_str());
        s
    }
}

fn format_adv_id(mac: &[u8; 6]) -> String<4> {
    let mut s: String<4> = String::new();
    let bytes = [mac[4], mac[5]];
    for b in bytes {
        let hi = b >> 4;
        let lo = b & 0x0F;
        let _ = s.push(nibble_char(hi));
        let _ = s.push(nibble_char(lo));
    }
    s
}

fn format_device_id(mac: &[u8; 6]) -> String<12> {
    let mut s: String<12> = String::new();
    for &b in mac {
        let hi = b >> 4;
        let lo = b & 0x0F;
        let _ = s.push(nibble_char_lower(hi));
        let _ = s.push(nibble_char_lower(lo));
    }
    s
}

fn nibble_char(n: u8) -> char {
    if n < 10 { (b'0' + n) as char } else { (b'A' + n - 10) as char }
}

fn nibble_char_lower(n: u8) -> char {
    if n < 10 { (b'0' + n) as char } else { (b'a' + n - 10) as char }
}

// Flash persistence via sequential-storage
/// Persistent configuration store backed by ESP32-S3 internal flash.
///
/// Owns the flash storage and provides async methods for reading and writing
/// configuration. Must be initialized once with the FLASH peripheral and then
/// stored in a static cell for shared access across tasks.
pub struct ConfigStore {
    pub config: CubeConfig,
    storage: MapStorage<u8, AsyncFlash, NoCache>,
    buf: [u8; 128],
}

impl ConfigStore {
    /// Load config from flash, or generate defaults if empty.
    ///
    /// Takes ownership of the FLASH peripheral. This can only be called once
    /// (FlashStorage panics on second instantiation).
    pub async fn load_or_generate(flash: esp_hal::peripherals::FLASH<'static>, mac: &[u8; 6]) -> Self {
        let raw_flash = FlashStorage::new(flash);
        let async_flash = BlockingAsync::new(raw_flash);
        let map_config: MapConfig<AsyncFlash> = MapConfig::new(NVS_FLASH_RANGE);
        let mut storage = MapStorage::<u8, _, _>::new(async_flash, map_config, NoCache::new());
        let mut buf = [0u8; 128];

        // Attempt to read the cube name from flash (key 0x01).
        let stored_name: Option<String<64>> =
            match storage.fetch_item::<&[u8]>(&mut buf, &KEY_NAME).await {
                Ok(Some(bytes)) => {
                    if let Ok(s) = core::str::from_utf8(bytes) {
                        let mut name = String::new();
                        let _ = name.push_str(s);
                        Some(name)
                    } else {
                        None
                    }
                }
                _ => None,
            };

        let found_in_flash = stored_name.is_some();
        let config = if let Some(name) = stored_name {
            info!("Config loaded from flash: name={}", name.as_str());

            let adv_id = read_string::<4>(&mut storage, &mut buf, KEY_ADV_ID)
                .await
                .unwrap_or_else(|| format_adv_id(mac));

            let wifi_ssid = read_string::<32>(&mut storage, &mut buf, KEY_WIFI_SSID)
                .await
                .unwrap_or_else(String::new);

            let wifi_password = read_string::<64>(&mut storage, &mut buf, KEY_WIFI_PASS)
                .await
                .unwrap_or_else(String::new);

            let mut psk = [0u8; 32];
            if let Ok(Some(bytes)) = storage.fetch_item::<&[u8]>(&mut buf, &KEY_PSK).await {
                if bytes.len() == 32 {
                    psk.copy_from_slice(bytes);
                }
            }

            let device_id = format_device_id(mac);
            let cfg = CubeConfig {
                name,
                adv_id,
                psk,
                wifi_ssid,
                wifi_password,
                device_id,
            };

            if cfg.has_wifi_creds() {
                info!("Stored WiFi SSID: {}", cfg.wifi_ssid.as_str());
            }

            cfg
        } else {
            // First boot or corrupted flash — generate defaults.
            info!("No config in flash, generating from MAC...");
            CubeConfig::generate_from_mac(mac)
        };

        let mut store = Self { config, storage, buf };

        // On first boot, persist the generated defaults.
        if !found_in_flash {
            store.save().await;
        }

        store
    }

    /// Persist current config to flash.
    pub async fn save(&mut self) {
        let _ = self.storage
            .store_item(&mut self.buf, &KEY_NAME, &self.config.name.as_bytes())
            .await;

        let _ = self.storage
            .store_item(&mut self.buf, &KEY_ADV_ID, &self.config.adv_id.as_bytes())
            .await;

        let _ = self.storage
            .store_item(&mut self.buf, &KEY_PSK, &&self.config.psk[..])
            .await;

        if !self.config.wifi_ssid.is_empty() {
            let _ = self.storage
                .store_item(&mut self.buf, &KEY_WIFI_SSID, &self.config.wifi_ssid.as_bytes())
                .await;
        }

        if !self.config.wifi_password.is_empty() {
            let _ = self.storage
                .store_item(&mut self.buf, &KEY_WIFI_PASS, &self.config.wifi_password.as_bytes())
                .await;
        }

        info!("Config persisted to flash");
    }

    /// Update the cube name and persist.
    pub async fn set_name(&mut self, new_name: &str) {
        self.config.name.clear();
        let _ = self.config.name.push_str(new_name);
        self.save().await;
    }

    /// Update WiFi credentials and persist.
    pub async fn set_wifi_creds(&mut self, ssid: &str, password: &str) {
        self.config.wifi_ssid.clear();
        let _ = self.config.wifi_ssid.push_str(ssid);
        self.config.wifi_password.clear();
        let _ = self.config.wifi_password.push_str(password);
        self.save().await;
    }

    /// Clear WiFi credentials from config and flash.
    pub async fn clear_wifi_creds(&mut self) {
        self.config.wifi_ssid.clear();
        self.config.wifi_password.clear();
        self.save().await;
    }

    /// Regenerate PSK from a true random source and persist.
    pub async fn regenerate_psk(&mut self, rng_bytes: &[u8; 32]) {
        self.config.psk = *rng_bytes;
        self.save().await;
    }
}

/// Helper: Read a string value from flash by key ID.
async fn read_string<const N: usize>(
    storage: &mut MapStorage<u8, AsyncFlash, NoCache>,
    buf: &mut [u8],
    key: u8,
) -> Option<String<N>> {
    match storage.fetch_item::<&[u8]>(buf, &key).await {
        Ok(Some(bytes)) => {
            if let Ok(s) = core::str::from_utf8(bytes) {
                let mut result = String::new();
                let _ = result.push_str(s);
                Some(result)
            } else {
                None
            }
        }
        _ => None,
    }
}
