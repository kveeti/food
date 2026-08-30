use std::{future::Future, time::Duration};

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, migrate::MigrateError, types::Uuid};

#[derive(Clone)]
pub struct Data {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub access_token: Vec<u8>,
    pub refresh_token: Vec<u8>,
    pub refresh_retry_after: Option<DateTime<Utc>>,
    pub refresh_expires_at: DateTime<Utc>,
    pub email: Option<String>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub issuer: String,
    pub subject: String,
}

pub struct NewSession<'a> {
    pub id: Uuid,
    pub token_hash: &'a [u8],
    pub access_token: Vec<u8>,
    pub refresh_token: Vec<u8>,
    pub oidc_session_id: &'a str,
    pub refresh_expires_at: DateTime<Utc>,
    pub issuer: &'a str,
    pub subject: &'a str,
    pub email: Option<&'a str>,
}

pub enum SessionChange {
    Keep,
    Delete,
    RetryAfter(DateTime<Utc>),
    SaveTokens {
        access_token: Vec<u8>,
        refresh_token: Vec<u8>,
        refresh_expires_at: DateTime<Utc>,
    },
}

enum SessionLock {
    Wait,
    Skip,
}

#[must_use]
pub struct SessionUpdate<'a> {
    data: &'a Data,
    token_hash: &'a [u8],
    lock: SessionLock,
}

impl Data {
    #[tracing::instrument(name = "data::new", level = "info", skip_all)]
    pub async fn new(database_url: &str) -> Result<Self> {
        let data = Self {
            pool: PgPool::connect(database_url).await?,
        };
        data.migrate().await?;
        Ok(data)
    }

    #[tracing::instrument(name = "data::migrate", level = "info", skip_all)]
    async fn migrate(&self) -> Result<(), MigrateError> {
        sqlx::migrate!().run(&self.pool).await
    }

    #[tracing::instrument(name = "data::create_session", level = "debug", skip_all)]
    pub async fn create_session(&self, session: NewSession<'_>) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        let user_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO users (id, issuer, subject, email)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (issuer, subject)
             DO UPDATE SET email = EXCLUDED.email, updated_at = now()
             RETURNING id",
        )
        .bind(Uuid::now_v7())
        .bind(session.issuer)
        .bind(session.subject)
        .bind(session.email)
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO sessions
             (id, token_hash, user_id, access_token, refresh_token, oidc_session_id, refresh_expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(session.id)
        .bind(session.token_hash)
        .bind(user_id)
        .bind(session.access_token)
        .bind(session.refresh_token)
        .bind(session.oidc_session_id)
        .bind(session.refresh_expires_at)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    #[tracing::instrument(name = "data::session", level = "debug", skip_all)]
    pub async fn session(&self, token_hash: &[u8]) -> Result<Option<Session>> {
        Ok(sqlx::query_as::<_, Session>(SESSION_QUERY)
            .bind(token_hash)
            .fetch_optional(&self.pool)
            .await?)
    }

    pub fn update_session<'a>(&'a self, token_hash: &'a [u8]) -> SessionUpdate<'a> {
        SessionUpdate {
            data: self,
            token_hash,
            lock: SessionLock::Wait,
        }
    }

    pub fn try_update_session<'a>(&'a self, token_hash: &'a [u8]) -> SessionUpdate<'a> {
        SessionUpdate {
            data: self,
            token_hash,
            lock: SessionLock::Skip,
        }
    }

    #[tracing::instrument(name = "data::save_user_settings", level = "debug", skip_all)]
    pub async fn save_user_settings(
        &self,
        user_id: Uuid,
        locale: &str,
        timezone: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE users
             SET locale = $1, timezone = $2, updated_at = now()
             WHERE id = $3",
        )
        .bind(locale)
        .bind(timezone)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    #[tracing::instrument(name = "data::delete_session", level = "debug", skip_all)]
    pub async fn delete_session(&self, token_hash: &[u8]) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[tracing::instrument(name = "data::delete_oidc_session", level = "debug", skip_all)]
    pub async fn delete_oidc_session(&self, issuer: &str, session_id: &str) -> Result<()> {
        sqlx::query(
            "DELETE FROM sessions
             USING users
             WHERE sessions.user_id = users.id
               AND users.issuer = $1
               AND sessions.oidc_session_id = $2",
        )
        .bind(issuer)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    #[tracing::instrument(
        name = "data::start_session_cleanup",
        level = "info",
        skip_all,
        fields(interval_hours = SESSION_CLEANUP_INTERVAL_HOURS)
    )]
    pub async fn start_session_cleanup(&self) -> Result<()> {
        self.clean_expired_sessions().await?;

        let data = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_hours(SESSION_CLEANUP_INTERVAL_HOURS)).await;
                if let Err(error) = data.clean_expired_sessions().await {
                    tracing::error!(?error, "expired session cleanup failed");
                }
            }
        });

        Ok(())
    }

    #[tracing::instrument(name = "data::clean_expired_sessions", level = "info", skip_all)]
    async fn clean_expired_sessions(&self) -> Result<()> {
        let deleted = sqlx::query("DELETE FROM sessions WHERE refresh_expires_at <= now()")
            .execute(&self.pool)
            .await?
            .rows_affected();
        if deleted > 0 {
            tracing::info!(deleted, "deleted expired sessions");
        }
        Ok(())
    }
}

