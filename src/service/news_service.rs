use std::collections::HashMap;

use crate::{
    global::FETCH_FLAG,
    model::{error::OmniNewsError, news::NewNews},
    news_error, news_info, news_warn,
    repository::news_repository,
    utils::api::query_gemini_summarize,
};
use chrono::{Duration, FixedOffset, NaiveDateTime};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use sqlx::MySqlPool;
use tokio::task;

type NewsType = HashMap<String, i32>;

pub async fn delete_old_news(pool: &MySqlPool) -> Result<i32, OmniNewsError> {
    match news_repository::delete_old_news(pool).await {
        Ok(count) => Ok(count),
        Err(e) => {
            news_error!("[Service] Failed to delete old news: {:?}", e);
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn crawl_news_and_store_every_5_minutes(pool: &MySqlPool) -> Result<(), OmniNewsError> {
    let news_type = set_news_type();

    match fetch_news_and_store(pool, news_type.clone()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            news_error!("[Service] Failed to fetch news: {:?}", e);
            Err(OmniNewsError::FetchNews)
        }
    }
}

/*
* 정치 : 100
* 경제 : 101
* 사회 : 102
* 생활/문화 : 103
* 세계 : 104
* IT/과학 : 105
*/
fn set_news_type() -> NewsType {
    let mut news_type = HashMap::new();
    news_type.insert("정치".to_string(), 100);
    news_type.insert("경제".to_string(), 101);
    news_type.insert("사회".to_string(), 102);
    news_type.insert("생활/문화".to_string(), 103);
    news_type.insert("세계".to_string(), 104);
    news_type.insert("IT/과학".to_string(), 105);

    news_type
}

async fn fetch_news_and_store(pool: &MySqlPool, news_type: NewsType) -> Result<(), OmniNewsError> {
    let client = Client::new();

    for (subject, code) in &news_type {
        let res = client
            .get(format!("https://news.naver.com/section/{}", code))
            .send()
            .await?
            .text()
            .await?;

        let document = Html::parse_document(&res);
        let news_selector = Selector::parse(".sa_item_flex").unwrap();

        let newsses = make_news(document, news_selector, subject);

        for mut news in newsses {
            match news_repository::select_news_by_title(pool, news.news_title.clone().unwrap())
                .await
            {
                Ok(_) => (),
                Err(_) => {
                    // 발행일 있는 뉴스만 다룸
                    if news.news_pub_date.is_none() {
                        continue;
                    }

                    match summarize_news(news.news_link.clone().unwrap().as_str()).await {
                        Ok(res) => {
                            news.news_summary = Some(res);
                        }
                        Err(e) => {
                            // 요약 실패 시 기본 설명으로 넣음.
                            news_warn!("[Service] Failed to summarize news. {}", e);
                            news.news_summary =
                                Some(news.news_description.clone().unwrap_or_default());
                        }
                    }

                    let _ = news_repository::insert_news(pool, news.clone())
                        .await
                        .map_err(|e| {
                            news_error!("[Service] Failed to insert news: {:?}", e);
                            OmniNewsError::Database(e)
                        });
                }
            };
        }
    }
    // 뉴스 패치가 끝났음으로 fetch_flag를 true로 설정
    // 비동기 함수에 Send 트레이트가 필요하므로, task::spawn_blocking을 사용하여 처리
    task::spawn_blocking(move || {
        let mut fetch_flag = FETCH_FLAG.lock().unwrap();
        *fetch_flag = true;
        news_info!("[Service] Fetching news completed, fetch_flag set to true");
    })
    .await
    .map_err(|_| OmniNewsError::FetchNews)?;
    Ok(())
}

fn make_news(document: Html, news_selector: Selector, subject: &String) -> Vec<NewNews> {
    let title_selector = Selector::parse(".sa_text_strong").unwrap();
    let description_selector = Selector::parse(".sa_text_lede").unwrap();
    let link_selector = Selector::parse(".sa_thumb_link").unwrap();
    let source_selector = Selector::parse(".sa_text_press").unwrap();
    let pub_date_selector = Selector::parse(".sa_text_datetime > b").unwrap();
    let image_link_selector = Selector::parse(".sa_thumb_link > img").unwrap();

    document
        .select(&news_selector)
        .map(|news| NewNews {
            news_title: Some(
                news.select(&title_selector)
                    .next()
                    .map(|e| e.inner_html())
                    .unwrap_or_default(),
            ),
            news_description: Some(
                news.select(&description_selector)
                    .next()
                    .map(|e| e.inner_html())
                    .unwrap_or_default(),
            ),
            news_summary: None,
            news_link: Some(
                news.select(&link_selector)
                    .next()
                    .map(|e| e.attr("href").unwrap())
                    .unwrap_or_default()
                    .to_string(),
            ),
            news_source: Some(
                news.select(&source_selector)
                    .next()
                    .map(|e| e.inner_html())
                    .unwrap_or_default(),
            ),
            news_pub_date: pub_date_to_naive_time(
                news.select(&pub_date_selector)
                    .next()
                    .map(|e| e.inner_html())
                    .unwrap_or_default(),
            ),
            news_image_link: Some(
                news.select(&image_link_selector)
                    .next()
                    .map(|e| e.attr("data-src").unwrap_or_default())
                    .unwrap_or_default()
                    .to_string(),
            ),
            news_category: Some(subject.to_string()),
        })
        .collect::<Vec<NewNews>>()
}

fn pub_date_to_naive_time(pub_date: String) -> Option<NaiveDateTime> {
    let current_time = chrono::Local::now();

    let kst = FixedOffset::east_opt(9 * 3600).unwrap();

    if pub_date.is_empty() {
        return None;
    }

    // 한글은 글자 당 3바이트
    if pub_date.ends_with("분전") {
        let (value, _) = pub_date.split_at(pub_date.len() - 6);
        Some(
            NaiveDateTime::parse_from_str(
                &current_time
                    .with_timezone(&kst)
                    .checked_sub_signed(Duration::minutes(value.parse::<i64>().unwrap_or_default()))
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap_or_default(),
        )
    } else if pub_date.ends_with("시간전") {
        let (value, _) = pub_date.split_at(pub_date.len() - 9);
        Some(
            NaiveDateTime::parse_from_str(
                &current_time
                    .with_timezone(&kst)
                    .checked_sub_signed(Duration::hours(value.parse::<i64>().unwrap_or_default()))
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap_or_default(),
        )
    } else {
        None
    }
}

async fn summarize_news(news_link: &str) -> Result<String, OmniNewsError> {
    let client = Client::new();

    let res = client.get(news_link).send().await?.text().await?;

    // HTML 파싱 동기 작업을 spawn_blocking으로 감싸서 별도 스레드에서 실행
    let content = task::spawn_blocking(move || {
        let document = Html::parse_document(&res);
        let news_selector = Selector::parse("#dic_area").unwrap();

        document
            .select(&news_selector)
            .next()
            .map(|element| {
                // <br> 태그를 줄바꿈으로 변환
                let html_with_newlines =
                    element.html().replace("<br>", "\n").replace("<br/>", "\n");

                // 모든 HTML 태그 제거
                let re = Regex::new(r"<[^>]*>").unwrap();
                let text = re.replace_all(&html_with_newlines, "");

                // HTML 엔티티 디코딩
                let decoded = html_escape::decode_html_entities(&text);

                // 연속된 공백과 줄바꿈 정리
                decoded
                    .split('\n')
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    })
    .await
    .map_err(|e| {
        news_error!("[Service] spawn_blocking failed: {:?}", e);
        OmniNewsError::FetchNews
    })?;
    news_info!("[Service] Fetched content length: {}", content.len());
    let summary = query_gemini_summarize(50, &content).await;

    Ok(summary)
}
