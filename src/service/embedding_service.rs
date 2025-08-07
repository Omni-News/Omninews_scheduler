use sqlx::MySqlPool;

use crate::{
    model::{
        embedding::{Embedding, NewEmbedding},
        error::OmniNewsError,
        rss::NewticleType,
    },
    repository::embedding_repository,
    rss_fetch_and_notification_error,
    utils::embedding_util::{embedding_sentence, encode_embedding, EmbeddingService},
};

pub async fn create_embedding(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    sentence: String,
    mut embedding: NewEmbedding,
) -> Result<i32, OmniNewsError> {
    let embedding_value = embedding_sentence(embedding_service, sentence).await?;
    let encoded_embedding_value = encode_embedding(&embedding_value);

    embedding.embedding_value = Some(encoded_embedding_value);

    match embedding_repository::insert_embedding(pool, embedding).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_fetch_and_notification_error!("[Service] Failed to insert embedding: {}", e);
            Err(OmniNewsError::EmbeddingError)
        }
    }
}

pub async fn find_all_embeddings_by(
    pool: &MySqlPool,
    category: NewticleType,
) -> Result<Vec<Embedding>, OmniNewsError> {
    match category {
        NewticleType::Channel => {
            match embedding_repository::select_all_channel_embeddings(pool).await {
                Ok(embeddings) => Ok(embeddings),
                Err(e) => {
                    rss_fetch_and_notification_error!(
                        "[Service] Failed to select all channel embeddings: {}",
                        e
                    );
                    Err(OmniNewsError::Database(e))
                }
            }
        }
        NewticleType::Rss => match embedding_repository::select_all_rss_embeddings(pool).await {
            Ok(embeddings) => Ok(embeddings),
            Err(e) => {
                rss_fetch_and_notification_error!(
                    "[Service] Failed to select all rss embeddings: {}",
                    e
                );
                Err(OmniNewsError::Database(e))
            }
        },
        NewticleType::News => match embedding_repository::select_all_news_embeddings(pool).await {
            Ok(embeddings) => Ok(embeddings),
            Err(e) => {
                rss_fetch_and_notification_error!(
                    "[Service] Failed to select all news embeddings: {}",
                    e
                );
                Err(OmniNewsError::Database(e))
            }
        },
    }
}
