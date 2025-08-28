use sqlx::{query, query_as, MySqlPool};

use crate::db_util::get_db;
use crate::model::rss::{NewRssChannel, RssChannel};

pub async fn select_channel_by_id(
    pool: &MySqlPool,
    channel_id: i32,
) -> Result<RssChannel, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query_as!(
        RssChannel,
        "SELECT * FROM rss_channel WHERE channel_id = ?",
        channel_id
    )
    .fetch_one(&mut *conn)
    .await;

    match result {
        Ok(res) => Ok(res),
        Err(e) => Err(e),
    }
}

pub async fn select_channel_id_by_rss_link(
    pool: &MySqlPool,
    rss_link: &str,
) -> Result<i32, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query!(
        "SELECT channel_id FROM rss_channel WHERE channel_rss_link = ?",
        rss_link
    )
    .fetch_one(&mut *conn)
    .await;

    match result {
        Ok(res) => Ok(res.channel_id),
        Err(e) => Err(e),
    }
}

pub async fn select_channel_id_by_channel_link(
    pool: &MySqlPool,
    channel_link: &str,
) -> Result<i32, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query!(
        "SELECT channel_id FROM rss_channel WHERE channel_link = ?",
        channel_link
    )
    .fetch_one(&mut *conn)
    .await;

    match result {
        Ok(res) => Ok(res.channel_id),
        Err(e) => Err(e),
    }
}

// 1. default (using other engine) // rss_generator not like '%Omninews%'
// 2. omninews (using webdriver) // rss_generator like 'Omninews%' and not like 'Omninews_css'
// 3. omninews_css (using webdriver, css) // rss_generator like 'Omninews_css'
pub async fn select_default_rss_channels(pool: &MySqlPool) -> Result<Vec<RssChannel>, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query_as!(
        RssChannel,
        "SELECT * FROM rss_channel WHERE rss_generator not like '%Omninews%'"
    )
    .fetch_all(&mut *conn)
    .await;

    match result {
        Ok(res) => Ok(res),
        Err(e) => Err(e),
    }
}

pub async fn select_default_rss_links(pool: &MySqlPool) -> Result<Vec<String>, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query!(
        "SELECT channel_rss_link FROM rss_channel WHERE rss_generator not like '%Omninews%'"
    )
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

pub async fn update_rss_channel_by_id(
    pool: &MySqlPool,
    rss_channel: &NewRssChannel,
    channel_id: i32,
) -> Result<bool, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query!(
        "UPDATE rss_channel
        SET channel_title = ?, channel_description = ?, channel_link = ?, channel_image_url = ?, channel_language = ?, rss_generator = ?, channel_rank = ? 
        WHERE channel_id = ?;",
        rss_channel.channel_title,
        rss_channel.channel_description,
        rss_channel.channel_link,
        rss_channel.channel_image_url,
        rss_channel.channel_language,
        rss_channel.rss_generator,
        rss_channel.channel_rank,
        channel_id,
    )
    .execute(&mut *conn)
    .await?;

    if result.rows_affected() > 0 {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn select_rss_channel_with_webdriver(
    pool: &MySqlPool,
) -> Result<Vec<RssChannel>, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query_as!(
    RssChannel,
        "SELECT * FROM rss_channel WHERE rss_generator like 'Omninews%' and rss_generator not like 'Omninews_css'"
)
    .fetch_all(&mut *conn)
    .await;
    match result {
        Ok(res) => Ok(res),
        Err(e) => Err(e),
    }
}
