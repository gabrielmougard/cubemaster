//! BLE GATT service and characteristic UUIDs.
//!
//! We use full 128-bit UUIDs under a CubeMaster-owned base so the app can
//! filter precisely and we avoid collisions with generic 16-bit services
//! that many third-party beacons also advertise.
//!
//! Base UUID: `c0be-ma57-er00-XXXX-000000000000`
//! (read as "cube master" 0x00XXXX . The last 16 bits are the short id).

/// Prefix used in the BLE advertised local name, e.g. `CubeMaster-AB12`.
pub const ADV_NAME_PREFIX: &str = "CubeMaster-";

/// Length of the short id appended to the adv name (hex chars).
pub const ADV_NAME_ID_LEN: usize = 4;

// ---------------------------------------------------------------------------
// 128-bit UUIDs.  Spelled out explicitly so the same literals are visible on
// both sides of the link.  All base-UUID bytes use `c0bea577-0000-4000-8000-…`
// (a CubeMaster-owned random prefix).
// ---------------------------------------------------------------------------

/// Pairing + session service.  Exposed even when not yet paired so the app can
/// complete the initial pairing handshake.
pub const PAIRING_SERVICE_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000f001");
pub const PAIRING_CHALLENGE_CHAR_UUID: [u8; 16] =
    uuid_from_str("c0bea577-0000-4000-8000-00000000f002");
pub const PAIRING_CONFIRM_CHAR_UUID: [u8; 16] =
    uuid_from_str("c0bea577-0000-4000-8000-00000000f003");
pub const PAIRING_INFO_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000f004");

/// Control service: command + status + config.  Requires a paired + session
/// token (writes contain an authenticated envelope).
pub const CONTROL_SERVICE_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe0");
pub const CMD_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe1");
pub const STATUS_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe2");
pub const CONFIG_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe3");

/// Wi-Fi provisioning service: SSID + password handoff, WiFi status.
pub const WIFI_SERVICE_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000f010");
pub const WIFI_CREDS_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000f011");
pub const WIFI_STATUS_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000f012");

/// Upload service (stub for now. TODO: Full impl lands with T-005 upload tests).
pub const UPLOAD_SERVICE_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe4");
pub const UPLOAD_START_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe5");
pub const UPLOAD_CHUNK_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe6");
pub const UPLOAD_DONE_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe7");
pub const UPLOAD_LIST_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe8");

/// LED patterns service (stub).
pub const PATTERN_SERVICE_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe9");
pub const PATTERN_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffea");
pub const PATTERN_LIST_CHAR_UUID: [u8; 16] = uuid_from_str("c0bea577-0000-4000-8000-00000000ffeb");

/// Renders a 128-bit UUID string `"xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"` into a
/// big-endian 16-byte array suitable for use with BLE stacks.
///
/// `trouble-host` expects big-endian; `btleplug` exposes `uuid::Uuid` which
/// is also big-endian on `as_bytes()`.
pub const fn uuid_from_str(s: &str) -> [u8; 16] {
    let bytes = s.as_bytes();
    let mut out = [0u8; 16];
    let mut out_i = 0usize;
    let mut i = 0usize;
    while i < bytes.len() && out_i < 16 {
        let b = bytes[i];
        if b == b'-' {
            i += 1;
            continue;
        }

        let hi = hex_nibble(b);
        let lo = hex_nibble(bytes[i + 1]);
        out[out_i] = (hi << 4) | lo;
        out_i += 1;
        i += 2;
    }

    out
}

const fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

/// Reverses a UUID's byte order (useful for stacks that use little-endian).
pub const fn uuid_reversed(u: [u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    let mut i = 0;
    while i < 16 {
        out[i] = u[15 - i];
        i += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_parse_roundtrip() {
        let u = uuid_from_str("c0bea577-0000-4000-8000-00000000ffe1");
        assert_eq!(u[0], 0xc0);
        assert_eq!(u[1], 0xbe);
        assert_eq!(u[15], 0xe1);
    }
}
