//! Persistent configuration stored in NVS flash.
//!
//! Uses `sequential-storage` for wear-leveled key-value persistence.
//! Keys:
//!   - "name"     → cube friendly name (UTF-8, max 64 bytes)
//!   - "psk"      → 32-byte pairing pre-shared key
//!   - "wifi_ss"  → WiFi SSID (max 32 bytes)
//!   - "wifi_pw"  → WiFi password (max 64 bytes)
//!   - "paired"   → list of paired app hashes (up to 8 × 32 bytes)
//!   - "adv_id"   → 4-char hex BLE advertising suffix

use defmt::info;
use heapless::String;

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

        // PSK: generated from a hardware random source (caller must seed it).
        // For now, derive from MAC. Replaced at runtime by true RNG once
        // esp-hal RNG is available in the task context.
        let mut psk = [0u8; 32];
        // Fill with a pseudo-random pattern from MAC bytes (TEMPORARY).
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

// Flash persistence (this is stub. Real impl requires async flash driver)

/// Placeholder: In the real implementation this will use `sequential-storage`
/// with `esp-storage` to persist CubeConfig fields into flash.  For this
/// bootstrap we do everything in-RAM so the type compiles and the APIs are
/// wired.
///
/// TODO (T-005 follow-up): Wire `esp_storage::FlashStorage` + sequential_storage::map
pub struct ConfigStore {
    pub config: CubeConfig,
}

impl ConfigStore {
    /// Load config from flash, or generate defaults if empty.
    pub fn load_or_generate(mac: &[u8; 6]) -> Self {
        // TODO: Read from flash partition using sequential-storage.
        // For now, always generate fresh config.
        let config = CubeConfig::generate_from_mac(mac);
        info!("Config loaded (in-RAM stub)");
        Self { config }
    }

    /// Persist current config to flash.
    pub fn save(&self) {
        // TODO: Write to flash using sequential-storage.
        info!("Config save called (stub — not yet persisted to flash)");
    }

    /// Update the cube name and persist.
    pub fn set_name(&mut self, new_name: &str) {
        self.config.name.clear();
        let _ = self.config.name.push_str(new_name);
        self.save();
    }

    /// Update WiFi credentials and persist.
    pub fn set_wifi_creds(&mut self, ssid: &str, password: &str) {
        self.config.wifi_ssid.clear();
        let _ = self.config.wifi_ssid.push_str(ssid);
        self.config.wifi_password.clear();
        let _ = self.config.wifi_password.push_str(password);
        self.save();
    }

    /// Regenerate PSK from a true random source and persist.
    pub fn regenerate_psk(&mut self, rng_bytes: &[u8; 32]) {
        self.config.psk = *rng_bytes;
        self.save();
    }
}
