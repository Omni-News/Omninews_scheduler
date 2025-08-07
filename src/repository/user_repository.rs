use sqlx::{query, MySqlPool};

use crate::{db_util::get_db, model::fcm_token::FcmTokenUser};

pub async fn selsect_users_fcm_token_subscribed_channel_by_channel_id(
    pool: &MySqlPool,
    channel_id: i32,
) -> Result<Vec<FcmTokenUser>, sqlx::Error> {
    let mut conn = get_db(pool).await?;

    let result = query!(
        "SELECT u.user_email, u.user_fcm_token FROM user u
        JOIN user_subscription_channel usc ON u.user_id = usc.user_id
        WHERE usc.channel_id = ? AND u.user_fcm_token IS NOT NULL AND u.user_notification_push = true",
        channel_id
    )
    .fetch_all(&mut *conn)
    .await;

    match result {
        Ok(res) => Ok(res
            .into_iter()
            .map(|r| FcmTokenUser {
                user_email: Some(r.user_email),
                user_fcm_token: r.user_fcm_token,
            })
            .collect()),
        Err(e) => Err(e),
    }
}
