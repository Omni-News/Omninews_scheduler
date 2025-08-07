use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NewEmbedding {
    pub embedding_value: Option<Vec<u8>>,
    pub channel_id: Option<i32>,
    pub rss_id: Option<i32>,
    pub news_id: Option<i32>,
    pub embedding_source_rank: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Embedding {
    pub embedding_id: Option<i32>,
    pub embedding_value: Option<Vec<u8>>,
    pub channel_id: Option<i32>,
    pub rss_id: Option<i32>,
    pub news_id: Option<i32>,
    pub embedding_source_rank: Option<i32>,
}
