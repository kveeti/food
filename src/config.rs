use std::env;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use url::Url;

pub struct Config {
    pub app_url: Url,
    pub database_url: String,
    pub oidc_issuer: Url,
    pub oidc_client_id: String,
    pub oidc_client_secret: String,
    pub oidc_audience: Option<String>,
    pub session_encryption_key: [u8; 32],
}

impl Config {
    #[tracing::instrument(name = "config::from_env", level = "debug", skip_all)]
    pub fn from_env() -> Result<Self, String> {
        let app_url = required_url("APP_URL")?;
        if app_url.path() != "/" || app_url.query().is_some() || app_url.fragment().is_some() {
            return Err("APP_URL must contain only an origin".to_owned());
        }

        let oidc_issuer = required_url("OIDC_ISSUER")?;
        if oidc_issuer.scheme() != "https" && env::var("ALLOW_INSECURE_OIDC").as_deref() != Ok("1")
        {
            return Err("OIDC_ISSUER must use HTTPS".to_owned());
        }

        let key = STANDARD
            .decode(required("SESSION_ENCRYPTION_KEY")?)
            .map_err(|_| "SESSION_ENCRYPTION_KEY must be valid base64".to_owned())?
            .try_into()
            .map_err(|_| "SESSION_ENCRYPTION_KEY must contain 32 bytes".to_owned())?;

        Ok(Self {
            app_url,
            database_url: required("DATABASE_URL")?,
            oidc_issuer,
            oidc_client_id: required("OIDC_CLIENT_ID")?,
            oidc_client_secret: required("OIDC_CLIENT_SECRET")?,
            oidc_audience: env::var("OIDC_AUDIENCE")
                .ok()
                .filter(|value| !value.is_empty()),
            session_encryption_key: key,
        })
    }
}

fn required(name: &str) -> Result<String, String> {
    let value = env::var(name).map_err(|_| format!("{name} must be set"))?;
    if value.is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(value)
}

fn required_url(name: &str) -> Result<Url, String> {
    Url::parse(&required(name)?).map_err(|_| format!("{name} must be a URL"))
}
