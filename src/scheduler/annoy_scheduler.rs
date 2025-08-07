use std::time::Duration;

use sqlx::MySqlPool;
use tokio::time::{interval_at, Instant};

use crate::{annoy_error, annoy_info, utils::annoy_util::save_annoy};

pub async fn save_annoy_scheduler(pool: &MySqlPool) {
    // 1 hour
    let mut interval = interval_at(Instant::now(), Duration::from_secs(3600));

    loop {
        interval.tick().await;

        match save_annoy(pool).await {
            Ok(_) => annoy_info!("[Scheduler] Successfully saved annoy"),
            Err(e) => annoy_error!("[Scheduler] Failed to save annoy: {:?}", e),
        };
    }
}
