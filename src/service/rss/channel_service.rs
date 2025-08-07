use rss::Channel;
use sqlx::MySqlPool;

use crate::{
    model::{
        error::OmniNewsError,
        rss::{NewRssChannel, RssChannel},
    },
    repository::rss_channel_repository,
    rss_fetch_and_notification_error, rss_info_error,
};

pub async fn parse_rss_link_to_channel(link: &str) -> Result<Channel, OmniNewsError> {
    let response = reqwest::get(link).await.map_err(|e| {
        rss_fetch_and_notification_error!("[Service] Not found url : {}", link);
        OmniNewsError::Request(e)
    })?;
    let body = response.text().await.map_err(OmniNewsError::Request)?;
    Channel::read_from(body.as_bytes()).map_err(|e| {
        rss_fetch_and_notification_error!("[Service] Failed to read from rss body: {:?}", e);
        OmniNewsError::Parse
    })
}

fn make_rss_channel(channel: Channel, rss_link: String) -> NewRssChannel {
    NewRssChannel::new(
        channel.title().to_string(),
        channel.link().to_string(),
        channel.description().to_string(),
        channel.image().map(|e| e.url().to_string()),
        channel.language().unwrap_or("None").to_string(),
        channel.generator().unwrap_or("None").to_string(),
        0,
        rss_link,
    )
}

pub async fn get_all_rss_channels(pool: &MySqlPool) -> Result<Vec<RssChannel>, OmniNewsError> {
    match rss_channel_repository::select_all_rss_channels(pool).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_fetch_and_notification_error!(
                "[Service] Failed to select all rss channels: {:?}",
                e
            );
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn get_all_rss_links(pool: &MySqlPool) -> Result<Vec<String>, OmniNewsError> {
    match rss_channel_repository::select_all_rss_links(pool).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_info_error!("[Service] Failed to select all rss links: {:?}", e);
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn get_rss_info(rss_link: &str) -> Result<NewRssChannel, OmniNewsError> {
    let channel = parse_rss_link_to_channel(rss_link).await?;
    let new_channel = make_rss_channel(channel, rss_link.to_string());

    Ok(new_channel)
}

pub async fn update_rss_channel(
    pool: &MySqlPool,
    rss_channel: &NewRssChannel,
) -> Result<bool, OmniNewsError> {
    match rss_channel_repository::update_rss_channel(pool, rss_channel).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_info_error!("[Service] Failed to update rss info: {:?}", e);
            Err(OmniNewsError::Database(e))
        }
    }
}
