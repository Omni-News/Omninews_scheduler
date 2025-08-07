use sqlx::{query, MySqlPool};

use crate::{db_util::get_db, model::news::NewNews};

pub async fn select_news_by_title(
    pool: &MySqlPool,
    news_title: String,
) -> Result<Option<i32>, sqlx::Error> {
    let mut conn = get_db(pool).await?;

    let result = query!(
        r#"
        SELECT news_id FROM news WHERE news_title = ?
        "#,
        news_title,
    )
    .fetch_one(&mut *conn)
    .await;

    match result {
        Ok(res) => Ok(Some(res.news_id)),
        Err(e) => Err(e),
    }
}

pub async fn insert_news(pool: &MySqlPool, news: NewNews) -> Result<i32, sqlx::Error> {
    let mut conn = get_db(pool).await?;

    let result = query!(
        r#"
        INSERT INTO news (news_title, news_description, news_summary, news_link, news_source, news_pub_date, news_image_link, news_category)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        news.news_title,
        news.news_description,
        news.news_summary,
        news.news_link,
        news.news_source,
        news.news_pub_date,
        news.news_image_link,
        news.news_category,
    ).execute(&mut *conn).await;

    match result {
        Ok(res) => Ok(res.last_insert_id() as i32),
        Err(e) => Err(e),
    }
}

pub async fn delete_old_news(pool: &MySqlPool) -> Result<i32, sqlx::Error> {
    let mut conn = get_db(pool).await?;

    let result = query!(
        r#"
        DELETE FROM news WHERE news_pub_date < DATE_SUB(CURDATE(), INTERVAL 7 DAY)
        "#
    )
    .execute(&mut *conn)
    .await;

    match result {
        Ok(res) => Ok(res.rows_affected() as i32),
        Err(e) => Err(e),
    }
}
