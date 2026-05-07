//! Local filesystem store for paired cubes.

use crate::ble::pairing::PairedCubeInfo;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

const APP_QUALIFIER: &str = "io";
const APP_ORG: &str = "cubemaster";
const APP_NAME: &str = "CubeMaster";
const STORE_FILENAME: &str = "paired_cubes.json";

/// On-disk format for the local store.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct LocalStore {
    /// List of paired cubes with their PSKs.
    pub cubes: Vec<PairedCubeInfo>,
}

impl LocalStore {
    /// Load the store from disk, or return an empty store if not found.
    pub fn load() -> Self {
        let path = Self::store_path();
        match fs::read_to_string(&path) {
            Ok(contents) => {
                match serde_json::from_str(&contents) {
                    Ok(store) => {
                        info!("Loaded {} paired cube(s) from {:?}", store_count(&store), path);
                        store
                    }
                    Err(e) => {
                        warn!("Failed to parse store at {:?}: {}", path, e);
                        Self::default()
                    }
                }
            }
            Err(_) => {
                info!("No existing store at {:?}, starting fresh", path);
                Self::default()
            }
        }
    }

    /// Save the store to disk.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::store_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }

        let json = serde_json::to_string_pretty(self).map_err(|e| format!("serialize: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("write: {e}"))?;
        info!("Saved {} paired cube(s) to {:?}", self.cubes.len(), path);
        Ok(())
    }

    /// Add a newly paired cube (or update if device_id already exists).
    pub fn add_or_update(&mut self, cube: PairedCubeInfo) {
        if let Some(existing) = self.cubes.iter_mut().find(|c| c.device_id == cube.device_id) {
            existing.cube_name = cube.cube_name;
            existing.psk = cube.psk;
            existing.ble_address = cube.ble_address;
            existing.last_connected = cube.last_connected;
            existing.short_id = cube.short_id;
        } else {
            self.cubes.push(cube);
        }
    }

    /// Remove a cube by device_id.
    pub fn remove(&mut self, device_id: &str) {
        self.cubes.retain(|c| c.device_id != device_id);
    }

    /// Find a cube by device_id.
    pub fn find(&self, device_id: &str) -> Option<&PairedCubeInfo> {
        self.cubes.iter().find(|c| c.device_id == device_id)
    }

    /// Get the platform-specific store file path.
    fn store_path() -> PathBuf {
        if let Some(proj) = ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME) {
            proj.config_dir().join(STORE_FILENAME)
        } else {
            PathBuf::from(STORE_FILENAME)
        }
    }
}

fn store_count(store: &LocalStore) -> usize {
    store.cubes.len()
}
