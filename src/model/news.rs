use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct NewNews {
    pub news_title: Option<String>,
    pub news_description: Option<String>,
    pub news_summary: Option<String>,
    pub news_link: Option<String>,
    pub news_source: Option<String>,
    pub news_pub_date: Option<NaiveDateTime>,
    pub news_image_link: Option<String>,
    pub news_category: Option<String>,
}
