use std::{future::Future, time::Duration};

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
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

#[derive(sqlx::FromRow)]
pub struct FoodSearchResult {
    pub id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub source: Option<String>,
    pub energy_kcal: Option<f64>,
}

pub struct FoodPreview {
    pub id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub source: Option<String>,
    pub basis_unit: String,
    pub energy_kcal: Option<f64>,
    pub protein: Option<f64>,
    pub carbohydrate: Option<f64>,
    pub fat: Option<f64>,
    pub fibre: Option<f64>,
    pub nutrients: Vec<FoodNutrient>,
}

#[derive(sqlx::FromRow)]
pub struct FoodNutrient {
    pub code: String,
    pub name: String,
    pub unit: String,
    pub value: f64,
}

#[derive(sqlx::FromRow)]
pub struct MealChoice {
    pub latest_id: Option<Uuid>,
    pub latest_name: Option<String>,
    pub latest_is_active: bool,
    pub suggested_name: String,
}

pub struct FoodDiary {
    pub meals: Vec<DiaryMeal>,
    pub totals: DiaryNutrients,
    pub nutrients: Vec<DiaryNutrient>,
}

pub struct DiaryMeal {
    pub id: Option<Uuid>,
    pub name: Option<String>,
    pub started_at: DateTime<Utc>,
    pub energy_kcal: Option<f64>,
    pub entries: Vec<DiaryEntry>,
}

pub struct DiaryEntry {
    pub id: Uuid,
    pub name: String,
    pub brand: Option<String>,
    pub amount: f64,
    pub unit: String,
    pub energy_kcal: Option<f64>,
}

pub struct DiaryNutrients {
    pub energy_kcal: Option<f64>,
    pub protein: Option<f64>,
    pub carbohydrate: Option<f64>,
    pub fat: Option<f64>,
    pub fibre: Option<f64>,
}

#[derive(sqlx::FromRow)]
pub struct DiaryNutrient {
    pub code: String,
    pub name: String,
    pub unit: String,
    pub value: Option<f64>,
}

#[derive(sqlx::FromRow)]
struct FoodPreviewRow {
    id: Uuid,
    name: String,
    brand: Option<String>,
    source: Option<String>,
    basis_unit: String,
}

#[derive(sqlx::FromRow)]
struct DiaryEntryRow {
    id: Uuid,
    meal_id: Option<Uuid>,
    meal_name: Option<String>,
    meal_started_at: DateTime<Utc>,
    name: String,
    brand: Option<String>,
    amount: f64,
    unit: String,
    energy_kcal: Option<f64>,
    protein: Option<f64>,
    carbohydrate: Option<f64>,
    fat: Option<f64>,
    fibre: Option<f64>,
}

pub enum MealAction<'a> {
    Continue(Uuid),
    Start(&'a str),
}

