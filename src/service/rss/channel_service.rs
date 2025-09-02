use reqwest::Url;
use rss::Channel;
use serde_json::Value;
use sqlx::MySqlPool;
use thirtyfour::WebDriver;

use crate::{
    model::{
        embedding::NewEmbedding,
        error::OmniNewsError,
        rss::{NewRssChannel, RssChannel},
    },
    repository::rss_channel_repository,
    rss_fetch_and_notification_error, rss_info_error,
    service::embedding_service,
    utils::embedding_util::EmbeddingService,
};

pub async fn get_channel_id_by_rss_link(
    pool: &MySqlPool,
    rss_link: &str,
) -> Result<i32, OmniNewsError> {
    match rss_channel_repository::select_channel_id_by_rss_link(pool, rss_link).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_fetch_and_notification_error!(
                "[Service] Failed to select channel id by rss link: {:?}",
                e
            );
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn get_channel_id_by_channel_link(
    pool: &MySqlPool,
    channel_link: &str,
) -> Result<i32, OmniNewsError> {
    info!("channel link: {}", channel_link);
    match rss_channel_repository::select_channel_id_by_channel_link(pool, channel_link).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_fetch_and_notification_error!(
                "[Service] Failed to select channel id by channel link: {:?}",
                e
            );
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn parse_rss_link_to_channel(link: &str) -> Result<Channel, OmniNewsError> {
    let response = reqwest::get(link).await.map_err(|e| {
        rss_fetch_and_notification_error!("[Service] Not found url : {}", link);
        OmniNewsError::Request(e)
    })?;
    let body = response.text().await.map_err(OmniNewsError::Request)?;
    Channel::read_from(body.as_bytes()).map_err(|e| {
        rss_fetch_and_notification_error!(
            "[Service] Failed to read from rss body: {:?}, link: {link}",
            e,
        );
        OmniNewsError::FetchUrl
    })
}

pub fn make_rss_channel(
    channel: &Channel,
    rss_link: String,
    is_generated_channel: bool,
) -> NewRssChannel {
    NewRssChannel::new(
        channel.title().to_string(),
        channel.link().to_string(),
        channel.description().to_string(),
        channel.image().map(|e| e.url().to_string()),
        channel.language().unwrap_or("None").to_string(),
        channel
            .generator()
            .unwrap_or(if is_generated_channel {
                "Omninews_default"
            } else {
                "None"
            })
            .to_string(),
        0,
        rss_link,
    )
}

pub async fn get_default_rss_channels(pool: &MySqlPool) -> Result<Vec<RssChannel>, OmniNewsError> {
    match rss_channel_repository::select_default_rss_channels(pool).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_info_error!("[Service] Failed to select default rss: {:?}", e);
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn get_default_rss_links(pool: &MySqlPool) -> Result<Vec<String>, OmniNewsError> {
    match rss_channel_repository::select_default_rss_links(pool).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_info_error!("[Service] Failed to select default rss links: {:?}", e);
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn get_rss_channel_by_parse(rss_link: &str) -> Result<NewRssChannel, OmniNewsError> {
    let channel = parse_rss_link_to_channel(rss_link).await?;
    let new_channel = make_rss_channel(&channel, rss_link.to_string(), false);

    Ok(new_channel)
}

pub async fn get_rss_channel_by_web_driver(
    driver: &WebDriver,
    rss_link: &str,
) -> Result<NewRssChannel, OmniNewsError> {
    let channel = parse_rss_link_to_channel_with_web_driver(rss_link, driver).await?;
    let new_channel = make_rss_channel(&channel, rss_link.to_string(), false);

    Ok(new_channel)
}

pub async fn get_rss_channel_by_id(
    pool: &MySqlPool,
    channel_id: i32,
) -> Result<RssChannel, OmniNewsError> {
    match rss_channel_repository::select_channel_by_id(pool, channel_id).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_info_error!("[Service] Failed to select rss channel by id: {:?}", e);
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn update_rss_channel_and_embedding(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    rss_channel: &NewRssChannel,
    channel_id: i32,
) -> Result<bool, OmniNewsError> {
    let _ = match embedding_update_channel(pool, embedding_service, rss_channel).await {
        Ok(res) => res,
        Err(e) => {
            rss_info_error!("[Service] Failed to update embedding for channel: {:?}", e);
            return Err(e);
        }
    };
    match rss_channel_repository::update_rss_channel_by_id(pool, rss_channel, channel_id).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_info_error!("[Service] Failed to update rss info: {:?}", e);
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn embedding_update_channel(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    rss_channel: &NewRssChannel,
) -> Result<i32, OmniNewsError> {
    let channel_id = get_channel_id_by_rss_link(
        pool,
        &rss_channel.channel_rss_link.clone().unwrap_or_default(),
    )
    .await?;

    let embedding_text = prepare_embedding_text(
        &rss_channel.channel_title.clone().unwrap_or_default(),
        &rss_channel.channel_description.clone().unwrap_or_default(),
    );

    let embedding = NewEmbedding {
        embedding_value: None,
        channel_id: Some(channel_id),
        rss_id: None,
        news_id: None,
        embedding_source_rank: Some(0),
    };
    embedding_service::update_embedding(pool, embedding_service, embedding_text, embedding).await?;
    Ok(channel_id)
}

fn prepare_embedding_text(title: &str, description: &str) -> String {
    // 1. HTML 태그 제거
    let clean_description = remove_html_tags(description);

    // 2. 구조화된 형식으로 정보 표현
    let mut text = format!("제목: {title}. 내용: {clean_description}");

    // 3. 특수문자 정리 및 중복 공백 제거 - 한글 보존 처리 추가
    text = text
        .replace(
            |c: char| {
                !c.is_alphanumeric()
                    && !c.is_whitespace()
                    && !is_hangul(c)
                    && c != '.'
                    && c != ','
                    && c != ':'
            },
            " ",
        )
        .replace("  ", " ")
        .trim()
        .to_string();

    // 4. 텍스트 길이 제한 (임베딩 모델의 최대 입력 길이 고려)
    if text.len() > 512 {
        text.truncate(512);
    }

    // 5. 제목 반복으로 중요성 강조 (선택적)
    text = format!("{text}. {title}");

    text
}

// 한글 문자 판별 함수 추가
fn is_hangul(c: char) -> bool {
    let cp = c as u32;
    // 한글 유니코드 범위 (가~힣)
    (0xAC00..=0xD7A3).contains(&cp) ||
    // 한글 자음/모음
    (0x1100..=0x11FF).contains(&cp) ||
    (0x3130..=0x318F).contains(&cp)
}

// HTML 태그 제거 함수
fn remove_html_tags(text: &str) -> String {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(text, "").to_string()
}
pub async fn get_rss_channels_with_webdriver(
    pool: &MySqlPool,
) -> Result<Vec<RssChannel>, OmniNewsError> {
    match rss_channel_repository::select_rss_channel_with_webdriver(pool).await {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_info_error!(
                "[Service] Failed to select channels with webdriver: {:?}",
                e
            );
            Err(OmniNewsError::Database(e))
        }
    }
}

pub async fn get_rss_channel_by_rss_link_crawl(
    link: String,
    driver: &WebDriver,
) -> Result<Channel, OmniNewsError> {
    let rss_channel = parse_rss_link_to_channel_with_web_driver(&link, driver).await?;

    if &rss_channel.title == "Not Found" || rss_channel.title.is_empty() {
        error!(
            "[Service] Failed to parse RSS link: {}, title is empty or not found",
            link
        );
        return Err(OmniNewsError::NotFound(
            "Failed to parse RSS link".to_string(),
        ));
    }
    Ok(rss_channel)
}

pub async fn parse_rss_link_to_channel_with_web_driver(
    link: &str,
    driver: &WebDriver,
) -> Result<Channel, OmniNewsError> {
    if let Ok(u) = Url::parse(link) {
        let origin = format!("{}://{}/", u.scheme(), u.host_str().unwrap_or_default());
        let _ = driver.goto(&origin).await;
    }
    // async script로 fetch → text 본문 받기
    // file download이므로, 본문을 text로 변환하여 반환
    let js = r#"
                const url = arguments[0];
                const done = arguments[arguments.length - 1];
                fetch(url, {
                    method: 'GET',
                    headers: {
                    'Accept': 'application/rss+xml, application/atom+xml, application/xml;q=0.9, text/xml;q=0.8, */*;q=0.1',
                    'Cache-Control': 'no-cache',
                    },
                    credentials: 'include'
                }).then(async (r) => {
                    const body = await r.text();
                    done({
                        ok: r.ok,
                        status: r.status,
                        contentType: r.headers.get('content-type'),
                        body
                    });
                }).catch(e => done({ ok: false, status: 0, contentType: null, body: String(e) }));
            "#;

    let ret = driver
        .execute_async(js, vec![Value::String(link.to_string())])
        .await?;

    let obj = ret.json().as_object().unwrap();
    let ok = obj.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let status = obj.get("status").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let ctype = obj
        .get("contentType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let body = obj
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or("no body")
        .unwrap()
        .to_string();

    if !ok || status >= 400 {
        error!(
            "[Service] Failed to fetch rss link: {}, status: {}, content-type: {}, body: {}",
            link, status, ctype, body
        );
        return Err(OmniNewsError::WebDriverNotFound);
    }
    if ctype.contains("text/html") && body.contains("Attention Required") {
        error!(
            "[Service] WebDriver blocked by Cloudflare or similar service for link: {}",
            link
        );
        return Err(OmniNewsError::WebDriverNotFound);
    }
    Channel::read_from(body.as_bytes()).map_err(|e| {
        error!(
            "[Service] Failed to read from rss body-with webdriver: {:?}, link: {link},\n body: {:?}",
            e, body
        );
        OmniNewsError::ParseRssChannel
    })
}
