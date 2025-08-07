use sqlx::MySqlPool;

use crate::{
    model::{error::OmniNewsError, fcm_token::FcmTokenUser},
    repository::user_repository,
    rss_fetch_and_notification_error,
};

pub async fn get_users_fcm_token_subscribed_channel_by_channel_id(
    pool: &MySqlPool,
    channel_id: i32,
) -> Result<Vec<FcmTokenUser>, OmniNewsError> {
    match user_repository::selsect_users_fcm_token_subscribed_channel_by_channel_id(
        pool, channel_id,
    )
    .await
    {
        Ok(res) => Ok(res),
        Err(e) => {
            rss_fetch_and_notification_error!(
                "[Service] Failed to check if user is subscribed to RSS channel: {}",
                e
            );
            Err(OmniNewsError::Database(e))
        }
    }
}
