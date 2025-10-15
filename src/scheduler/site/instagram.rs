use std::{env, time::Duration};

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use rss::ItemBuilder;
use serde_json::Value;
use sqlx::MySqlPool;
use thirtyfour::{error::WebDriverError, By, WebDriver, WebElement};
use tokio::time::sleep;

use crate::{
    config::webdriver::{AcquireStrategy, DriverPool},
    model::{
        error::OmniNewsError,
        rss::{NewRssChannel, NewRssItem, RssChannel},
    },
    service::rss::{channel_service, item_service},
    utils::embedding_util::EmbeddingService,
};

pub async fn update_instagram_channel_info(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    driver_pool: &DriverPool,
    channel: &RssChannel,
) -> Result<i32, OmniNewsError> {
    let link = channel.channel_link.clone().unwrap_or_default();
    let strategy = AcquireStrategy::Wait(Some(Duration::from_secs(10)));
    let driver_handle = driver_pool.acquire(strategy).await.map_err(|e| {
        error!("[Service-instagram] Failed to acquire WebDriver: {:?}", e);
        OmniNewsError::WebDriverPool(e)
    })?;
    let driver = driver_handle.driver();

    let username = extract_username(&link).ok_or_else(|| OmniNewsError::ExtractLinkError)?;
    let instagram_channel_link = format!("http://instagram.com/{username}");

    let feeds_graphql_url = format!(
        r#"
            http://www.instagram.com/graphql/query?variables={{"data":{{"count":12,"include_relationship_info":false,"latest_besties_reel_media":false,"latest_reel_media":true}},"username":"{username}","__relay_internal__pv__PolarisFeedShareMenurelayprovider":false}}&doc_id=7898261790222653&server_timestamps=true
        "#
    );
    // TODO: 아니 좀 쓰니까 내 계정 잠김. 이거 나중에 해보고 계속 잠겨있으면 메일파서 새로 ㄱㄱ;
    let _ = driver
        .goto(feeds_graphql_url.clone())
        .await
        .map_err(map_wd_err);
    let is_sign_in = is_sign_in_by_graphql(driver).await?;

    let channel_id;
    if is_sign_in {
        channel_id =
            update_channel_info(pool, embedding_service, driver, &instagram_channel_link).await?;
    } else {
        // 로그인
        let _ = driver
            .goto("http://www.instagram.com")
            .await
            .map_err(map_wd_err);
        sleep(Duration::from_millis(1000)).await;
        if is_login_page(driver).await? {
            attempt_login(driver).await?;
            channel_id =
                update_channel_info(pool, embedding_service, driver, &instagram_channel_link)
                    .await?;
        } else {
            // 혹시 로그인 유도 모달(닫기 버튼) 존재시 닫기 (한국어/영어 모두 대응)
            error!("여기오면ㅇ ㅏㄴ됨.");
            dismiss_close_overlay(driver).await.ok();
            channel_id =
                update_channel_info(pool, embedding_service, driver, &instagram_channel_link)
                    .await?;
        }
    }
    Ok(channel_id)
}

async fn update_channel_info(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    driver: &WebDriver,
    link: &str,
) -> Result<i32, OmniNewsError> {
    let _ = driver.goto(link).await.map_err(map_wd_err);
    sleep(Duration::from_millis(500)).await;

    let (channel_title, channel_description, channel_image_url) =
        extract_profile_meta(driver).await?;

    let channel_id = channel_service::get_channel_id_by_channel_link(pool, link).await?;
    let channel = channel_service::get_rss_channel_by_id(pool, channel_id).await?;

    let updated_rss_channel = NewRssChannel::new(
        channel_title,
        channel.channel_link.unwrap_or_default(),
        channel_description,
        channel_image_url.into(),
        channel.channel_language.unwrap_or_default(),
        channel.rss_generator.unwrap_or_default(),
        channel.channel_rank.unwrap_or(0),
        channel.channel_rss_link.unwrap_or_default(),
    );

    match channel_service::update_rss_channel_and_embedding(
        pool,
        embedding_service,
        &updated_rss_channel,
        channel_id,
    )
    .await
    {
        Ok(_) => info!(
            "[Service-instagram] Updated channel info and embedding for link: {}",
            link
        ),
        Err(e) => {
            error!(
                "[Service-instagram] Failed to update channel info and embedding for link: {}, error: {:?}",
                link, e
            );
            return Err(e);
        }
    };

    Ok(channel_id)
}

