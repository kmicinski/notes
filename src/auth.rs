//! Authentication and session management.
//!
//! Uses Argon2id for password hashing and sled for server-side sessions.
//! Authentication is optional and enabled by setting the NOTES_PASSWORD
//! environment variable.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::env;

/// Session cookie name
pub const SESSION_COOKIE: &str = "notes_session";

/// Session time-to-live in hours
pub const SESSION_TTL_HOURS: i64 = 24;

/// CSRF token time-to-live in seconds
const CSRF_TTL_SECS: i64 = 600; // 10 minutes

// ============================================================================
// Session Structure (stored in sled)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionData {
    created: i64,
    expires: i64,
}

// ============================================================================
// Password Hashing
// ============================================================================

/// Hash the NOTES_PASSWORD at startup using Argon2id.
/// Returns None if NOTES_PASSWORD is not set.
pub fn hash_password_at_startup() -> Option<String> {
    let password = env::var("NOTES_PASSWORD").ok()?;
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password at startup");
    Some(hash.to_string())
}

/// Verify a password attempt against the stored Argon2 hash.
pub fn verify_password(attempt: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(attempt.as_bytes(), &parsed_hash)
        .is_ok()
}

// ============================================================================
// Authentication Check
// ============================================================================

/// Check if proxy-level auth is trusted (e.g., behind Authelia).
/// When TRUST_PROXY_AUTH is set, all requests are treated as authenticated.
fn trust_proxy_auth() -> bool {
    env::var("TRUST_PROXY_AUTH").is_ok()
}

/// Check if authentication is enabled
pub fn is_auth_enabled() -> bool {
    trust_proxy_auth() || env::var("NOTES_PASSWORD").is_ok()
}

/// Check if the user is logged in via cookie (server-side session lookup).
pub fn is_logged_in(jar: &CookieJar, db: &sled::Db) -> bool {
    if trust_proxy_auth() {
        return true;
    }

    if !is_auth_enabled() {
        return false;
    }

    match jar.get(SESSION_COOKIE) {
        Some(cookie) => verify_session(cookie.value(), db),
        None => false,
    }
}

// ============================================================================
// Server-Side Sessions (sled)
// ============================================================================

fn sessions_tree(db: &sled::Db) -> sled::Tree {
    db.open_tree("sessions").expect("Failed to open sessions tree")
}

/// Create a new session, store it in sled, and return the session ID (hex string).
pub fn create_session(db: &sled::Db) -> Option<String> {
    let mut id_bytes = [0u8; 32];
    OsRng.fill(&mut id_bytes);
    let session_id = hex_encode(&id_bytes);

    let now = Utc::now().timestamp();
    let data = SessionData {
        created: now,
        expires: now + (SESSION_TTL_HOURS * 3600),
    };

    let encoded = serde_json::to_vec(&data).ok()?;
    let tree = sessions_tree(db);
    tree.insert(session_id.as_bytes(), encoded).ok()?;

    Some(session_id)
}

/// Verify a session ID exists and is not expired.
pub fn verify_session(session_id: &str, db: &sled::Db) -> bool {
    let tree = sessions_tree(db);
    match tree.get(session_id.as_bytes()) {
        Ok(Some(data)) => {
            if let Ok(session) = serde_json::from_slice::<SessionData>(&data) {
                if Utc::now().timestamp() < session.expires {
                    return true;
                }
                // Expired — clean up
                let _ = tree.remove(session_id.as_bytes());
            }
            false
        }
        _ => false,
    }
}

/// Delete a session (server-side revocation for logout).
pub fn delete_session(session_id: &str, db: &sled::Db) {
    let tree = sessions_tree(db);
    let _ = tree.remove(session_id.as_bytes());
}

// ============================================================================
// CSRF Tokens
// ============================================================================

fn csrf_tree(db: &sled::Db) -> sled::Tree {
    db.open_tree("csrf_tokens")
        .expect("Failed to open csrf_tokens tree")
}

/// Create a one-time CSRF token stored in sled with a 10-minute TTL.
pub fn create_csrf_token(db: &sled::Db) -> String {
    let mut token_bytes = [0u8; 32];
    OsRng.fill(&mut token_bytes);
    let token = hex_encode(&token_bytes);

    let expires = Utc::now().timestamp() + CSRF_TTL_SECS;
    let tree = csrf_tree(db);
    let _ = tree.insert(token.as_bytes(), &expires.to_be_bytes());

    token
}

/// Verify and consume a CSRF token (one-time use).
pub fn verify_and_consume_csrf_token(token: &str, db: &sled::Db) -> bool {
    let tree = csrf_tree(db);
    match tree.remove(token.as_bytes()) {
        Ok(Some(data)) => {
            if data.len() == 8 {
                let expires = i64::from_be_bytes(data.as_ref().try_into().unwrap());
                return Utc::now().timestamp() < expires;
            }
            false
        }
        _ => false,
    }
}

// ============================================================================
// Session Cleanup
// ============================================================================

/// Purge expired sessions and CSRF tokens from sled.
/// Called at startup.
pub fn purge_expired_sessions(db: &sled::Db) {
    let now = Utc::now().timestamp();

    // Purge expired sessions
    let tree = sessions_tree(db);
    let mut to_remove = Vec::new();
    for entry in tree.iter() {
        if let Ok((key, value)) = entry {
            if let Ok(session) = serde_json::from_slice::<SessionData>(&value) {
                if now >= session.expires {
                    to_remove.push(key);
                }
            } else {
                // Corrupt entry — remove it
                to_remove.push(key);
            }
        }
    }
    for key in to_remove {
        let _ = tree.remove(key);
    }

    // Purge expired CSRF tokens
    let csrf = csrf_tree(db);
    let mut to_remove = Vec::new();
    for entry in csrf.iter() {
        if let Ok((key, value)) = entry {
            if value.len() == 8 {
                let expires = i64::from_be_bytes(value.as_ref().try_into().unwrap());
                if now >= expires {
                    to_remove.push(key);
                }
            } else {
                to_remove.push(key);
            }
        }
    }
    for key in to_remove {
        let _ = csrf.remove(key);
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Encode bytes as hexadecimal
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
