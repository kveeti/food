use anyhow::Result;

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub database_url: String,
    pub frontend_dir: Option<String>,
    pub vapid_subject: String,
    pub vapid_public_key: String,
    pub vapid_private_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1:8000".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://food.db?mode=rwc".to_string()),
            frontend_dir: std::env::var("FRONTEND_DIR")
                .ok()
                .filter(|dir| !dir.trim().is_empty()),
            vapid_subject: std::env::var("VAPID_SUBJECT")
                .unwrap_or_else(|_| "mailto:security@veetik.com".to_string()),
            vapid_public_key: std::env::var("VAPID_PUBLIC_KEY").unwrap_or_default(),
            vapid_private_key: std::env::var("VAPID_PRIVATE_KEY").unwrap_or_default(),
        })
    }
}