pub async fn fetch_instagram_rss_and_store(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    driver_pool: &DriverPool,
    link: &str,
    channel_id: i32,
) -> Result<Vec<String>, OmniNewsError> {
    // update info랑 겹치지 않기 위함.
    sleep(Duration::from_millis(5000)).await;
    let strategy = AcquireStrategy::Wait(Some(Duration::from_secs(10)));
    let driver_handle = driver_pool.acquire(strategy).await.map_err(|e| {
        error!("[Service-instagram] Failed to acquire WebDriver: {:?}", e);
        OmniNewsError::WebDriverPool(e)
    })?;
    let driver = driver_handle.driver();
    let username = extract_username(link).ok_or_else(|| OmniNewsError::ExtractLinkError)?;

    let feeds_graphql_url = format!(
        r#"
            http://www.instagram.com/graphql/query?variables={{"data":{{"count":12,"include_relationship_info":false,"latest_besties_reel_media":false,"latest_reel_media":true}},"username":"{username}","__relay_internal__pv__PolarisFeedShareMenurelayprovider":false}}&doc_id=7898261790222653&server_timestamps=true
        "#
    );

    let _ = driver
        .goto(feeds_graphql_url.clone())
        .await
        .map_err(map_wd_err);
    let is_sign_in = is_sign_in_by_graphql(driver).await?;

    let item_titles: Vec<String>;
    if is_sign_in {
        item_titles = fetch_rss_and_store_new_feeds(
            pool,
            embedding_service,
            driver,
            feeds_graphql_url,
            channel_id,
        )
        .await?;
    } else {
        // 로그인
        info!("[OInstagram-fetch] Sign in...");
        let _ = driver
            .goto("http://www.instagram.com")
            .await
            .map_err(map_wd_err);
        info!("[Instagram-fetch]goto instagram.com");
        info!("[Instagram-fetch] check is login page");
        sleep(Duration::from_millis(1000)).await;
        if is_login_page(driver).await? {
            info!("[Instagram-fetch]  is login page. attempt login");
            attempt_login(driver).await?;
            sleep(Duration::from_millis(3000)).await;
            info!("[Instagram-fetch] login success");
            item_titles = fetch_rss_and_store_new_feeds(
                pool,
                embedding_service,
                driver,
                feeds_graphql_url,
                channel_id,
            )
            .await?;
        } else {
            warn!("여기오면안됨.");
            // 혹시 로그인 유도 모달(닫기 버튼) 존재시 닫기 (한국어/영어 모두 대응)
            error!("여기오면 안됩니다. 다시 시도하세요.");
            return Err(OmniNewsError::NotFound("메롱".to_string()));
        }
    }

    Ok(item_titles)
}

async fn fetch_rss_and_store_new_feeds(
    pool: &MySqlPool,
    embedding_service: &EmbeddingService,
    driver: &WebDriver,
    feeds_graphql_url: String,
    channel_id: i32,
) -> Result<Vec<String>, OmniNewsError> {
    let _ = driver.goto(feeds_graphql_url).await.map_err(map_wd_err);
    sleep(Duration::from_millis(1000)).await;

    let data = driver.find(By::Css("body")).await.map_err(map_wd_err);
    match data {
        Ok(res) => {
            // 새로운 item들
            let new_items = build_item_not_exist_in_db(pool, channel_id, res).await?;
            info!(
                "instagram new_items: {}",
                new_items
                    .iter()
                    .map(|i| i.rss_title.clone().unwrap_or_default())
                    .collect::<Vec<String>>()
                    .join(", ")
            );

            for item in &new_items {
                let _ = item_service::create_rss_item_and_embedding(
                    pool,
                    embedding_service,
                    item.clone(),
                )
                .await
                .map_err(|e| {
                    error!(
                        "[Service-instagram] Failed to create rss item and embedding: {:?}",
                        e
                    );
                    e
                });
            }

            Ok(new_items
                .iter()
                .map(|i| i.rss_title.clone().unwrap_or_default())
                .collect::<Vec<String>>())
        }
        Err(_) => {
            error!("[Instagram] Failed to get body in graphql data.");
            Err(OmniNewsError::NotFound(
                "Failed to get body in graphql data.".to_string(),
            ))
        }
    }
}