pub struct NewFoodEntry<'a> {
    pub user_id: Uuid,
    pub food_id: Uuid,
    pub amount: f64,
    pub date: NaiveDate,
    pub timezone: &'a str,
    pub meal: MealAction<'a>,
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

    #[tracing::instrument(name = "data::water_total", level = "debug", skip_all)]
    pub async fn water_total(&self, user_id: Uuid, date: NaiveDate, timezone: &str) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_ml), 0)::bigint
             FROM water_entries
             WHERE user_id = $1
               AND consumed_at >= ($2::date::timestamp AT TIME ZONE $3)
               AND consumed_at < (($2::date + 1)::timestamp AT TIME ZONE $3)",
        )
        .bind(user_id)
        .bind(date)
        .bind(timezone)
        .fetch_one(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::add_water", level = "debug", skip_all)]
    pub async fn add_water(
        &self,
        user_id: Uuid,
        amount_ml: i32,
        date: NaiveDate,
        timezone: &str,
    ) -> Result<i64> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO water_entries (id, user_id, amount_ml, consumed_at)
             VALUES (
                 $1,
                 $2,
                 $3,
                 (($4::date + (now() AT TIME ZONE $5)::time)::timestamp AT TIME ZONE $5)
             )",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(amount_ml)
        .bind(date)
        .bind(timezone)
        .execute(&mut *transaction)
        .await?;
        let total = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_ml), 0)::bigint
             FROM water_entries
             WHERE user_id = $1
               AND consumed_at >= ($2::date::timestamp AT TIME ZONE $3)
               AND consumed_at < (($2::date + 1)::timestamp AT TIME ZONE $3)",
        )
        .bind(user_id)
        .bind(date)
        .bind(timezone)
        .fetch_one(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(total)
    }

    #[tracing::instrument(name = "data::search_foods", level = "debug", skip_all)]
    pub async fn search_foods(&self, user_id: Uuid, query: &str) -> Result<Vec<FoodSearchResult>> {
        if query
            .chars()
            .filter(|character| character.is_alphanumeric())
            .take(2)
            .count()
            < 2
        {
            return Ok(Vec::new());
        }
        Ok(sqlx::query_as::<_, FoodSearchResult>(FOOD_SEARCH_QUERY)
            .bind(user_id)
            .bind(query)
            .fetch_all(&self.pool)
            .await?)
    }

    #[tracing::instrument(name = "data::food_preview", level = "debug", skip_all)]
    pub async fn food_preview(&self, user_id: Uuid, food_id: Uuid) -> Result<Option<FoodPreview>> {
        let food = sqlx::query_as::<_, FoodPreviewRow>(
            "SELECT id, display_name AS name, brand, source, basis_unit
             FROM foods
             WHERE id = $1
               AND NOT is_archived
               AND (owner_user_id IS NULL OR owner_user_id = $2)",
        )
        .bind(food_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(food) = food else {
            return Ok(None);
        };
        let nutrients = sqlx::query_as::<_, FoodNutrient>(
            "SELECT nutrients.code, nutrients.display_name AS name,
                    nutrients.display_unit AS unit,
                    values.value * nutrients.display_scale AS value
             FROM food_nutrients values
             JOIN nutrients ON nutrients.id = values.nutrient_id
             WHERE values.food_id = $1 AND NOT nutrients.is_archived
             ORDER BY nutrients.display_order",
        )
        .bind(food_id)
        .fetch_all(&self.pool)
        .await?;
        let value = |code| {
            nutrients
                .iter()
                .find(|nutrient| nutrient.code == code)
                .map(|nutrient| nutrient.value)
        };
        Ok(Some(FoodPreview {
            id: food.id,
            name: food.name,
            brand: food.brand,
            source: food.source,
            basis_unit: food.basis_unit,
            energy_kcal: value("energy"),
            protein: value("protein"),
            carbohydrate: value("carbohydrate"),
            fat: value("fat"),
            fibre: value("fibre"),
            nutrients,
        }))
    }

    #[tracing::instrument(name = "data::meal_choice", level = "debug", skip_all)]
    pub async fn meal_choice(
        &self,
        user_id: Uuid,
        date: NaiveDate,
        timezone: &str,
    ) -> Result<MealChoice> {
        Ok(sqlx::query_as::<_, MealChoice>(MEAL_CHOICE_QUERY)
            .bind(user_id)
            .bind(date)
            .bind(timezone)
            .fetch_one(&self.pool)
            .await?)
    }

    #[tracing::instrument(name = "data::food_diary", level = "debug", skip_all)]
    pub async fn food_diary(
        &self,
        user_id: Uuid,
        date: NaiveDate,
        timezone: &str,
    ) -> Result<FoodDiary> {
        let rows = sqlx::query_as::<_, DiaryEntryRow>(
            "SELECT entries.id, entries.meal_id, meals.name AS meal_name,
                    COALESCE(meals.started_at, entries.eaten_at) AS meal_started_at,
                    entries.food_name AS name, entries.food_brand AS brand,
                    entries.amount, entries.unit,
                    max(snapshots.basis_value * entries.amount / 100 / 4.184)
                        FILTER (WHERE nutrients.code = 'energy') AS energy_kcal,
                    max(snapshots.basis_value * entries.amount / 100)
                        FILTER (WHERE nutrients.code = 'protein') AS protein,
                    max(snapshots.basis_value * entries.amount / 100)
                        FILTER (WHERE nutrients.code = 'carbohydrate') AS carbohydrate,
                    max(snapshots.basis_value * entries.amount / 100)
                        FILTER (WHERE nutrients.code = 'fat') AS fat,
                    max(snapshots.basis_value * entries.amount / 100)
                        FILTER (WHERE nutrients.code = 'fibre') AS fibre
             FROM food_entries entries
             LEFT JOIN meals ON meals.id = entries.meal_id
             LEFT JOIN food_entry_nutrients snapshots ON snapshots.food_entry_id = entries.id
             LEFT JOIN nutrients ON nutrients.id = snapshots.nutrient_id
             WHERE entries.user_id = $1
               AND entries.eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
               AND entries.eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
             GROUP BY entries.id, meals.id
             ORDER BY COALESCE(meals.started_at, entries.eaten_at) DESC,
                      entries.eaten_at DESC",
        )
        .bind(user_id)
        .bind(date)
        .bind(timezone)
        .fetch_all(&self.pool)
        .await?;
        let nutrients = sqlx::query_as::<_, DiaryNutrient>(
            "WITH day_entries AS (
                 SELECT id, amount
                 FROM food_entries
                 WHERE user_id = $1
                   AND eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
                   AND eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
             )
             SELECT nutrients.code, nutrients.display_name AS name,
                    nutrients.display_unit AS unit,
                    CASE WHEN count(*) = (SELECT count(*) FROM day_entries)
                         THEN sum(snapshots.basis_value * day_entries.amount / 100)
                              * nutrients.display_scale
                    END AS value
             FROM day_entries
             JOIN food_entry_nutrients snapshots ON snapshots.food_entry_id = day_entries.id
             JOIN nutrients ON nutrients.id = snapshots.nutrient_id
             WHERE NOT nutrients.is_archived
             GROUP BY nutrients.id
             ORDER BY nutrients.display_order",
        )
        .bind(user_id)
        .bind(date)
        .bind(timezone)
        .fetch_all(&self.pool)
        .await?;
        let mut diary = FoodDiary::from_rows(rows);
        diary.nutrients = nutrients;
        Ok(diary)
    }

    #[tracing::instrument(name = "data::add_food_entry", level = "debug", skip_all)]
    pub async fn add_food_entry(&self, entry: NewFoodEntry<'_>) -> Result<bool> {
        let NewFoodEntry {
            user_id,
            food_id,
            amount,
            date,
            timezone,
            meal,
        } = entry;
        let mut transaction = self.pool.begin().await?;
        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *transaction)
            .await?;

        let food = sqlx::query_as::<
            _,
            (
                String,
                Option<String>,
                String,
                Option<String>,
                Option<String>,
            ),
        >(
            "SELECT display_name, brand, basis_unit, source, source_id
             FROM foods
             WHERE id = $1
               AND NOT is_archived
               AND (owner_user_id IS NULL OR owner_user_id = $2)
             FOR SHARE",
        )
        .bind(food_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some((name, brand, unit, source, source_id)) = food else {
            transaction.rollback().await?;
            return Ok(false);
        };

        let eaten_at = sqlx::query_scalar::<_, DateTime<Utc>>(
            "SELECT (($1::date + (now() AT TIME ZONE $2)::time)::timestamp AT TIME ZONE $2)",
        )
        .bind(date)
        .bind(timezone)
        .fetch_one(&mut *transaction)
        .await?;

        let (meal_id, meal_name) = match meal {
            MealAction::Start(name) => (None, Some(name.to_owned())),
            MealAction::Continue(selected_id) => {
                let selected = sqlx::query_as::<_, (Option<String>, bool)>(
                    "SELECT selected.name,
                            COALESCE(selected.id = (
                                SELECT entries.meal_id
                                FROM food_entries entries
                                WHERE entries.user_id = $1
                                  AND entries.meal_id IS NOT NULL
                                  AND entries.eaten_at <= $3
                                ORDER BY entries.eaten_at DESC, entries.id DESC
                                LIMIT 1
                            ), false) AS is_latest
                     FROM meals selected
                     WHERE selected.user_id = $1 AND selected.id = $2",
                )
                .bind(user_id)
                .bind(selected_id)
                .bind(eaten_at)
                .fetch_optional(&mut *transaction)
                .await?;
                let Some((name, is_latest)) = selected else {
                    transaction.rollback().await?;
                    return Ok(false);
                };
                (is_latest.then_some(selected_id), name)
            }
        };
        let meal_id = match meal_id {
            Some(meal_id) => meal_id,
            None => {
                let meal_id = Uuid::now_v7();
                sqlx::query(
                    "INSERT INTO meals (id, user_id, name, started_at)
                     VALUES ($1, $2, $3, $4)",
                )
                .bind(meal_id)
                .bind(user_id)
                .bind(meal_name)
                .bind(eaten_at)
                .execute(&mut *transaction)
                .await?;
                meal_id
            }
        };

        let entry_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO food_entries (
                 id, user_id, meal_id, food_id, amount, unit, eaten_at,
                 food_name, food_brand, food_source, food_source_id
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(entry_id)
        .bind(user_id)
        .bind(meal_id)
        .bind(food_id)
        .bind(amount)
        .bind(unit)
        .bind(eaten_at)
        .bind(name)
        .bind(brand)
        .bind(source)
        .bind(source_id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO food_entry_nutrients (food_entry_id, nutrient_id, basis_value)
             SELECT $1, nutrient_id, value
             FROM food_nutrients
             WHERE food_id = $2",
        )
        .bind(entry_id)
        .bind(food_id)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(true)
    }

    #[tracing::instrument(name = "data::update_food_entry", level = "debug", skip_all)]
    pub async fn update_food_entry(
        &self,
        user_id: Uuid,
        entry_id: Uuid,
        amount: f64,
    ) -> Result<bool> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *transaction)
            .await?;
        let updated = sqlx::query(
            "UPDATE food_entries
             SET amount = $1
             WHERE id = $2 AND user_id = $3",
        )
        .bind(amount)
        .bind(entry_id)
        .bind(user_id)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        transaction.commit().await?;
        Ok(updated == 1)
    }

    #[tracing::instrument(name = "data::delete_food_entry", level = "debug", skip_all)]
    pub async fn delete_food_entry(&self, user_id: Uuid, entry_id: Uuid) -> Result<bool> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *transaction)
            .await?;
        let meal_id = sqlx::query_scalar::<_, Option<Uuid>>(
            "DELETE FROM food_entries
             WHERE id = $1 AND user_id = $2
             RETURNING meal_id",
        )
        .bind(entry_id)
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(meal_id) = meal_id else {
            transaction.rollback().await?;
            return Ok(false);
        };
        if let Some(meal_id) = meal_id {
            sqlx::query(
                "DELETE FROM meals
                 WHERE id = $1 AND user_id = $2
                   AND NOT EXISTS (
                       SELECT 1 FROM food_entries WHERE meal_id = meals.id
                   )",
            )
            .bind(meal_id)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(true)
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

impl FoodDiary {
    fn from_rows(rows: Vec<DiaryEntryRow>) -> Self {
        let mut diary = Self {
            meals: Vec::new(),
            totals: DiaryNutrients::zero(),
            nutrients: Vec::new(),
        };
        for row in rows {
            diary.totals.add(&row);
            let needs_meal = diary.meals.last().is_none_or(|meal| meal.id != row.meal_id);
            if needs_meal {
                diary.meals.push(DiaryMeal {
                    id: row.meal_id,
                    name: row.meal_name.clone(),
                    started_at: row.meal_started_at,
                    energy_kcal: Some(0.0),
                    entries: Vec::new(),
                });
            }
            let meal = diary.meals.last_mut().expect("a diary meal was just added");
            add_nutrient(&mut meal.energy_kcal, row.energy_kcal);
            meal.entries.push(DiaryEntry {
                id: row.id,
                name: row.name,
                brand: row.brand,
                amount: row.amount,
                unit: row.unit,
                energy_kcal: row.energy_kcal,
            });
        }
        diary
    }
}

impl DiaryNutrients {
    fn zero() -> Self {
        Self {
            energy_kcal: Some(0.0),
            protein: Some(0.0),
            carbohydrate: Some(0.0),
            fat: Some(0.0),
            fibre: Some(0.0),
        }
    }

    fn add(&mut self, row: &DiaryEntryRow) {
        add_nutrient(&mut self.energy_kcal, row.energy_kcal);
        add_nutrient(&mut self.protein, row.protein);
        add_nutrient(&mut self.carbohydrate, row.carbohydrate);
        add_nutrient(&mut self.fat, row.fat);
        add_nutrient(&mut self.fibre, row.fibre);
    }
}

fn add_nutrient(total: &mut Option<f64>, value: Option<f64>) {
    *total = match (*total, value) {
        (Some(total), Some(value)) => Some(total + value),
        _ => None,
    };
}

const SESSION_CLEANUP_INTERVAL_HOURS: u64 = 24;

const MEAL_CHOICE_QUERY: &str = "WITH candidate AS (
         SELECT (($2::date + (now() AT TIME ZONE $3)::time)::timestamp AT TIME ZONE $3) AS moment,
                ($2::date + (now() AT TIME ZONE $3)::time)::timestamp AS local_moment
     ), latest AS (
         SELECT meals.id, COALESCE(meals.name, 'Meal') AS name, entries.eaten_at
         FROM food_entries entries
         JOIN meals ON meals.id = entries.meal_id
         CROSS JOIN candidate
         WHERE entries.user_id = $1
           AND entries.eaten_at <= candidate.moment
         ORDER BY entries.eaten_at DESC, entries.id DESC
         LIMIT 1
     ), history AS (
         SELECT initcap(lower(meals.name)) AS name,
                abs(extract(epoch FROM (
                    (meals.started_at AT TIME ZONE $3)::time
                    - candidate.local_moment::time
                ))) / 3600 AS time_distance,
                extract(epoch FROM (candidate.moment - meals.started_at)) / 86400 AS age_days,
                (extract(isodow FROM meals.started_at AT TIME ZONE $3) >= 6)
                    = (extract(isodow FROM candidate.local_moment) >= 6) AS same_day_kind
         FROM meals
         CROSS JOIN candidate
         WHERE meals.user_id = $1
           AND lower(meals.name) IN ('breakfast', 'lunch', 'snack', 'dinner')
           AND meals.started_at < candidate.moment - interval '2 hours'
           AND meals.started_at >= candidate.moment - interval '180 days'
     ), predicted AS (
         SELECT name
         FROM history
         GROUP BY name
         ORDER BY sum(
             (1 / (1 + power(time_distance, 2)))
             * (CASE WHEN same_day_kind THEN 1.25 ELSE 1 END)
             * (1 / (1 + age_days / 90))
         ) DESC,
         count(*) DESC,
         name
         LIMIT 1
     )
     SELECT (SELECT id FROM latest) AS latest_id,
            (SELECT name FROM latest) AS latest_name,
            COALESCE(
                (SELECT eaten_at >= candidate.moment - interval '2 hours' FROM latest),
                false
            ) AS latest_is_active,
            COALESCE(
                (SELECT name FROM predicted),
                CASE
                    WHEN extract(hour FROM local_moment) BETWEEN 5 AND 10 THEN 'Breakfast'
                    WHEN extract(hour FROM local_moment) BETWEEN 11 AND 14 THEN 'Lunch'
                    WHEN extract(hour FROM local_moment) BETWEEN 17 AND 21 THEN 'Dinner'
                    ELSE 'Snack'
                END
            ) AS suggested_name
     FROM candidate";

