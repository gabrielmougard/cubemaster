//! BLE pairing protocol implementation.
//!
//! Implements the BLE-bootstrapped PSK pairing flow described in
//! `shared/src/auth.rs`.
//! TODO: not used for now since the cube has no visual output for the code yet.

use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;
use tracing::info;

use super::client::CubeConnection;
use super::scanner::BleError;
use cubemaster_shared::ble;

type HmacSha256 = Hmac<Sha256>;

/// Initiate the pairing flow with a connected cube.
///
/// Returns the 6-digit code the user must verify (shown by the cube)
/// and the app nonce needed for the confirmation step.
pub async fn initiate_pairing(
    conn: &CubeConnection,
) -> Result<PairingSession, BleError> {
    // Generate a random 32-byte app nonce.
    let mut rng = rand::rng();
    let mut app_nonce = [0u8; 32];
    rng.fill(&mut app_nonce);

    // Encode the pairing challenge and write to the cube.
    let challenge = cubemaster_shared::auth::PairingChallenge { app_nonce };
    let json = serde_json::to_vec(&challenge)
        .map_err(|e| BleError::Pairing(format!("serialize: {e}")))?;

    conn.write_char(&ble::PAIRING_CHALLENGE_CHAR_UUID, &json)
        .await?;

    info!("Pairing challenge sent, waiting for cube to present code...");

    // Read the pairing info characteristic to get the 6-digit code.
    // (In production, the user would read this from the cube's LED display.)
    let info_data = conn.read_char(&ble::PAIRING_INFO_CHAR_UUID).await?;
    let code = String::from_utf8(info_data)
        .map_err(|_| BleError::Pairing("Invalid pairing code encoding".into()))?;

    info!("Cube presented pairing code: {}", code);

    Ok(PairingSession { app_nonce, code })
}

/// Confirm pairing by sending the user-verified code back to the cube.
///
/// On success, returns the PSK that should be stored locally.
pub async fn confirm_pairing(
    conn: &CubeConnection,
    session: &PairingSession,
    user_code: &str,
) -> Result<PairedCubeInfo, BleError> {
    if user_code != session.code {
        return Err(BleError::Pairing("Code mismatch".into()));
    }

    // Build confirmation message.
    let mut code_bytes: heapless::String<6> = Default::default();
    let _ = code_bytes.push_str(user_code);

    let confirm = cubemaster_shared::auth::PairingConfirm {
        code: code_bytes,
        app_nonce: session.app_nonce,
    };
    let json = serde_json::to_vec(&confirm)
        .map_err(|e| BleError::Pairing(format!("serialize confirm: {e}")))?;

    conn.write_char(&ble::PAIRING_CONFIRM_CHAR_UUID, &json)
        .await?;

    // Read the pairing result (masked PSK + device info).
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let result_data = conn.read_char(&ble::PAIRING_CONFIRM_CHAR_UUID).await?;
    let result: cubemaster_shared::auth::PairingResult = serde_json::from_slice(&result_data)
        .map_err(|e| BleError::Pairing(format!("deserialize result: {e}")))?;

    // Unmask the PSK: PSK = masked_psk XOR SHA-256(app_nonce ++ code)
    let psk = unmask_psk(&result.masked_psk, &session.app_nonce, user_code);

    info!(
        "Pairing complete! Cube: {} ({})",
        result.cube_name.as_str(),
        result.device_id.as_str()
    );

    Ok(PairedCubeInfo {
        device_id: result.device_id.to_string(),
        cube_name: result.cube_name.to_string(),
        psk,
    })
}

/// Create an HMAC session response for an already-paired cube's challenge.
pub fn compute_session_hmac(psk: &[u8; 32], challenge: &[u8; 16]) -> [u8; 32] {
    let mut mac =
        HmacSha256::new_from_slice(psk).expect("HMAC can take key of any size");
    mac.update(challenge);
    let result = mac.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result.into_bytes());
    out
}

/// Compute the HTTP auth token for a given nonce.
pub fn compute_http_auth(psk: &[u8; 32], nonce: &[u8; 16]) -> String {
    let hmac = compute_session_hmac(psk, nonce);
    hex::encode(hmac)
}

fn unmask_psk(masked: &[u8; 32], app_nonce: &[u8; 32], code: &str) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(app_nonce);
    hasher.update(code.as_bytes());
    let mask = hasher.finalize();

    let mut psk = [0u8; 32];
    for i in 0..32 {
        psk[i] = masked[i] ^ mask[i];
    }
    psk
}

/// Intermediate state during the pairing flow.
pub struct PairingSession {
    pub app_nonce: [u8; 32],
    /// The 6-digit code shown by the cube.
    pub code: String,
}

/// Information about a successfully paired cube.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PairedCubeInfo {
    pub device_id: String,
    pub cube_name: String,
    #[serde(with = "hex_serde")]
    pub psk: [u8; 32],
}

/// Hex serialization for the PSK in JSON config files.
mod hex_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        let mut arr = [0u8; 32];
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("PSK must be 32 bytes"));
        }
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}
