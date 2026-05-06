//! BLE scanner. Discovers CubeMaster devices by advertising name prefix.

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use cubemaster_shared::ble::ADV_NAME_PREFIX;
use std::time::Duration;
use tokio::time;
use tracing::{debug, info};

/// A discovered CubeMaster device.
#[derive(Debug, Clone)]
pub struct DiscoveredCube {
    /// The BLE peripheral handle.
    pub peripheral: Peripheral,
    /// The advertised local name (e.g. "CubeMaster-AB12").
    pub name: String,
    /// RSSI signal strength.
    pub rssi: Option<i16>,
    /// The advertising short id (last 4 chars of name).
    pub short_id: String,
}

/// Get the default BLE adapter.
pub async fn get_adapter() -> Result<Adapter, BleError> {
    let manager = Manager::new().await.map_err(|e| BleError::Init(e.to_string()))?;
    let adapters = manager.adapters().await.map_err(|e| BleError::Init(e.to_string()))?;
    adapters.into_iter().next().ok_or(BleError::NoAdapter)
}

/// Scan for CubeMaster devices for the given duration.
pub async fn scan_for_cubes(
    adapter: &Adapter,
    duration: Duration,
) -> Result<Vec<DiscoveredCube>, BleError> {
    info!("Starting BLE scan for {} seconds...", duration.as_secs());

    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| BleError::Scan(e.to_string()))?;

    time::sleep(duration).await;

    adapter
        .stop_scan()
        .await
        .map_err(|e| BleError::Scan(e.to_string()))?;

    let peripherals = adapter
        .peripherals()
        .await
        .map_err(|e| BleError::Scan(e.to_string()))?;

    let mut cubes = Vec::new();
    for p in peripherals {
        if let Some(props) = p.properties().await.unwrap_or(None) {
            if let Some(ref name) = props.local_name {
                if name.starts_with(ADV_NAME_PREFIX) {
                    let short_id = name
                        .strip_prefix(ADV_NAME_PREFIX)
                        .unwrap_or("")
                        .to_string();
                    debug!("Found CubeMaster device: {} (RSSI: {:?})", name, props.rssi);
                    cubes.push(DiscoveredCube {
                        peripheral: p,
                        name: name.clone(),
                        rssi: props.rssi,
                        short_id,
                    });
                }
            }
        }
    }

    info!("Scan complete: found {} CubeMaster device(s)", cubes.len());
    Ok(cubes)
}

#[derive(Debug, Clone)]
pub enum BleError {
    Init(String),
    NoAdapter,
    Scan(String),
    Connect(String),
    Gatt(String),
    Pairing(String),
}

impl std::fmt::Display for BleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init(e) => write!(f, "BLE init error: {e}"),
            Self::NoAdapter => write!(f, "No BLE adapter found"),
            Self::Scan(e) => write!(f, "BLE scan error: {e}"),
            Self::Connect(e) => write!(f, "BLE connect error: {e}"),
            Self::Gatt(e) => write!(f, "BLE GATT error: {e}"),
            Self::Pairing(e) => write!(f, "BLE pairing error: {e}"),
        }
    }
}
