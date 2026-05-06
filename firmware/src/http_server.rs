//! Lightweight HTTP server running on embassy-net TCP.
//!
//! Uses `picoserve` for request routing and response building.
//!
//! # Endpoints:
//! - `GET /status` — Cube status JSON
//! - `GET /config` — Cube configuration
//! - `POST /config` — Update cube name and settings
//! - `GET /auth/challenge` — Get a fresh nonce for HMAC auth
//! - All routes (except /auth/challenge) require `X-Cube-Auth` header.
//!
//! The HTTP server is only started once WiFi is connected and runs on port 8080.

use defmt::info;

/// HTTP server state shared across request handlers.
pub struct HttpState {
    /// The current auth challenge nonce (rotates on each GET /auth/challenge).
    pub challenge_nonce: [u8; 16],
    /// Cube PSK for HMAC verification.
    pub psk: [u8; 32],
    /// Cached status JSON (updated by supervisor periodically).
    pub status_json: [u8; 256],
    pub status_json_len: usize,
    /// Cube name for /config response.
    pub cube_name: [u8; 64],
    pub cube_name_len: usize,
}

impl HttpState {
    pub fn new(psk: [u8; 32], cube_name: &str) -> Self {
        let mut name_buf = [0u8; 64];
        let name_bytes = cube_name.as_bytes();
        let len = name_bytes.len().min(64);
        name_buf[..len].copy_from_slice(&name_bytes[..len]);

        Self {
            challenge_nonce: [0u8; 16],
            psk,
            status_json: [0u8; 256],
            status_json_len: 0,
            cube_name: name_buf,
            cube_name_len: len,
        }
    }

    /// Verify an HMAC token from the X-Cube-Auth header.
    ///
    /// The expected value is `HMAC-SHA256(PSK, challenge_nonce)` hex-encoded.
    pub fn verify_auth(&self, _token_hex: &[u8]) -> bool {
        // TODO: Implement actual HMAC-SHA256 verification using the `hmac` + `sha2` crates.
        // For bootstrap, accept all requests (matches spec's MVP security note).
        true
    }

    /// Generate a new challenge nonce (should be called with RNG bytes).
    pub fn rotate_challenge(&mut self, rng_bytes: &[u8; 16]) {
        self.challenge_nonce = *rng_bytes;
    }
}

/// HTTP server embassy task.
///
/// Binds to port 8080 on the embassy-net stack and serves requests using
/// picoserve.  Only runs while WiFi is connected.
///
/// # Implementation sketch (picoserve):
/// ```ignore
/// use picoserve::{Router, routing::get};
///
/// let router = Router::new()
///     .route("/status", get(handle_status))
///     .route("/config", get(handle_get_config).post(handle_post_config))
///     .route("/auth/challenge", get(handle_challenge));
///
/// picoserve::listen_and_serve(router, &config, &stack, state).await;
/// ```
pub async fn http_task_impl() {
    info!("HTTP server task started (bootstrap — listening stub on :8080)");

    // In the real implementation:
    // 1. Accept TCP connections on port 8080 from the embassy-net stack.
    // 2. Parse HTTP requests with picoserve.
    // 3. Route to handlers.
    // 4. Return JSON responses.

    loop {
        embassy_time::Timer::after(embassy_time::Duration::from_secs(30)).await;
        info!("HTTP server heartbeat");
    }
}
