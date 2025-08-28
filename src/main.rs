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

use std::time::Duration;

use config::{env, logging};
use sqlx::MySqlPool;
use tokio::time::sleep;
use utils::embedding_util::EmbeddingService;

use crate::config::webdriver::{DriverPool, DriverPoolConfig};

#[tokio::main]
async fn main() {
    env::load_env();
    logging::load_logger();

    let pool = db_util::create_pool().await;

    let embedding_service = EmbeddingService::new();

    let dp_cfg = DriverPoolConfig::default();

    let driver_pool = DriverPool::new(dp_cfg);

    start_scheduler(&pool, &embedding_service, &driver_pool).await;
}

async fn start_scheduler(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    driver_pool: &DriverPool,
) {
    use scheduler::{
        annoy_scheduler::*, news_scheduler::*, rss_info_update_scheduler::*,
        rss_notification_scheduler::*,
    };
    sleep(Duration::from_secs(10)).await; // 서버 시작 후 10초 대기

    tokio::join!(
        // 매일 자정 1주 전 뉴스 삭제
        delete_old_news_scheduler(pool),
        // 5분마다 뉴스 패치
        fetch_news_scheduler(pool),
        // 1시간마다 Annoy 인덱스 저장
        save_annoy_scheduler(pool),
        // TODO: 아래 두개는 잘되는지 검증은 실사용 해보면서 하기.
        // 10분마다 RSS 아이템 패치 및 채널 구독자에게 알림
        rss_fetch_and_notification_scheduler(pool, embedding_service, driver_pool),
        // 매일 RSS 채널 정보 업데이트
        rss_info_update_scheduler(pool, embedding_service, driver_pool),
    );
}
