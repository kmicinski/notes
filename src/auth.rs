//! Authentication and session management.
//!
//! Handles user sessions with HMAC-signed cookies. Authentication is optional
//! and enabled by setting the NOTES_PASSWORD environment variable.

use axum_extra::extract::CookieJar;
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use hmac::{Hmac, Mac};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::env;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Session cookie name
pub const SESSION_COOKIE: &str = "notes_session";

/// Session time-to-live in hours
pub const SESSION_TTL_HOURS: i64 = 24;

// ============================================================================
// Session Structure
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Session {
    created: i64,
    expires: i64,
    nonce: String,
}

// ============================================================================
// Authentication Functions
// ============================================================================

/// Get the secret key from environment (NOTES_PASSWORD)
pub fn get_secret_key() -> Option<Vec<u8>> {
    env::var("NOTES_PASSWORD").ok().map(|p| p.into_bytes())
}

/// Check if authentication is enabled
pub fn is_auth_enabled() -> bool {
    get_secret_key().is_some()
}

/// Create a new session token
pub fn create_session() -> Option<String> {
    let secret = get_secret_key()?;
    let now = Utc::now().timestamp();
    let expires = now + (SESSION_TTL_HOURS * 3600);
    let nonce: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    let session = Session {
        created: now,
        expires,
        nonce,
    };
    let session_json = serde_json::to_string(&session).ok()?;

    let mut mac = HmacSha256::new_from_slice(&secret).ok()?;
    mac.update(session_json.as_bytes());
    let signature = hex_encode(mac.finalize().into_bytes().as_slice());

    Some(format!("{}.{}", base64_encode(&session_json), signature))
}

/// Verify a session token
pub fn verify_session(token: &str, secret: &[u8]) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 2 {
        return false;
    }

    let session_json = match base64_decode(parts[0]) {
        Some(s) => s,
        None => return false,
    };

    // Verify signature
    let mut mac = match HmacSha256::new_from_slice(secret) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(session_json.as_bytes());
    let expected_sig = hex_encode(mac.finalize().into_bytes().as_slice());

    // Constant-time comparison to prevent timing attacks
    let sig_bytes = parts[1].as_bytes();
    let expected_bytes = expected_sig.as_bytes();
    if sig_bytes.len() != expected_bytes.len() {
        return false;
    }
    if sig_bytes.ct_eq(expected_bytes).unwrap_u8() != 1 {
        return false;
    }

    // Check expiration
    let session: Session = match serde_json::from_str(&session_json) {
        Ok(s) => s,
        Err(_) => return false,
    };

    Utc::now().timestamp() < session.expires
}

/// Check if the user is logged in via cookie
pub fn is_logged_in(jar: &CookieJar) -> bool {
    let secret = match get_secret_key() {
        Some(s) => s,
        None => return false,
    };

    match jar.get(SESSION_COOKIE) {
        Some(cookie) => verify_session(cookie.value(), &secret),
        None => false,
    }
}

// ============================================================================
// Encoding Helpers
// ============================================================================

/// Encode a string as base64
pub fn base64_encode(s: &str) -> String {
    STANDARD.encode(s.as_bytes())
}

/// Decode a base64 string
pub fn base64_decode(s: &str) -> Option<String> {
    let bytes = STANDARD.decode(s).ok()?;
    String::from_utf8(bytes).ok()
}

/// Encode bytes as hexadecimal
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
