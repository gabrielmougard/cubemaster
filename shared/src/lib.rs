#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

pub mod auth;
pub mod ble;
pub mod led;
pub mod naming;
pub mod protocol;
pub mod sound;

/// Maximum size of a single BLE characteristic payload (MTU 512 minus ATT
/// overhead).  Stay conservative to work with MTU 247 as well.
pub const BLE_MAX_PAYLOAD: usize = 244;

/// Well-known HTTP port exposed by the cube once it is on Wi-Fi.
pub const HTTP_PORT: u16 = 8080;

/// Well-known mDNS service name used by the cube.  The companion app browses
/// `_cubemaster._tcp.local.` to discover paired cubes on the network.
pub const MDNS_SERVICE: &str = "_cubemaster._tcp.local.";
pub const MDNS_SERVICE_TYPE: &str = "_cubemaster._tcp";

/// Length of a cube device ID (first 6 bytes of the MAC as lowercase hex).
pub const DEVICE_ID_LEN: usize = 12;

/// Length of the bootstrap pairing PSK in bytes (256-bit).
pub const PAIRING_PSK_LEN: usize = 32;

/// Length of the short human-verifiable pairing code shown by the cube
/// during first-time pairing.  6 decimal digits.
pub const PAIRING_CODE_LEN: usize = 6;
