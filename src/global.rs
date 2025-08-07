use std::sync::Mutex;

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub struct FcmAccessToken {
    pub access_token: String,
    pub expires_at: DateTime<Utc>, // Unix timestamp in seconds
}

lazy_static! {
    pub static ref FETCH_FLAG: Mutex<bool> = Mutex::new(true);
    pub static ref FCM_ACCESS_TOKEN: Mutex<Option<FcmAccessToken>> = Mutex::new(None);
}
