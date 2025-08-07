use sqlx::{query, MySqlPool};

use crate::{db_util::get_db, model::rss::NewRssItem};

pub async fn is_exist_rss_item_by_link(
    pool: &MySqlPool,
    item_link: &str,
) -> Result<bool, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query!(
        "SELECT rss_item.rss_title FROM rss_item WHERE rss_link=?;",
        item_link,
    )
    .fetch_one(&mut *conn)
    .await;

    match result {
        Ok(_) => Ok(true),
        Err(e) => Err(e),
    }
}

pub async fn insert_rss_item(pool: &MySqlPool, rss_item: NewRssItem) -> Result<i32, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query!(
        "INSERT INTO rss_item 
            (channel_id, rss_title, rss_description, rss_link, rss_author, rss_pub_date, rss_rank, rss_image_link)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        rss_item.channel_id,
        rss_item.rss_title,
        rss_item.rss_description,
        rss_item.rss_link,
        rss_item.rss_author,
        rss_item.rss_pub_date,
        rss_item.rss_rank,
        rss_item.rss_image_link,
    )
    .execute(&mut *conn)
    .await;

    match result {
        Ok(res) => Ok(res.last_insert_id() as i32),
        Err(e) => Err(e),
    }
}

pub async fn select_rss_items_len_by_channel_id(
    pool: &MySqlPool,
    channel_id: i32,
) -> Result<i32, sqlx::Error> {
    let mut conn = get_db(pool).await?;
    let result = query!(
        "SELECT COUNT(*) as count FROM rss_item WHERE channel_id = ?;",
        channel_id,
    )
    .fetch_one(&mut *conn)
    .await;

    match result {
        Ok(res) => Ok(res.count as i32),
        Err(e) => Err(e),
    }
}