async fn build_item_not_exist_in_db(
    pool: &MySqlPool,
    channel_id: i32,
    el: WebElement,
) -> Result<Vec<NewRssItem>, OmniNewsError> {
    let mut items = Vec::new();

    let data_s = el.text().await.map_err(map_wd_err)?;
    let data_v: Value = match serde_json::from_str(data_s.as_str()) {
        Ok(v) => v,
        Err(e) => {
            error!("[Instagram-fetch] Failed to parse graphql data to json: {e}");
            return Err(OmniNewsError::ParseError);
        }
    };
    let items_json = data_v
        .get("data")
        .and_then(|v| v.get("xdt_api__v1__feed__user_timeline_graphql_connection"))
        .and_then(|v| v.get("edges"))
        .and_then(|v| v.as_array())
        .unwrap();
    sleep(Duration::from_millis(1000)).await;
    for v in items_json {
        let raw_texts = match v
            .get("node")
            .and_then(|v| v.get("caption"))
            .and_then(|v| v.get("text"))
        {
            Some(res) => res,
            None => continue,
        }
        .to_string();

        let texts = raw_texts.split("\\n").collect::<Vec<&str>>();

        let title = texts.first().unwrap_or(&"");
        let description = texts.join(" ");
        let feed_code = v
            .get("node")
            .and_then(|v| v.get("code"))
            .and_then(|v| v.as_str());

        let link = if let Some(code) = feed_code {
            format!("http://instagram.com/p/{code}")
        } else {
            "".to_string()
        };

        let author = v
            .get("node")
            .and_then(|v| v.get("user"))
            .and_then(|v| v.get("full_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let pub_date_timestamp = match v
            .get("node")
            .and_then(|v| v.get("caption"))
            .and_then(|v| v.get("created_at"))
        {
            Some(v) => v.as_i64(),
            None => v
                .get("node")
                .and_then(|v| v.get("taken_at"))
                .and_then(|v| v.as_i64()),
        }
        .map(|v| Utc.timestamp_opt(v, 0))
        .unwrap()
        .unwrap();

        let kst = FixedOffset::east_opt(9 * 3600).unwrap();
        let pub_date_kst: DateTime<FixedOffset> = pub_date_timestamp.with_timezone(&kst);
        let pub_date_rfc2822 = pub_date_kst.to_rfc2822();

        let image_link = v
            .get("node")
            .and_then(|v| v.get("image_versions2"))
            .and_then(|v| v.get("candidates"))
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 이미 link가 db에 존재하는지 확인.
        if let Ok(res) = item_service::is_exist_rss_item_by_link(pool, &link).await {
            if res {
                continue;
            }
        }

        let item = ItemBuilder::default()
            .title(title.to_string())
            .description(description)
            .link(link)
            .author(author.to_string())
            .pub_date(pub_date_rfc2822.clone())
            .build();

        info!("pub_date_rfc2822: {}", pub_date_rfc2822);
        let pub_date = match DateTime::parse_from_rfc2822(&pub_date_rfc2822) {
            Ok(res) => res.naive_local(),
            Err(e) => {
                error!("Failed to parse NaiveDateTime from pub_date_rfc2822: {e}");
                return Err(OmniNewsError::ParseError);
            }
        };

        let new_item = NewRssItem::new(channel_id, &item, Some(pub_date), image_link);

        items.push(new_item);
    }

    Ok(items)
}

/* ---------------- Helper Functions ---------------- */

fn extract_username(link: &str) -> Option<String> {
    // https://www.instagram.com/{username}/
    let trimmed = link.trim_end_matches('/');
    trimmed.rsplit('/').next().map(|s| s.to_string())
}

async fn is_sign_in_by_graphql(driver: &WebDriver) -> Result<bool, OmniNewsError> {
    let body = driver.find(By::XPath(".//body")).await?;
    let body_len = body.text().await.unwrap().len();
    Ok(body_len > 200)
}

async fn is_login_page(driver: &WebDriver) -> Result<bool, OmniNewsError> {
    Ok(driver.find(By::Name("username")).await.is_ok()
        || driver.find(By::Name("password")).await.is_ok())
}

async fn attempt_login(driver: &WebDriver) -> Result<(), OmniNewsError> {
    let username = env::var("INSTAGRAM_ID").expect("INSTAGRAM_ID is must be set.");

    let password = env::var("INSTAGRAM_PW").expect("INSTAGRAM_PW is must be set.");

    info!("[Instagram] Attempting login...");

    let user_field = driver
        .find(By::Name("username"))
        .await
        .map_err(map_wd_err)?;
    let pass_field = driver
        .find(By::Name("password"))
        .await
        .map_err(map_wd_err)?;

    user_field.send_keys(username).await.map_err(map_wd_err)?;
    pass_field.send_keys(password).await.map_err(map_wd_err)?;

    sleep(Duration::from_millis(600)).await;
    if let Ok(btn) = driver.find(By::XPath("//button[@type='submit']")).await {
        btn.click().await.map_err(map_wd_err)?;
    }

    // save login info window
    sleep(Duration::from_millis(7000)).await;
    let save_info_el = driver
        .find(By::XPath(
            "//button[text()='정보 저장'] | //button[text()='Save info']",
        ))
        .await;

    sleep(Duration::from_millis(1000)).await;

    if save_info_el.is_ok() {
        if let Ok(btn) = save_info_el {
            btn.click().await.map_err(map_wd_err)?;
            info!("[Instagram] Saved login info.");
        } else {
            info!("[Instagram] No 'Save login info' button found.");
        }
    }

    // 로그인 처리 대기 (최대 30초)
    for _ in 0..10 {
        if !is_login_page(driver).await? {
            info!("[Instagram] Login success (username/password gone).");
            return Ok(());
        }
        info!("[Instagram] Waiting for login...");
        sleep(Duration::from_secs(3)).await;
    }

    error!("[Service-Instagram] Login failed or timed out.");
    Err(OmniNewsError::WebDriverNotFound)
}

// TODO: 동작 안함.
async fn dismiss_close_overlay(driver: &WebDriver) -> Result<(), OmniNewsError> {
    // 한국어 title='닫기', 영어 title='Close'
    if let Ok(close_div) = driver
        .find(By::XPath(
            "//div[.//svg/title[text()='닫기' or text()='Close']]",
        ))
        .await
    {
        close_div.click().await.map_err(map_wd_err)?;
        sleep(Duration::from_millis(400)).await;
        info!("[Instagram] Closed overlay (Close/닫기).");
    }
    Ok(())
}

async fn extract_profile_meta(
    driver: &WebDriver,
) -> Result<(String, String, String), OmniNewsError> {
    // meta og:title, og:description, og:image
    let title = get_meta_content(driver, "og:title").await?;
    let desc = get_meta_content(driver, "og:description")
        .await
        .unwrap_or_else(|_| "".into());
    let img = get_meta_content(driver, "og:image")
        .await
        .unwrap_or_default();

    Ok((title, desc, img))
}

async fn get_meta_content(driver: &WebDriver, property: &str) -> Result<String, OmniNewsError> {
    let selector = format!("meta[property='{property}']");
    let el = driver.find(By::Css(&selector)).await.map_err(map_wd_err)?;
    let content = el
        .attr("content")
        .await
        .map_err(map_wd_err)?
        .unwrap_or_default();
    Ok(content)
}

fn map_wd_err(e: WebDriverError) -> OmniNewsError {
    OmniNewsError::WebDriverError(e)
}
