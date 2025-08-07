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
