#[macro_use]
extern crate rocket;

mod config;
mod db_util;
mod global;
mod model;
mod repository;
mod scheduler;
mod service;
mod utils;

use config::{env, logging};
use sqlx::MySqlPool;
use utils::embedding_util::EmbeddingService;

#[tokio::main]
async fn main() {
    env::load_env();
    logging::load_logger();

    let pool = db_util::create_pool().await;

    let embedding_service = EmbeddingService::new();

    start_scheduler(&pool, &embedding_service).await;
}

async fn start_scheduler(pool: &MySqlPool, embedding_service: &EmbeddingService) {
    use scheduler::{
        annoy_scheduler::*, news_scheduler::*, rss_info_update_scheduler::*,
        rss_notification_scheduler::*,
    };

    tokio::join!(
        // 매일 1주 전 뉴스 삭제
        delete_old_news_scheduler(pool),
        // 5분마다 뉴스 패치
        fetch_news_scheduler(pool),
        // 1시간마다 Annoy 인덱스 저장
        save_annoy_scheduler(pool),
        // 10분마다 RSS 아이템 패치 및 채널 구독자에게 알림
        rss_fetch_and_notification_scheduler(pool, embedding_service),
        // 매일 RSS 채널 정보 업데이트
        rss_info_update_scheduler(pool),
    );
}
