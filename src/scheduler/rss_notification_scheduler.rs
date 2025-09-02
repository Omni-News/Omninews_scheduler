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
    config::webdriver::DriverPool,
    model::{error::OmniNewsError, fcm_token::FcmTokenUser, rss::NewRssItem},
    rss_fetch_and_notification_error, rss_fetch_and_notification_info,
    rss_fetch_and_notification_warn,
    scheduler::site::{default, instagram},
    service::{
        rss::{
            channel_service::{self, parse_rss_link_to_channel},
            item_service::{self, create_rss_item_and_embedding, parse_pub_date},
        },
        user_service,
    },
    utils::{embedding_util::EmbeddingService, firebase::send_fcm::send_fcm_message},
};
pub async fn rss_fetch_and_notification_scheduler(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    driver_pool: &DriverPool,
) {
    let mut interval = interval_at(Instant::now(), Duration::from_secs(60 * 10));

    loop {
        interval.tick().await;
        rss_fetch_and_notification_info!("[Scheduler] Rss Notification Scheduler started");
        let mut channel_ids: Vec<i32>;
        let mut channel_titles: Vec<String>;
        let mut item_titles: Vec<String>;

        // default
        (channel_ids, channel_titles, item_titles) =
            match fetch_default_rss_and_store(pool, embedding_service).await {
                Ok(res) => res,
                Err(e) => {
                    rss_fetch_and_notification_error!(
                        "[Scheduler] Failed to fetch and store rss: {}",
                        e
                    );
                    continue;
                }
            };

        // instagram or default using webdriver
        if let Ok((ci, ct, it)) =
            fetch_webdriver_rss_and_store(pool, embedding_service, driver_pool).await
        {
            channel_ids.extend(ci);
            channel_titles.extend(ct);
            item_titles.extend(it);
        }

        let _: () = send_notification_each_user(pool, channel_ids, channel_titles, item_titles)
            .await
            .unwrap_or_else(|e| {
                rss_fetch_and_notification_error!(
                    "[Scheduler] Failed to send notification to each user: {}",
                    e
                );
            });
    }
}

