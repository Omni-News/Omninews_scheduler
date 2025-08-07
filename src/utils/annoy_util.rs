use std::path::PathBuf;

use sqlx::MySqlPool;

use crate::{
    annoy_info,
    model::{embedding::Embedding, error::OmniNewsError, rss::NewticleType},
    service::embedding_service,
    utils::embedding_util::decode_embedding,
};

pub async fn save_annoy(pool: &MySqlPool) -> Result<(), OmniNewsError> {
    let embeddings_channel =
        embedding_service::find_all_embeddings_by(pool, NewticleType::Channel).await?;
    let embeddings_rss = embedding_service::find_all_embeddings_by(pool, NewticleType::Rss).await?;
    let embeddings_news =
        embedding_service::find_all_embeddings_by(pool, NewticleType::News).await?;
    save_channel_annoy(embeddings_channel).await?;
    save_rss_annoy(embeddings_rss).await?;
    save_news_annoy(embeddings_news).await?;

    Ok(())
}

async fn save_channel_annoy(embeddings: Vec<Embedding>) -> Result<(), OmniNewsError> {
    if embeddings.is_empty() {
        annoy_info!("[Service] No embeddings found for channel.");
        return Ok(());
    }
    let embedding_dim = embeddings[0].embedding_value.as_ref().unwrap().len();
    annoy_info!(
        "[Service] Rss Channel Embedding dimension: {}",
        embedding_dim
    );

    let annoy = rannoy::Rannoy::new(384);
    annoy.set_seed(123);

    for embedding in embeddings.iter() {
        let decoded_embedding = decode_embedding(embedding.embedding_value.as_ref().unwrap());
        annoy.add_item(embedding.embedding_id.unwrap(), decoded_embedding.as_ref());
    }
    // 트리 개수 증가: 40 -> 100
    annoy.build(100);
    annoy.save(PathBuf::from("../resources/channel_embeddings.ann"));

    Ok(())
}

async fn save_rss_annoy(embeddings: Vec<Embedding>) -> Result<(), OmniNewsError> {
    if embeddings.is_empty() {
        annoy_info!("[Service] No embeddings found for rss.");
        return Ok(());
    }
    let embedding_dim = embeddings[0].embedding_value.as_ref().unwrap().len();
    annoy_info!("[Service] Rss Item Embedding dimension: {}", embedding_dim);

    let annoy = rannoy::Rannoy::new(384);
    annoy.set_seed(123);

    for embedding in embeddings.iter() {
        let decoded_embedding = decode_embedding(embedding.embedding_value.as_ref().unwrap());
        annoy.add_item(embedding.embedding_id.unwrap(), decoded_embedding.as_ref());
    }
    // 트리 개수 증가
    annoy.build(100);
    annoy.save(PathBuf::from("../resources/rss_embeddings.ann"));

    Ok(())
}

async fn save_news_annoy(embeddings: Vec<Embedding>) -> Result<(), OmniNewsError> {
    if embeddings.is_empty() {
        annoy_info!("[Service] No embeddings found for news.");
        return Ok(());
    }
    let embedding_dim = embeddings[0].embedding_value.as_ref().unwrap().len();
    annoy_info!("[Service] Embedding dimension: {}", embedding_dim);

    let annoy = rannoy::Rannoy::new(384);
    annoy.set_seed(123);

    for embedding in embeddings.iter() {
        let decoded_embedding = decode_embedding(embedding.embedding_value.as_ref().unwrap());
        annoy.add_item(embedding.embedding_id.unwrap(), decoded_embedding.as_ref());
    }
    // 트리 개수 증가
    annoy.build(100);
    annoy.save(PathBuf::from("../resources/news_embeddings.ann"));

    Ok(())
}
