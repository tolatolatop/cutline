pub mod constants;
pub mod auth;
pub mod a_bogus;
pub mod client;
pub mod api;

use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
