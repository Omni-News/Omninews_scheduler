use sqlx::{query, query_as, MySqlPool};

use crate::db_util::get_db;
use crate::model::rss::{NewRssChannel, RssChannel};

pub async fn select_all_rss_channels(pool: &MySqlPool) -> Result<Vec<RssChannel>, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query_as!(RssChannel, "SELECT * FROM rss_channel")
        .fetch_all(&mut *conn)
        .await;

    match result {
        Ok(res) => Ok(res),
        Err(e) => Err(e),
    }
}

pub async fn select_all_rss_links(pool: &MySqlPool) -> Result<Vec<String>, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query!("SELECT channel_rss_link FROM rss_channel")
        .fetch_all(&mut *conn)
        .await;

    match result {
        Ok(res) => Ok(res
            .into_iter()
            .map(|r| r.channel_rss_link.unwrap_or_default())
            .collect()),
        Err(e) => Err(e),
    }
}

pub async fn update_rss_channel(
    pool: &MySqlPool,
    rss_channel: &NewRssChannel,
) -> Result<bool, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query!(
        "UPDATE rss_channel
        SET channel_title = ?, channel_description = ?, channel_link = ?, channel_image_url = ?, channel_language = ?, rss_generator = ?, channel_rank = ? 
        WHERE channel_rss_link = ?;",
        rss_channel.channel_title,
        rss_channel.channel_description,
        rss_channel.channel_link,
        rss_channel.channel_image_url,
        rss_channel.channel_language,
        rss_channel.rss_generator,
        rss_channel.channel_rank,
        rss_channel.channel_rss_link,
    )
    .execute(&mut *conn)
    .await?;

    if result.rows_affected() > 0 {
        Ok(true)
    } else {
        Ok(false)
    }
}
