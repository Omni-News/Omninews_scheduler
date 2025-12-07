use crate::{
    model::{embedding::NewEmbedding, error::OmniNewsError, rss::NewRssItem},
    repository::rss_item_repository,
    rss_fetch_and_notification_error, rss_fetch_and_notification_warn,
    service::embedding_service,
    utils::embedding_util::EmbeddingService,
};
use chrono::FixedOffset;
use chrono::{DateTime, NaiveDateTime};
use scraper::{Html, Selector};
use sqlx::MySqlPool;

pub async fn create_rss_item_and_embedding(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    rss_item: NewRssItem,
) -> Result<bool, OmniNewsError> {
    if rss_item.rss_title.is_none() {
        return Err(OmniNewsError::NotFound("rss item".to_string()));
    }

    let mut rss_item = rss_item;
    let description = rss_item
        .rss_description
        .clone()
        .unwrap_or("None".to_string());

    let (extracted_description, item_image_link) =
        extract_html_to_passage_and_image_link(&description);
    rss_item.rss_description = Some(extracted_description.clone());
    let item_image_link = use_channel_url_if_none(
        item_image_link,
        rss_item.rss_image_link.clone().unwrap_or_default(),
    );
    rss_item.rss_image_link = Some(item_image_link);

    let item_id = store_rss_item(pool, rss_item.clone()).await.unwrap();

    let sentence = format!(
        "{}\n{}\n{}",
        rss_item.rss_title.clone().unwrap_or_default(),
        extracted_description,
        rss_item.rss_author.clone().unwrap_or_default()
    );
    let embedding = NewEmbedding {
        embedding_value: None,
        channel_id: None,
        rss_id: Some(item_id),
        news_id: None,
        embedding_source_rank: Some(0),
    };

    embedding_service::create_embedding(pool, embedding_service, sentence, embedding).await?;
    Ok(true)
}

fn extract_html_to_passage_and_image_link(html: &str) -> (String, Option<String>) {
    let document = Html::parse_document(html);

    let passage_selector = Selector::parse("h3, p").unwrap();
    let image_selector = Selector::parse("img").unwrap();

    let extracted_text: Vec<String> = document
        .clone()
        .select(&passage_selector)
        .map(|e| e.text().collect::<Vec<_>>().join(" "))
        .collect();

    let image_link = document
        .select(&image_selector)
        .next()
        .and_then(|link| link.attr("src").map(|s| s.to_string()))
        .filter(|link| link.len() <= 1000);

    (extracted_text.join(" "), image_link)
}

fn use_channel_url_if_none(link: Option<String>, channel_image_url: String) -> String {
    match link {
        Some(link) => link,
        None => channel_image_url,
    }
}

pub fn parse_pub_date(pub_date_str: Option<&str>) -> Option<NaiveDateTime> {
    pub_date_str.and_then(|date_str| {
        if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
            let kst = FixedOffset::east_opt(9 * 3600).unwrap();
            Some(dt.with_timezone(&kst).naive_local())
        } else {
            None
        }
    })
}

async fn store_rss_item(pool: &MySqlPool, mut rss_item: NewRssItem) -> Result<i32, OmniNewsError> {
    let item_link = rss_item.rss_link.clone().unwrap_or_default();

    if let Some(str) = rss_item.rss_description.as_mut() {
        *str = str.chars().take(200).collect()
    };

    match rss_item_repository::is_exist_rss_item_by_link(pool, &item_link).await {
        Ok(_) => {
            rss_fetch_and_notification_warn!(
                "[Service] Item already exists with link: {}",
                item_link
            );
            Err(OmniNewsError::AlreadyExists)
        }

        Err(_) => rss_item_repository::insert_rss_item(pool, rss_item)
            .await
            .map_err(|e| {
                rss_fetch_and_notification_error!(
                    "[Service] Failed to select item by link : {}",
                    e
                );
                OmniNewsError::Database(e)
            }),
    }
}

pub async fn get_items_len_by_channel_id(
    pool: &MySqlPool,
    channel_id: i32,
) -> Result<i32, OmniNewsError> {
    match rss_item_repository::select_rss_items_len_by_channel_id(pool, channel_id).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_fetch_and_notification_error!(
                "[Service] Failed to select rss channel length: {:?}",
                e
            );
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn is_exist_rss_item_by_link(
    pool: &MySqlPool,
    link: &str,
) -> Result<bool, OmniNewsError> {
    match rss_item_repository::is_exist_rss_item_by_link(pool, link).await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
