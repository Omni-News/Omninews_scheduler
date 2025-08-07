#[derive(Debug, Clone)]
pub struct FcmTokenUser {
    pub user_email: Option<String>,
    pub user_fcm_token: Option<String>,
}
