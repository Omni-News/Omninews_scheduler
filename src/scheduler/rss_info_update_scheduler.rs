use std::time::Duration;

use sqlx::MySqlPool;
use tokio::time::{interval_at, Instant};

use crate::{
    config::webdriver::DriverPool,
    model::error::OmniNewsError,
    rss_info_error, rss_info_info,
    scheduler::site::{default, instagram},
    service::rss::channel_service,
    utils::embedding_util::EmbeddingService,
};

pub async fn rss_info_update_scheduler(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    driver_pool: &DriverPool,
) {
    // 1 day
    let mut interval = interval_at(Instant::now(), Duration::from_secs(60 * 60 * 24));

    loop {
        interval.tick().await;
        rss_info_info!("[Scheduler] Rss Information Update Scheduler started");

        /*
         * 1. DB에서 보관중인 RSS 링크 가져오기.
         * 2. 해당 링크 통해 아래 정보 스크래이핑.
         *
         *            pub channel_title: Option<String>,
         *            pub channel_link: Option<String>,
         *            pub channel_description: Option<String>,
         *            pub channel_image_url: Option<String>,
         *            pub channel_language: Option<String>,
         *            pub rss_generator: Option<String>,
         *            pub channel_rank: Option<i32>,
         *            pub channel_rss_link: Option<String>
         * 3. DB에 내용 업데이트.
         *
         */
        let _ = update_channel_info_default(pool, embedding_service).await;
        let _ = update_channel_info_webdriver(pool, embedding_service, driver_pool).await;
        rss_info_info!("[Scheduler] Rss Information Update Scheduler ended");
    }
}

async fn update_channel_info_default(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
) -> Result<bool, OmniNewsError> {
    let rss_links = channel_service::get_default_rss_links(pool).await?;
    for rss_link in &rss_links {
        let update_rss = match channel_service::get_rss_channel_by_parse(rss_link).await {
            Ok(channel) => channel,
            Err(e) => {
                rss_info_error!("[Scheduler] Failed to get RSS info for {}: {}", rss_link, e);
                continue;
            }
        };
        let channel_id = channel_service::get_channel_id_by_rss_link(pool, rss_link).await?;
        match channel_service::update_rss_channel_and_embedding(
            pool,
            embedding_service,
            &update_rss,
            channel_id,
        )
        .await
        {
            Ok(_) => rss_info_info!(
                "[Scheduler] Rss Information Update Scheduler updated: {}",
                rss_link
            ),
            Err(e) => rss_info_error!(
                "[Scheduler] Failed Rss Information Update Scheduler: {}: {}",
                rss_link,
                e
            ),
        };
    }
    Ok(true)
}

// TODO: instagram info update, default info update 검증 필요
// ex) instagram, default, etc.
async fn update_channel_info_webdriver(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    driver_pool: &DriverPool,
) -> Result<bool, OmniNewsError> {
    let channels = channel_service::get_rss_channels_with_webdriver(pool).await?;

    for channel in channels {
        let platform = channel
            .rss_generator
            .clone()
            .unwrap_or_default()
            .split('_')
            .next_back()
            .unwrap_or("")
            .to_string();

        match platform.as_str() {
            "instagram" => {
                let _ = instagram::update_instagram_channel_info(
                    pool,
                    embedding_service,
                    driver_pool,
                    &channel,
                )
                .await?;
            }
            "default" => {
                let _ = default::update_default_channel_info(
                    pool,
                    embedding_service,
                    driver_pool,
                    &channel.channel_link.unwrap_or_default(),
                )
                .await?;
            }
            _ => {}
        }
    }

    Ok(true)
}
