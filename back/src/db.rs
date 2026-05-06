use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, sqlite::SqlitePoolOptions};

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        let db = Self { pool };
        sqlx::migrate!("./migrations").run(&db.pool).await?;
        Ok(db)
    }

    pub async fn get_settings(&self) -> Result<Option<Settings>> {
        Ok(sqlx::query_as::<_, Settings>(
            "SELECT meal_interval_minutes, reminder_offset_minutes, meals, reminders_paused, timezone, updated_at
             FROM settings
             WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn save_settings(&self, settings: SaveSettings) -> Result<Settings> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO settings (
                id,
                meal_interval_minutes,
                reminder_offset_minutes,
                meals,
                reminders_paused,
                timezone,
                updated_at
             )
             VALUES (1, ?, ?, ?, 0, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                meal_interval_minutes = excluded.meal_interval_minutes,
                reminder_offset_minutes = excluded.reminder_offset_minutes,
                meals = excluded.meals,
                timezone = excluded.timezone,
                updated_at = excluded.updated_at",
        )
        .bind(settings.meal_interval_minutes)
        .bind(settings.reminder_offset_minutes)
        .bind(serde_json::to_string(&settings.meals)?)
        .bind(settings.timezone)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(self
            .get_settings()
            .await?
            .expect("settings row exists after save"))
    }

    pub async fn set_reminders_paused(&self, paused: bool) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE settings SET reminders_paused = ?, updated_at = ? WHERE id = 1")
            .bind(paused)
            .bind(now)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_open_meal(&self) -> Result<Option<Meal>> {
        Ok(sqlx::query_as::<_, Meal>(
            "SELECT id, meal_type, started_at, ended_at, note, created_at, updated_at
             FROM meals
             WHERE ended_at IS NULL
             ORDER BY started_at DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn start_meal(&self, meal_type: &str, now: &str) -> Result<Meal> {
        Ok(sqlx::query_as::<_, Meal>(
            "INSERT INTO meals (meal_type, started_at, ended_at, note, created_at, updated_at)
             VALUES (?, ?, NULL, NULL, ?, ?)
             RETURNING id, meal_type, started_at, ended_at, note, created_at, updated_at",
        )
        .bind(meal_type)
        .bind(now)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn stop_open_meal(&self, now: &str) -> Result<Option<Meal>> {
        let Some(open) = self.get_open_meal().await? else {
            return Ok(None);
        };

        sqlx::query("UPDATE meals SET ended_at = ?, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(now)
            .bind(open.id)
            .execute(&self.pool)
            .await?;

        Ok(Some(self.get_meal(open.id).await?))
    }

    pub async fn get_meal(&self, id: i64) -> Result<Meal> {
        Ok(sqlx::query_as::<_, Meal>(
            "SELECT id, meal_type, started_at, ended_at, note, created_at, updated_at
             FROM meals
             WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn list_meals_between(&self, from: &str, to: &str) -> Result<Vec<Meal>> {
        Ok(sqlx::query_as::<_, Meal>(
            "SELECT id, meal_type, started_at, ended_at, note, created_at, updated_at
             FROM meals
             WHERE started_at >= ? AND started_at < ?
             ORDER BY started_at",
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn list_recent_meals(&self, limit: i64) -> Result<Vec<Meal>> {
        Ok(sqlx::query_as::<_, Meal>(
            "SELECT id, meal_type, started_at, ended_at, note, created_at, updated_at
             FROM meals
             ORDER BY started_at DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn delete_meal(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM meals WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn insert_subscription(
        &self,
        endpoint: &str,
        p256dh: &str,
        auth: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO push_subscriptions (endpoint, p256dh, auth, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(endpoint) DO UPDATE SET
                p256dh = excluded.p256dh,
                auth = excluded.auth,
                updated_at = excluded.updated_at",
        )
        .bind(endpoint)
        .bind(p256dh)
        .bind(auth)
        .bind(now.clone())
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_subscription(&self, endpoint: &str) -> Result<()> {
        sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = ?")
            .bind(endpoint)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn list_subscriptions(&self) -> Result<Vec<Subscription>> {
        Ok(sqlx::query_as::<_, Subscription>(
            "SELECT endpoint, p256dh, auth FROM push_subscriptions",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn already_notified(&self, kind: &str, target_at: &str) -> Result<bool> {
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM notification_log WHERE kind = ? AND target_at = ?")
                .bind(kind)
                .bind(target_at)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.is_some())
    }

    pub async fn log_notification(
        &self,
        kind: &str,
        target_at: &str,
        meals_started_for_day: i64,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR IGNORE INTO notification_log
             (kind, target_at, sent_at, meals_started_for_day)
             VALUES (?, ?, ?, ?)",
        )
        .bind(kind)
        .bind(target_at)
        .bind(now)
        .bind(meals_started_for_day)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub meal_interval_minutes: i64,
    pub reminder_offset_minutes: i64,
    pub meals: sqlx::types::Json<Vec<String>>,
    pub reminders_paused: bool,
    pub timezone: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettings {
    pub meal_interval_minutes: i64,
    pub reminder_offset_minutes: i64,
    pub meals: Vec<String>,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Meal {
    pub id: i64,
    pub meal_type: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct Subscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}