const FOOD_SEARCH_QUERY: &str = "WITH normalized AS (
         SELECT trim(regexp_replace(lower($2), '[^[:alnum:]]+', ' ', 'g')) AS query
     ), input AS (
         SELECT query,
                CASE
                    WHEN char_length(replace(query, ' ', '')) >= 2 THEN
                        to_tsquery('simple', (
                            SELECT string_agg(term || ':*', ' & ' ORDER BY position)
                            FROM unnest(regexp_split_to_array(query, ' '))
                                 WITH ORDINALITY AS terms(term, position)
                            WHERE term <> ''
                        ))
                END AS prefix_query
         FROM normalized
     ), candidates AS (
         SELECT foods.id, foods.display_name, foods.brand, foods.source,
                foods.source_id, foods.source_data,
                foods.search_vector, input.query, input.prefix_query,
                trim(regexp_replace(lower(foods.display_name), '[^[:alnum:]]+', ' ', 'g')) AS normalized_name,
                trim(regexp_replace(lower(split_part(foods.display_name, ',', 1)), '[^[:alnum:]]+', ' ', 'g')) AS primary_name,
                trim(regexp_replace(lower(COALESCE(foods.brand, '')), '[^[:alnum:]]+', ' ', 'g')) AS normalized_brand,
                energy.value / 4.184 AS energy_kcal
         FROM foods
         CROSS JOIN input
         LEFT JOIN nutrients energy_name ON energy_name.code = 'energy'
         LEFT JOIN food_nutrients energy
                ON energy.food_id = foods.id AND energy.nutrient_id = energy_name.id
         WHERE NOT foods.is_archived
           AND (foods.owner_user_id IS NULL OR foods.owner_user_id = $1)
           AND (
               foods.source_id = trim($2)
               OR (input.prefix_query IS NOT NULL AND foods.search_vector @@ input.prefix_query)
               OR foods.search_fi_vector @@ plainto_tsquery('finnish', $2)
               OR foods.search_sv_vector @@ plainto_tsquery('swedish', $2)
               OR foods.search_en_vector @@ plainto_tsquery('english', $2)
           )
     ), ranked AS (
         SELECT candidates.*,
                CASE
                    WHEN source_id = trim($2) THEN 0
                    WHEN normalized_name = query THEN 1
                    WHEN primary_name = query OR normalized_brand = query THEN 2
                    WHEN normalized_name LIKE query || '%' THEN 3
                    ELSE 4
                END AS match_class,
                ts_rank_cd(search_vector, prefix_query) AS relevance
         FROM candidates
     )
     SELECT id, display_name AS name, brand, source, energy_kcal
     FROM ranked
     ORDER BY
         CASE WHEN source_id = trim($2) THEN 0 ELSE 1 END,
         CASE source WHEN 'fineli' THEN 1 WHEN 'open_food_facts' THEN 2 ELSE 0 END,
         match_class,
         CASE WHEN match_class > 2 AND source_data->>'food_type' = 'DISH' THEN 1 ELSE 0 END,
         CASE WHEN lower(display_name) LIKE '%, keskiarvo%' THEN 0 ELSE 1 END,
         CASE COALESCE(source_data->>'process', '')
             WHEN 'RAW' THEN 0
             WHEN 'IND' THEN 1
             WHEN '' THEN 1
             ELSE 2
         END,
         relevance DESC,
         char_length(display_name),
         display_name
     LIMIT 10";

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
