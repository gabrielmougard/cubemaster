//! BLE GATT client. Connects to a cube and performs read/write/subscribe.

use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use cubemaster_shared::ble;
use std::time::Duration;
use tokio::time;
use tracing::{debug, info};
use uuid::Uuid;

use super::scanner::BleError;

pub struct CubeConnection {
    pub peripheral: Peripheral,
    /// Discovered GATT characteristics keyed by UUID.
    pub characteristics: Vec<Characteristic>,
}

impl CubeConnection {
    pub async fn connect(peripheral: Peripheral) -> Result<Self, BleError> {
        info!("Connecting to cube...");

        peripheral
            .connect()
            .await
            .map_err(|e| BleError::Connect(e.to_string()))?;

        time::sleep(Duration::from_millis(500)).await;

        peripheral
            .discover_services()
            .await
            .map_err(|e| BleError::Gatt(e.to_string()))?;

        let characteristics = peripheral.characteristics().into_iter().collect();

        info!("Connected and discovered services");
        Ok(Self {
            peripheral,
            characteristics,
        })
    }

    pub async fn disconnect(&self) -> Result<(), BleError> {
        self.peripheral
            .disconnect()
            .await
            .map_err(|e| BleError::Connect(e.to_string()))
    }

    /// Find a characteristic by its 128-bit UUID.
    pub fn find_char(&self, uuid_bytes: &[u8; 16]) -> Option<&Characteristic> {
        let target = Uuid::from_bytes(*uuid_bytes);
        self.characteristics.iter().find(|c| c.uuid == target)
    }

    /// Write data to a characteristic (with response).
    pub async fn write_char(&self, uuid_bytes: &[u8; 16], data: &[u8]) -> Result<(), BleError> {
        let char = self
            .find_char(uuid_bytes)
            .ok_or_else(|| BleError::Gatt("Characteristic not found".into()))?;
        self.peripheral
            .write(char, data, WriteType::WithResponse)
            .await
            .map_err(|e| BleError::Gatt(e.to_string()))?;
        debug!("Wrote {} bytes to {:?}", data.len(), char.uuid);
        Ok(())
    }

    /// Read data from a characteristic.
    pub async fn read_char(&self, uuid_bytes: &[u8; 16]) -> Result<Vec<u8>, BleError> {
        let char = self
            .find_char(uuid_bytes)
            .ok_or_else(|| BleError::Gatt("Characteristic not found".into()))?;
        let data = self
            .peripheral
            .read(char)
            .await
            .map_err(|e| BleError::Gatt(e.to_string()))?;
        debug!("Read {} bytes from {:?}", data.len(), char.uuid);
        Ok(data)
    }

    /// Subscribe to notifications on a characteristic.
    pub async fn subscribe_char(&self, uuid_bytes: &[u8; 16]) -> Result<(), BleError> {
        let char = self
            .find_char(uuid_bytes)
            .ok_or_else(|| BleError::Gatt("Characteristic not found".into()))?;
        self.peripheral
            .subscribe(char)
            .await
            .map_err(|e| BleError::Gatt(e.to_string()))?;
        debug!("Subscribed to notifications on {:?}", char.uuid);
        Ok(())
    }

    /// Send WiFi credentials to the cube via the WiFi provisioning service.
    pub async fn send_wifi_credentials(
        &self,
        ssid: &str,
        password: &str,
    ) -> Result<(), BleError> {
        use cubemaster_shared::protocol::{CmdWifiConfig, MessageType, encode_message};

        let cmd = CmdWifiConfig {
            ssid: ssid.try_into().map_err(|_| BleError::Gatt("SSID too long".into()))?,
            password: password
                .try_into()
                .map_err(|_| BleError::Gatt("Password too long".into()))?,
        };

        let mut buf = [0u8; 128];
        let len = encode_message(MessageType::CmdWifiConfig, &cmd, &mut buf)
            .map_err(|_| BleError::Gatt("Failed to encode WiFi config".into()))?;

        self.write_char(&ble::WIFI_CREDS_CHAR_UUID, &buf[..len])
            .await?;

        info!("WiFi credentials sent to cube");
        Ok(())
    }

    /// Send a play command to the cube.
    pub async fn send_play(&self, id: u8, face: u8, volume: u8) -> Result<(), BleError> {
        use cubemaster_shared::protocol::{CmdPlay, MessageType, encode_message};

        let cmd = CmdPlay { id, face, volume };
        let mut buf = [0u8; 64];
        let len = encode_message(MessageType::CmdPlay, &cmd, &mut buf)
            .map_err(|_| BleError::Gatt("Failed to encode play cmd".into()))?;

        self.write_char(&ble::CMD_CHAR_UUID, &buf[..len]).await?;
        info!("Play command sent: id={}, face={}, vol={}", id, face, volume);
        Ok(())
    }

    /// Send a stop command.
    pub async fn send_stop(&self) -> Result<(), BleError> {
        use cubemaster_shared::protocol::{CmdStop, MessageType, encode_message};

        let cmd = CmdStop {};
        let mut buf = [0u8; 16];
        let len = encode_message(MessageType::CmdStop, &cmd, &mut buf)
            .map_err(|_| BleError::Gatt("Failed to encode stop cmd".into()))?;

        self.write_char(&ble::CMD_CHAR_UUID, &buf[..len]).await?;
        info!("Stop command sent");
        Ok(())
    }

    /// Send a rename command.
    pub async fn send_rename(&self, new_name: &str) -> Result<(), BleError> {
        use cubemaster_shared::protocol::{CmdRename, MessageType, encode_message};

        let cmd = CmdRename {
            name: new_name
                .try_into()
                .map_err(|_| BleError::Gatt("Name too long".into()))?,
        };
        let mut buf = [0u8; 128];
        let len = encode_message(MessageType::CmdRename, &cmd, &mut buf)
            .map_err(|_| BleError::Gatt("Failed to encode rename cmd".into()))?;

        self.write_char(&ble::CONFIG_CHAR_UUID, &buf[..len]).await?;
        info!("Rename command sent: {}", new_name);
        Ok(())
    }
}