impl SessionLock {
    fn name(&self) -> &'static str {
        match self {
            Self::Wait => "wait",
            Self::Skip => "skip_locked",
        }
    }
}

impl SessionUpdate<'_> {
    #[tracing::instrument(
        name = "session_update::run",
        level = "debug",
        skip_all,
        fields(lock = self.lock.name())
    )]
    pub async fn run<T, F, Fut>(self, update: F) -> Result<Option<T>>
    where
        T: Send,
        F: FnOnce(Session) -> Fut + Send,
        Fut: Future<Output = Result<(T, SessionChange)>> + Send,
    {
        let mut transaction = self.data.pool.begin().await?;
        let query = match self.lock {
            SessionLock::Wait => format!("{SESSION_QUERY} FOR UPDATE"),
            SessionLock::Skip => format!("{SESSION_QUERY} FOR UPDATE SKIP LOCKED"),
        };
        let Some(session) = sqlx::query_as::<_, Session>(&query)
            .bind(self.token_hash)
            .fetch_optional(&mut *transaction)
            .await?
        else {
            transaction.rollback().await?;
            return Ok(None);
        };
        let session_id = session.id;
        let (result, change) = update(session).await?;

        match change {
            SessionChange::Keep => {}
            SessionChange::Delete => {
                sqlx::query("DELETE FROM sessions WHERE id = $1")
                    .bind(session_id)
                    .execute(&mut *transaction)
                    .await?;
            }
            SessionChange::RetryAfter(retry_after) => {
                sqlx::query("UPDATE sessions SET refresh_retry_after = $1 WHERE id = $2")
                    .bind(retry_after)
                    .bind(session_id)
                    .execute(&mut *transaction)
                    .await?;
            }
            SessionChange::SaveTokens {
                access_token,
                refresh_token,
                refresh_expires_at,
            } => {
                sqlx::query(
                    "UPDATE sessions
                     SET access_token = $1,
                         refresh_token = $2,
                         refresh_retry_after = NULL,
                         refresh_expires_at = $3
                     WHERE id = $4",
                )
                .bind(access_token)
                .bind(refresh_token)
                .bind(refresh_expires_at)
                .bind(session_id)
                .execute(&mut *transaction)
                .await?;
            }
        }

        transaction.commit().await?;
        Ok(Some(result))
    }
}

const SESSION_CLEANUP_INTERVAL_HOURS: u64 = 24;

const SESSION_QUERY: &str = "SELECT sessions.id,
            users.id AS user_id,
            sessions.access_token,
            sessions.refresh_token,
            sessions.refresh_retry_after,
            sessions.refresh_expires_at,
            users.email,
            users.locale,
            users.timezone,
            users.issuer,
            users.subject
     FROM sessions
     JOIN users ON users.id = sessions.user_id
     WHERE sessions.token_hash = $1
       AND sessions.refresh_expires_at > now()
     LIMIT 1";
