//! HTTP client for communicating with a cube over WiFi.
//!
//! All requests (except `/auth/challenge`) require the `X-Cube-Auth` header
//! containing an HMAC-SHA256 of the challenge nonce, proving PSK knowledge.

use cubemaster_shared::auth::HTTP_AUTH_HEADER;
use cubemaster_shared::protocol::CubeStatus;
use reqwest::Client;
use tracing::{debug, info};

use crate::ble::pairing::compute_http_auth;

/// HTTP client for a specific cube on the network.
pub struct CubeHttpClient {
    /// Base URL, e.g. "http://192.168.1.42:8080"
    base_url: String,
    /// The PSK shared with this cube.
    psk: [u8; 32],
    /// HTTP client instance.
    client: Client,
    /// Current auth token (computed from latest challenge).
    auth_token: Option<String>,
}

impl CubeHttpClient {
    pub fn new(address: &str, psk: [u8; 32]) -> Self {
        let base_url = if address.starts_with("http") {
            address.to_string()
        } else {
            format!("http://{address}")
        };

        Self {
            base_url,
            psk,
            client: Client::new(),
            auth_token: None,
        }
    }

    /// Authenticate with the cube by fetching a challenge and computing HMAC.
    pub async fn authenticate(&mut self) -> Result<(), HttpError> {
        let url = format!("{}/auth/challenge", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| HttpError::Request(e.to_string()))?;

        let challenge: cubemaster_shared::auth::HttpChallenge = resp
            .json()
            .await
            .map_err(|e| HttpError::Deserialize(e.to_string()))?;

        let token = compute_http_auth(&self.psk, &challenge.nonce);
        self.auth_token = Some(token);

        debug!("Authenticated with cube (challenge expires in {}s)", challenge.expires_in);
        Ok(())
    }

    /// Get the cube's status.
    pub async fn get_status(&self) -> Result<CubeStatus, HttpError> {
        let url = format!("{}/status", self.base_url);
        let mut req = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            req = req.header(HTTP_AUTH_HEADER, token);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| HttpError::Request(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(HttpError::Status(resp.status().as_u16()));
        }

        let status: CubeStatus = resp
            .json()
            .await
            .map_err(|e| HttpError::Deserialize(e.to_string()))?;

        Ok(status)
    }

    /// Get the cube's configuration (name, etc.).
    pub async fn get_config(&self) -> Result<CubeConfigResponse, HttpError> {
        let url = format!("{}/config", self.base_url);
        let mut req = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            req = req.header(HTTP_AUTH_HEADER, token);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| HttpError::Request(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(HttpError::Status(resp.status().as_u16()));
        }

        resp.json()
            .await
            .map_err(|e| HttpError::Deserialize(e.to_string()))
    }

    /// Update the cube's name.
    pub async fn set_name(&self, new_name: &str) -> Result<(), HttpError> {
        let url = format!("{}/config", self.base_url);
        let mut req = self.client.post(&url).json(&serde_json::json!({
            "name": new_name,
        }));

        if let Some(ref token) = self.auth_token {
            req = req.header(HTTP_AUTH_HEADER, token);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| HttpError::Request(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(HttpError::Status(resp.status().as_u16()));
        }

        info!("Cube renamed to: {}", new_name);
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CubeConfigResponse {
    pub name: String,
    pub device_id: String,
    pub firmware_version: String,
}

#[derive(Debug, Clone)]
pub enum HttpError {
    Request(String),
    Deserialize(String),
    Status(u16),
    Auth(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(e) => write!(f, "HTTP request error: {e}"),
            Self::Deserialize(e) => write!(f, "HTTP response parse error: {e}"),
            Self::Status(code) => write!(f, "HTTP error status: {code}"),
            Self::Auth(e) => write!(f, "HTTP auth error: {e}"),
        }
    }
}
