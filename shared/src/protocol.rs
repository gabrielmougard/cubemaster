//! Wire protocol message types for BLE and Wi-Fi communication.
//!
//! Message format: `[type: u8][json_payload: N bytes]`
//!
//! All messages are length-delimited by the underlying transport (BLE GATT
//! characteristics have a known length, HTTP bodies have Content-Length, WS
//! frames have a length field).

use heapless::String;
use serde::{Deserialize, Serialize};

/// Discriminant byte for a protocol message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    CmdPlay = 0x01,
    CmdStop = 0x02,
    CmdVolume = 0x03,
    CmdWifiConfig = 0x04,
    StatusUpdate = 0x05,
    UploadStart = 0x06,
    UploadChunk = 0x07,
    UploadDone = 0x08,
    ConfigKeyword = 0x09,
    /// Cube rename command.
    CmdRename = 0x0A,
}

impl MessageType {
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::CmdPlay),
            0x02 => Some(Self::CmdStop),
            0x03 => Some(Self::CmdVolume),
            0x04 => Some(Self::CmdWifiConfig),
            0x05 => Some(Self::StatusUpdate),
            0x06 => Some(Self::UploadStart),
            0x07 => Some(Self::UploadChunk),
            0x08 => Some(Self::UploadDone),
            0x09 => Some(Self::ConfigKeyword),
            0x0A => Some(Self::CmdRename),
            _ => None,
        }
    }
}

// Command payloads (App -> Cube)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdPlay {
    pub id: u8,
    pub face: u8,
    pub volume: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdStop {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdVolume {
    pub volume: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdWifiConfig {
    /// SSID (max 32 bytes).
    pub ssid: String<32>,
    /// WPA2 password (max 64 bytes).
    pub password: String<64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdRename {
    /// New cube friendly name (max 64 UTF-8 bytes).
    pub name: String<64>,
}

// Status payload (Cube -> App)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubeStatus {
    /// Firmware version string.
    pub firmware: String<16>,
    /// Uptime in seconds.
    pub uptime: u32,
    /// SD card free space in bytes (0 if no SD).
    pub sd_free: u32,
    /// Number of connected BLE clients.
    pub ble_clients: u8,
    /// Currently playing sound ID, or None.
    pub playing: Option<u8>,
    /// WiFi IP address as a dotted-quad string or empty if not connected.
    pub wifi_ip: String<16>,
    /// Cube friendly name.
    pub cube_name: String<64>,
}

// WiFi status (Cube -> App, via BLE notify)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WifiState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiStatus {
    pub state: WifiState,
    /// IP address if connected.
    pub ip: String<16>,
    /// RSSI of the WiFi connection.
    pub rssi: i8,
}

// Codec helpers

/// Encode a message type + JSON payload into a buffer.  Returns the number of
/// bytes written.
///
/// Format: `[type_byte][json...]`
pub fn encode_message<T: Serialize>(
    msg_type: MessageType,
    payload: &T,
    buf: &mut [u8],
) -> Result<usize, EncodeError> {
    if buf.is_empty() {
        return Err(EncodeError::BufferTooSmall);
    }

    buf[0] = msg_type as u8;
    let json_len = serde_json_core::to_slice(payload, &mut buf[1..])
        .map_err(|_| EncodeError::SerializationFailed)?;
    Ok(1 + json_len)
}

/// Decode a message type from the first byte.
pub fn decode_type(buf: &[u8]) -> Option<MessageType> {
    buf.first().and_then(|&b| MessageType::from_u8(b))
}

/// Decode the JSON payload portion (everything after the type byte).
pub fn decode_payload<'a, T: Deserialize<'a>>(buf: &'a [u8]) -> Result<T, DecodeError> {
    if buf.len() < 2 {
        return Err(DecodeError::TooShort);
    }

    let (val, _) = serde_json_core::from_slice(&buf[1..]).map_err(|_| DecodeError::DeserializationFailed)?;
    Ok(val)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    BufferTooSmall,
    SerializationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    TooShort,
    DeserializationFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let cmd = CmdPlay {
            id: 3,
            face: 1,
            volume: 80,
        };
        let mut buf = [0u8; 128];
        let len = encode_message(MessageType::CmdPlay, &cmd, &mut buf).unwrap();
        assert_eq!(buf[0], 0x01);
        let decoded: CmdPlay = decode_payload(&buf[..len]).unwrap();
        assert_eq!(decoded.id, 3);
        assert_eq!(decoded.volume, 80);
    }
}
