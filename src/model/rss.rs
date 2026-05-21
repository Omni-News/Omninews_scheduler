use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use rss::Item;

#[derive(Debug, Clone)]
pub enum NewticleType {
    Channel,
    Rss,
    News,
}

#[derive(Debug, Clone)]
pub struct NewRssChannel {
    pub channel_title: Option<String>,
    pub channel_link: Option<String>,
    pub channel_description: Option<String>,
    pub channel_image_url: Option<String>,
    pub channel_language: Option<String>,
    pub rss_generator: Option<String>,
    pub channel_rank: Option<i32>,
    pub channel_rss_link: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RssChannel {
    pub channel_id: Option<i32>,
    pub channel_title: Option<String>,
    pub channel_link: Option<String>,
    pub channel_description: Option<String>,
    pub channel_image_url: Option<String>,
    pub channel_language: Option<String>,
    pub rss_generator: Option<String>,
    pub channel_rank: Option<i32>,
    pub channel_rss_link: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewRssItem {
    pub channel_id: Option<i32>,
    pub rss_title: Option<String>,
    pub rss_description: Option<String>,
    pub rss_link: Option<String>,
    pub rss_author: Option<String>,
    pub rss_pub_date: Option<NaiveDateTime>,
    pub rss_rank: Option<i32>,
    pub rss_image_link: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreatedRssItem {
    pub rss_id: i32,
    pub channel_id: i32,
    pub rss_title: String,
    pub rss_description: String,
    pub rss_link: String,
    pub rss_author: String,
    pub rss_pub_date: Option<NaiveDateTime>,
    pub rss_rank: i32,
    pub rss_image_link: String,
}

#[allow(clippy::too_many_arguments)]
impl NewRssChannel {
    pub fn new(
        channel_title: String,
        channel_link: String,
        channel_description: String,
        channel_image_url: Option<String>,
        channel_language: String,
        rss_generator: String,
        channel_rank: i32,
        channel_rss_link: String,
    ) -> Self {
        Self {
            channel_title: Some(channel_title),
            channel_link: Some(channel_link),
            channel_description: Some(channel_description),
            channel_image_url,
            channel_language: Some(channel_language),
            rss_generator: Some(rss_generator),
            channel_rank: Some(channel_rank),
            channel_rss_link: Some(channel_rss_link),
        }
    }
}

impl NewRssItem {
    pub fn new(
        channel_id: i32,
        item: &Item,
        rss_pub_date: Option<NaiveDateTime>,
        item_image_link: String,
    ) -> Self {
        Self {
            channel_id: Some(channel_id),
            rss_title: Some(
                item.title()
                    .filter(|title| title.len() <= 200)
                    .unwrap_or_default()
                    .to_string(),
            ),
            rss_description: Some(item.description().unwrap_or("None").to_string()),
            rss_link: Some(item.link().unwrap_or("None").to_string()),
            rss_author: Some(item.author().unwrap_or("None").to_string()),
            rss_pub_date,
            rss_rank: Some(0),
            rss_image_link: Some(item_image_link),
        }
    }
}

impl CreatedRssItem {
    pub fn from_new(rss_id: i32, item: NewRssItem) -> Self {
        Self {
            rss_id,
            channel_id: item.channel_id.unwrap_or_default(),
            rss_title: item.rss_title.unwrap_or_default(),
            rss_description: item.rss_description.unwrap_or_default(),
            rss_link: item.rss_link.unwrap_or_default(),
            rss_author: item.rss_author.unwrap_or_default(),
            rss_pub_date: item.rss_pub_date,
            rss_rank: item.rss_rank.unwrap_or_default(),
            rss_image_link: item.rss_image_link.unwrap_or_default(),
        }
    }

    pub fn to_fcm_data(&self) -> BTreeMap<String, String> {
        let mut data = BTreeMap::new();
        data.insert("type".to_string(), "rss_item".to_string());
        data.insert("rss_id".to_string(), self.rss_id.to_string());
        data.insert("channel_id".to_string(), self.channel_id.to_string());
        data.insert("rss_title".to_string(), self.rss_title.clone());
        data.insert("rss_description".to_string(), self.rss_description.clone());
        data.insert("rss_link".to_string(), self.rss_link.clone());
        data.insert("rss_author".to_string(), self.rss_author.clone());
        data.insert(
            "rss_pub_date".to_string(),
            self.rss_pub_date
                .map(|date| date.format("%Y-%m-%dT%H:%M:%S").to_string())
                .unwrap_or_default(),
        );
        data.insert("rss_rank".to_string(), self.rss_rank.to_string());
        data.insert("rss_image_link".to_string(), self.rss_image_link.clone());
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_rss_item_fcm_data_contains_routing_fields() {
        let created = CreatedRssItem {
            rss_id: 42,
            channel_id: 7,
            rss_title: "새 글".to_string(),
            rss_description: "본문 요약".to_string(),
            rss_link: "https://example.com/post".to_string(),
            rss_author: "작성자".to_string(),
            rss_pub_date: Some(
                NaiveDateTime::parse_from_str("2026-05-21T12:34:56", "%Y-%m-%dT%H:%M:%S").unwrap(),
            ),
            rss_rank: 0,
            rss_image_link: "https://example.com/image.png".to_string(),
        };

        let data = created.to_fcm_data();

        assert_eq!(data.get("type"), Some(&"rss_item".to_string()));
        assert_eq!(data.get("rss_id"), Some(&"42".to_string()));
        assert_eq!(data.get("channel_id"), Some(&"7".to_string()));
        assert_eq!(data.get("rss_title"), Some(&"새 글".to_string()));
        assert_eq!(
            data.get("rss_link"),
            Some(&"https://example.com/post".to_string())
        );
        assert_eq!(
            data.get("rss_pub_date"),
            Some(&"2026-05-21T12:34:56".to_string())
        );
    }
}
