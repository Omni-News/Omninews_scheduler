// 10분마다 내 DB에 등록된 RSS들을 가져와서 추가로 등록된 글 있는지 검증 후, 추가로 등록되었다면,
// 내 DB에 추가하고, 그 Rss를 구독하고 있던 사용자들에게 알림을 보낸다.

/*
* 1. DB에서 RSS채널 목록 가져오기.
* 2. 해당 RSS 채널들 스크래핑하기.
*   2-1. 가장 최신 글부터,내 DB에 없다면 글 추가하기.
*   2-2. 만약 내 DB에 있는 글이라면, 그때부터 추가 X
* 3. 추가할 때마다, 해당 Rss채널을 구독하고 있는 사용자에게 알림 보냄.
*/

use std::time::Duration;

use rss::Item;
use sqlx::MySqlPool;
use tokio::time::{interval_at, Instant};

use crate::{
    model::{error::OmniNewsError, fcm_token::FcmTokenUser, rss::RssChannel},
    rss_fetch_and_notification_error, rss_fetch_and_notification_info,
    service::{
        rss::{
            channel_service::{get_all_rss_channels, parse_rss_link_to_channel},
            item_service::{self, create_rss_item_and_embedding},
        },
        user_service,
    },
    utils::{embedding_util::EmbeddingService, firebase::send_fcm::send_fcm_message},
};

pub async fn rss_fetch_and_notification_scheduler(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
) {
    // loop for 10 minutes
    let mut interval = interval_at(Instant::now(), Duration::from_secs(60 * 10));

    loop {
        interval.tick().await;
        rss_fetch_and_notification_info!("[Scheduler] Rss Notification Scheduler started");
        let channels = get_all_rss_channels(pool).await.unwrap();

        for channel in channels {
            let channel_id = channel.channel_id.unwrap_or_default();

            let items_len = item_service::get_items_len_by_channel_id(pool, channel_id)
                .await
                .unwrap();

            for index in 0..items_len {
                let mut item =
                    match get_rss_item_by_channel_from_scraping(index, channel.clone()).await {
                        Ok(item) => item,
                        Err(e) => {
                            rss_fetch_and_notification_error!(
                                "[Scheduler] Failed to get rss item from scraping: {}",
                                e
                            );
                            continue;
                        }
                    };

                // 이미 있는 글임
                if let Ok(res) = item_service::is_exist_rss_item_by_link(
                    pool,
                    item.link.clone().unwrap_or_default(),
                )
                .await
                {
                    if res {
                        break;
                    }
                }

                match create_rss_item_and_embedding(
                    pool,
                    embedding_service,
                    channel_id,
                    channel.channel_image_url.clone().unwrap_or_default(),
                    &mut item,
                )
                .await
                {
                    Ok(_) => {
                        rss_fetch_and_notification_info!(
                            "[Scheduler] Rss Item Created. channel: {}, rss item: {}",
                            channel.channel_title.clone().unwrap_or_default(),
                            item.title.clone().unwrap_or_default()
                        );
                    }
                    Err(e) => {
                        rss_fetch_and_notification_info!(
                            "[Scheduler] Rss Item Already Exists: {}",
                            item.title.clone().unwrap_or_default()
                        );
                        rss_fetch_and_notification_error!(
                            "[Scheduler] Failed to create rss item: {}",
                            e
                        );
                        continue;
                    }
                }

                // Rss채널 구독한 사람들 토큰 가져와서 뿌리기
                let users_tokens =
                    user_service::get_users_fcm_token_subscribed_channel_by_channel_id(
                        pool, channel_id,
                    )
                    .await
                    .unwrap();

                send_notification_each_user(users_tokens, channel.clone(), &mut item)
                    .await
                    .unwrap_or_else(|e| {
                        rss_fetch_and_notification_error!(
                            "[Scheduler] Failed to send notification: {}",
                            e
                        );
                    });
            }
        }
        rss_fetch_and_notification_info!("[Scheduler] Rss Notification Scheduler Ended");
    }
}

pub async fn get_rss_item_by_channel_from_scraping(
    index: i32,
    channel: RssChannel,
) -> Result<Item, OmniNewsError> {
    let link = channel.channel_rss_link.clone().unwrap_or_default();

    let mut channel = parse_rss_link_to_channel(&link).await?;

    let item = channel
        .items_mut()
        .get(index as usize)
        .cloned()
        .unwrap_or_default();

    // TODO rss item의 title이  Some("")일 수 있음. 문제 발생 시 추적 및 조치
    Ok(item)
}

pub async fn send_notification_each_user(
    tokens: Vec<FcmTokenUser>,
    channel: RssChannel,
    item: &mut Item,
) -> Result<(), OmniNewsError> {
    for token in tokens {
        send_fcm_message(
            token,
            format!(
                "{:?}의 새로운 RSS",
                channel.channel_title.clone().unwrap_or_default().as_str()
            ),
            format!("{:?}", item.title.clone().unwrap_or_default().as_str()),
        )
        .await
        .map_err(|_| OmniNewsError::FirebaseError)?;
    }
    Ok(())
}