//TODO:  이제 이게 default고, webdriver사용하는 것 만들기
pub async fn fetch_default_rss_and_store(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
) -> Result<(Vec<i32>, Vec<String>, Vec<String>), OmniNewsError> {
    // loop for 10 minutes
    let mut channel_ids: Vec<i32> = Vec::new();
    let mut channel_titles: Vec<String> = Vec::new();
    let mut item_titles: Vec<String> = Vec::new();

    let rss_channels = channel_service::get_default_rss_channels(pool)
        .await
        .unwrap();
    for rss_channel in rss_channels {
        let channel_id = rss_channel.channel_id.unwrap_or_default();
        let rss_link = &rss_channel.channel_rss_link.unwrap_or_default();
        let channel_title = &rss_channel.channel_title.unwrap_or_default();
        let channel_image_url = &rss_channel.channel_image_url.unwrap_or_default();

        let items_len_in_db = item_service::get_items_len_by_channel_id(pool, channel_id)
            .await
            .unwrap();
        let raw_items = match get_rss_items_by_channel_crawl(rss_link).await {
            Ok(items) => items,
            Err(e) => {
                rss_fetch_and_notification_error!(
                    "[Scheduler] Failed to get rss items by channel: {}",
                    e
                );
                continue;
            }
        };

        for index in 0..items_len_in_db {
            // xml파일의 items중 index순으로 가져옴.
            let item = raw_items.get(index as usize).cloned().unwrap_or_default();

            // item link기준으로 db에 존재하는지 확인.
            if let Ok(res) = item_service::is_exist_rss_item_by_link(
                pool,
                &item.link.clone().unwrap_or_default(),
            )
            .await
            {
                if res {
                    break;
                }
            }

            let rss_pub_date = parse_pub_date(item.pub_date());
            let rss_item = NewRssItem {
                channel_id: Some(channel_id),
                rss_link: item.link.clone(),
                rss_title: item.title.clone(),
                rss_description: item.description.clone(),
                rss_pub_date,
                rss_author: item.author.clone(),
                rss_rank: Some(0),
                rss_image_link: Some(channel_image_url.to_string()),
            };
            match create_rss_item_and_embedding(pool, embedding_service, rss_item).await {
                Ok(_) => {
                    let item_title = item.title.clone().unwrap_or_default();

                    channel_ids.push(channel_id);
                    channel_titles.push(channel_title.to_string());
                    item_titles.push(item_title.clone());

                    rss_fetch_and_notification_info!(
                        "[Scheduler] Rss Item Created. channel id: {channel_id}, rss item: {item_title}"
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
        }
    }
    Ok((channel_ids, channel_titles, item_titles))
}

async fn fetch_webdriver_rss_and_store(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    driver_pool: &DriverPool,
) -> Result<(Vec<i32>, Vec<String>, Vec<String>), OmniNewsError> {
    // loop for 10 minutes
    let mut channel_ids: Vec<i32> = Vec::new();
    let mut channel_titles: Vec<String> = Vec::new();
    let mut item_titles: Vec<String> = Vec::new();

    let rss_channels = channel_service::get_rss_channels_with_webdriver(pool).await?;
    info!("channels: {:?}", rss_channels);
    for rss_channel in rss_channels {
        let channel_id = rss_channel.channel_id.unwrap_or_default();
        let channel_title = rss_channel.channel_title.unwrap_or_default();
        let channel_link = rss_channel.channel_link.unwrap_or_default();

        let platform = &rss_channel
            .rss_generator
            .unwrap_or_default()
            .split('_')
            .next_back()
            .unwrap_or("")
            .to_string();

        let item_title = match platform.as_str() {
            "instagram" => {
                instagram::fetch_instagram_rss_and_store(
                    pool,
                    embedding_service,
                    driver_pool,
                    &channel_link,
                    channel_id,
                )
                .await?
            }
            "default" => {
                default::fetch_default_rss_and_store(
                    pool,
                    embedding_service,
                    driver_pool,
                    &channel_link,
                    channel_id,
                )
                .await?
            }
            // TODO: css 만들어야됨.
            _ => {
                rss_fetch_and_notification_warn!("[Scheduler] Unsupported platform: {}", platform);
                continue;
            }
        };
        for i in 0..item_title.len() {
            channel_ids.push(channel_id);
            channel_titles.push(channel_title.clone());
            item_titles.push(item_title.get(i).cloned().unwrap_or_default());
        }
    }
    Ok((channel_ids, channel_titles, item_titles))
}

pub async fn get_rss_items_by_channel_crawl(rss_link: &str) -> Result<Vec<Item>, OmniNewsError> {
    let mut channel = parse_rss_link_to_channel(rss_link).await?;
    let item = channel.items_mut();
    // TODO:  rss item의 title이  Some("")일 수 있음. 문제 발생 시 추적 및 조치
    Ok(item.to_vec())
}

async fn send_notification_each_user(
    pool: &MySqlPool,
    channel_ids: Vec<i32>,
    channel_titles: Vec<String>,
    item_titles: Vec<String>,
) -> Result<(), OmniNewsError> {
    for ((channel_id, channel_title), item_title) in
        (channel_ids.iter().zip(channel_titles)).zip(item_titles)
    {
        // Rss채널 구독한 사람들 토큰 가져와서 뿌리기
        let users_tokens =
            user_service::get_users_fcm_token_subscribed_channel_by_channel_id(pool, *channel_id)
                .await
                .unwrap();

        send_notification_each_token(users_tokens, &channel_title, &item_title)
            .await
            .unwrap_or_else(|e| {
                rss_fetch_and_notification_error!("[Scheduler] Failed to send notification: {}", e);
            });
    }
    rss_fetch_and_notification_info!("[Scheduler] Rss Notification Scheduler Ended");
    Ok(())
}
pub async fn send_notification_each_token(
    tokens: Vec<FcmTokenUser>,
    channel_title: &str,
    item_title: &str,
) -> Result<(), OmniNewsError> {
    for token in tokens {
        send_fcm_message(
            token,
            format!("{channel_title}의 새로운 RSS"),
            format!("{item_title}."),
        )
        .await
        .map_err(|_| OmniNewsError::FirebaseError)?;
    }
    Ok(())
}
