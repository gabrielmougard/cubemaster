//! mDNS service discovery for finding CubeMaster devices on the local network.

use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info};

/// A cube discovered via mDNS on the local network.
#[derive(Debug, Clone)]
pub struct NetworkCube {
    /// The cube's device ID from TXT record.
    pub device_id: String,
    /// The cube's friendly name from TXT record.
    pub cube_name: String,
    /// IP address + port.
    pub address: String,
    /// Firmware version from TXT record.
    pub firmware_version: String,
}

/// Browse for CubeMaster devices on the network using mDNS.
///
/// Returns a receiver that will yield discovered cubes as they appear.
pub fn browse_cubes() -> Result<mpsc::Receiver<NetworkCube>, MdnsError> {
    let mdns = ServiceDaemon::new().map_err(|e| MdnsError::Init(e.to_string()))?;

    let service_type = format!("{}.", cubemaster_shared::MDNS_SERVICE_TYPE);
    let receiver = mdns
        .browse(&service_type)
        .map_err(|e| MdnsError::Browse(e.to_string()))?;

    let (tx, rx) = mpsc::channel(16);

    // Spawn a task to process mDNS events.
    tokio::spawn(async move {
        while let Ok(event) = receiver.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let txt: HashMap<String, String> = info
                        .get_properties()
                        .iter()
                        .map(|p| (p.key().to_string(), p.val_str().to_string()))
                        .collect();

                    let device_id = txt.get("device_id").cloned().unwrap_or_default();
                    let cube_name = txt.get("name").cloned().unwrap_or_default();
                    let firmware_version = txt.get("fw").cloned().unwrap_or_default();

                    // Get the first IPv4 address.
                    let ip = info
                        .get_addresses()
                        .iter()
                        .find(|a| a.is_ipv4())
                        .map(|a| a.to_string())
                        .unwrap_or_default();

                    if ip.is_empty() {
                        debug!("Resolved cube without IPv4 address, skipping");
                        continue;
                    }

                    let port = info.get_port();
                    let address = format!("{ip}:{port}");

                    info!("Discovered cube on network: {} at {}", cube_name, address);
                    let cube = NetworkCube {
                        device_id,
                        cube_name,
                        address,
                        firmware_version,
                    };

                    if tx.send(cube).await.is_err() {
                        break; // Receiver dropped.
                    }
                }
                ServiceEvent::ServiceRemoved(_, name) => {
                    debug!("Cube removed from network: {}", name);
                }
                _ => {}
            }
        }
    });

    Ok(rx)
}

/// Perform a one-shot scan: browse for `duration` and return all found cubes.
pub async fn scan_network(duration: Duration) -> Result<Vec<NetworkCube>, MdnsError> {
    let mut rx = browse_cubes()?;
    let mut cubes = Vec::new();

    let deadline = tokio::time::sleep(duration);
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            Some(cube) = rx.recv() => {
                cubes.push(cube);
            }
            _ = &mut deadline => {
                break;
            }
        }
    }

    info!("Network scan complete: found {} cube(s)", cubes.len());
    Ok(cubes)
}

#[derive(Debug, Clone)]
pub enum MdnsError {
    Init(String),
    Browse(String),
}

impl std::fmt::Display for MdnsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init(e) => write!(f, "mDNS init error: {e}"),
            Self::Browse(e) => write!(f, "mDNS browse error: {e}"),
        }
    }
}
