//! Timer utilities.

use std::time::{SystemTime, UNIX_EPOCH};

/// Get the current time in seconds since the UNIX epoch.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}