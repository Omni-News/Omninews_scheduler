use std::error::Error;

use reqwest::Client;
use serde_json::json;

use crate::{model::fcm_token::FcmTokenUser, rss_fetch_and_notification_info};

use super::firebase_util::get_fcm_access_token_with_expiry;

// TODO 다중 인원에게 보내는 것도 생각해보기
pub async fn send_fcm_message(
    token: FcmTokenUser,
    title: String,
    body: String,
) -> Result<(), Box<dyn Error>> {
    let access_token = get_fcm_access_token_with_expiry().await?;

    let payload = json!({
        "message": {
            "token": token.user_fcm_token,
            "notification": {
                "title": title,
                "body": body,
            },
        }
    });

    let url = "https://fcm.googleapis.com/v1/projects/kdh-omninews/messages:send";
    let client = Client::new();
    let resp = client
        .post(url)
        .bearer_auth(&access_token.0)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await?;
    let user = token.user_email.unwrap_or_default();
    rss_fetch_and_notification_info!("FCM 전송 응답: {user} \n {status} {text}");
    if !status.is_success() {
        return Err(format!("FCM 전송 실패: {status} {text}").into());
    }
    Ok(())
}
