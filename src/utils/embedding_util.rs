use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};

use crate::{annoy_info, model::error::OmniNewsError, rss_fetch_and_notification_error};

struct EmbeddingRequest {
    text: String,
    response_tx: mpsc::Sender<Vec<f32>>,
}

#[derive(Clone)]
pub struct EmbeddingService {
    request_tx: Arc<Mutex<mpsc::Sender<EmbeddingRequest>>>,
}

impl EmbeddingService {
    pub fn new() -> Self {
        // 요청 채널 생성
        let (request_tx, request_rx) = mpsc::channel::<EmbeddingRequest>();

        thread::spawn(move || {
            annoy_info!("[Worker Thread] Initializing worker thread module");

            // TOO 다국어 지원 모델로 변경
            let model = SentenceEmbeddingsBuilder::remote(
                SentenceEmbeddingsModelType::DistiluseBaseMultilingualCased,
            )
            .create_model()
            .expect("[Worker Thread] Error while initalizing model");

            annoy_info!("[Worker Thread] Worker thread initialized");

            // 요청 대기
            while let Ok(request) = request_rx.recv() {
                // 임베딩 생성
                match model.encode(&[request.text]) {
                    Ok(embeddings) if !embeddings.is_empty() => {
                        let _ = request.response_tx.send(embeddings[0].clone());
                    }
                    _ => {
                        let _ = request.response_tx.send(Vec::new());
                    }
                }
            }

            annoy_info!("[Worker Thread] worker thread terminated");
        });

        Self {
            request_tx: Arc::new(Mutex::new(request_tx)),
        }
    }

    // 임베딩 생성 요청 메서드
    fn embed_text(&self, text: String) -> Result<Vec<f32>, String> {
        // 응답용 채널 생성
        let (response_tx, response_rx) = mpsc::channel();

        // 요청 생성 및 전송
        let request = EmbeddingRequest { text, response_tx };

        // 요청 전송
        self.request_tx
            .lock()
            .map_err(|_| "Failed to acquire lock".to_string())?
            .send(request)
            .map_err(|_| "Failed to request to worker thread ".to_string())?;

        // 응답 대기
        response_rx
            .recv()
            .map_err(|_| "Failed to recieved from worker thread ".to_string())
    }
}

pub async fn embedding_sentence(
    embedding_service: &EmbeddingService,
    sentence: String,
) -> Result<Vec<f32>, OmniNewsError> {
    let service = embedding_service.clone();
    // Generate Embeddings
    let embedding = tokio::task::spawn_blocking(move || service.embed_text(sentence))
        .await
        .map_err(|_| OmniNewsError::EmbeddingError)?;

    match embedding {
        Ok(mut res) => {
            // 벡터 정규화
            let norm: f32 = res.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut res {
                    *x /= norm;
                }
            }
            Ok(res)
        }
        Err(e) => {
            rss_fetch_and_notification_error!(
                "[Embedding Service] Failed to generate embedding: {}",
                e
            );
            Err(OmniNewsError::EmbeddingError)
        }
    }
}

pub fn encode_embedding(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for &value in embedding {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub fn decode_embedding(bytes: &[u8]) -> Vec<f32> {
    let mut embedding = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let mut array = [0u8; 4];
        array.copy_from_slice(chunk);
        embedding.push(f32::from_le_bytes(array));
    }
    embedding
}
