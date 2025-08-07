use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs};

use crate::{
    global::{FcmAccessToken, FCM_ACCESS_TOKEN},
    rss_fetch_and_notification_info,
};

#[derive(Deserialize)]
struct ServiceAccount {
    client_email: String,
    private_key: String,
    token_uri: String,
}

#[derive(Serialize)]
struct Claims<'a> {
    iss: &'a str,   // issuer
    scope: &'a str, // space-separated scopes
    aud: &'a str,   // audience (token_uri)
    exp: usize,     // expiration time (epoch seconds)
    iat: usize,     // issued at (epoch seconds)
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

pub async fn get_fcm_access_token_with_expiry() -> Result<(String, u64), Box<dyn Error>> {
    // TODO 1시간 지나도 새로 잘 발급하는지 확인
    // If access token is already set and not expired, return it
    let token = FCM_ACCESS_TOKEN.lock().unwrap().clone();
    if token.is_some() {
        let token = token.unwrap();
        // Check if the token is still valid
        if token.expires_at > Utc::now() {
            rss_fetch_and_notification_info!("Using cached FCM access token");
            return Ok((token.access_token, token.expires_at.timestamp() as u64));
        }
    }
    // cargo run한 곳에서 경로 시작
    let sa_json = fs::read_to_string("omninews_firebase_sdk.json")?;
    let sa: ServiceAccount = serde_json::from_str(&sa_json)?;

    let now = Utc::now();
    let exp = now + Duration::minutes(55); // 1시간 유효
    let claims = Claims {
        iss: &sa.client_email,
        scope: "https://www.googleapis.com/auth/firebase.messaging",
        aud: &sa.token_uri,
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    // JWT 서명
    let key = EncodingKey::from_rsa_pem(sa.private_key.as_bytes())?;
    let jwt = encode(&Header::new(Algorithm::RS256), &claims, &key)?;

    // Google OAuth2 토큰 요청
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", &jwt),
    ];
    let client = Client::new();
    let res = client.post(&sa.token_uri).form(&params).send().await?;

    let token_resp: TokenResponse = res.json().await?;

    // Set new access token
    let token = FcmAccessToken {
        access_token: token_resp.access_token.clone(),
        expires_at: exp,
    };
    let mut fcm_access_token = FCM_ACCESS_TOKEN.lock().unwrap();
    fcm_access_token.replace(token);

    Ok((token_resp.access_token, token_resp.expires_in))
}
