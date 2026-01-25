//! Authentication and session management.
//!
//! Handles user sessions with HMAC-signed cookies. Authentication is optional
//! and enabled by setting the NOTES_PASSWORD environment variable.

use axum_extra::extract::CookieJar;
use chrono::Utc;
use hmac::{Hmac, Mac};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::env;

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

    if parts[1] != expected_sig {
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
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
    let mut result = String::new();

    for chunk in bytes.chunks(3) {
        let mut n: u32 = 0;
        for (i, &byte) in chunk.iter().enumerate() {
            n |= (byte as u32) << (16 - i * 8);
        }

        let chars_to_add = chunk.len() + 1;
        for i in 0..4 {
            if i < chars_to_add {
                result.push(CHARS[((n >> (18 - i * 6)) & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
        }
    }

    result
}

/// Decode a base64 string
pub fn base64_decode(s: &str) -> Option<String> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let s = s.trim_end_matches('=');
    let mut result = Vec::new();

    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;

    for c in s.chars() {
        let val = CHARS.iter().position(|&x| x == c as u8)? as u32;
        buffer = (buffer << 6) | val;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            result.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }

    String::from_utf8(result).ok()
}

/// Encode bytes as hexadecimal
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
