use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::watch};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    config::Config,
    db::{Db, SaveSettings, Settings, Subscription},
    meal::{self, MealHistory, Today},
    notify::{self, VapidConfig},
};

mod error;
mod frontend;

use error::{ApiError, ApiResult};

#[derive(Clone)]
struct AppState {
    config: Config,
    db: Db,
}

pub async fn start(
    config: Config,
    db: Db,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let state = AppState {
        config: config.clone(),
        db,
    };

    let api_routes = Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status))
        .route("/v1/settings", get(get_settings).put(save_settings))
        .route("/v1/today", get(get_today))
        .route("/v1/meals", get(get_meals))
        .route("/v1/meals/start", post(start_meal))
        .route("/v1/meals/current/stop", post(stop_meal))
        .route("/v1/meals/{id}", delete(delete_meal))
        .route("/v1/reminders/pause", post(pause_reminders))
        .route("/v1/reminders/resume", post(resume_reminders))
        .route("/v1/push/public-key", get(push_public_key))
        .route("/v1/push/subscribe", post(subscribe_push))
        .route("/v1/push/unsubscribe", post(unsubscribe_push))
        .route("/v1/push/test", post(test_push))
        .with_state(state);

    let mut app = Router::new()
        .nest("/api", api_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    if let Some(dir) = &config.frontend_dir {
        tracing::info!("serving frontend from {dir}");
        app = app.merge(frontend::router(dir));
    }

    let listener = TcpListener::bind(&config.host).await?;
    tracing::info!("listening at {}", listener.local_addr()?);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.wait_for(|&v| v).await;
            tracing::info!("api server shutting down");
        })
        .await?;

    Ok(())
}

async fn health() -> &'static str {
    "OK"
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    ok: bool,
    settings_set: bool,
}

async fn status(State(state): State<AppState>) -> ApiResult<Json<Status>> {
    Ok(Json(Status {
        ok: true,
        settings_set: state.db.get_settings().await?.is_some(),
    }))
}

async fn get_settings(State(state): State<AppState>) -> ApiResult<Json<Option<Settings>>> {
    Ok(Json(state.db.get_settings().await?))
}

async fn save_settings(
    State(state): State<AppState>,
    Json(mut payload): Json<SaveSettings>,
) -> ApiResult<Json<Settings>> {
    payload.meals = normalize_meals(payload.meals);
    validate_settings(&payload)?;
    Ok(Json(state.db.save_settings(payload).await?))
}

async fn get_today(State(state): State<AppState>) -> ApiResult<Json<Today>> {
    Ok(Json(meal::today(&state.db, Utc::now()).await?))
}

#[derive(Deserialize)]
struct MealsQuery {
    limit: Option<i64>,
}

async fn get_meals(
    State(state): State<AppState>,
    Query(query): Query<MealsQuery>,
) -> ApiResult<Json<MealHistory>> {
    let settings = require_settings(&state.db).await?;
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    Ok(Json(meal::history(&state.db, &settings, limit).await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartMealRequest {
    meal_type: String,
}

async fn start_meal(
    State(state): State<AppState>,
    Json(payload): Json<StartMealRequest>,
) -> ApiResult<Json<Today>> {
    let settings = require_settings(&state.db).await?;
    let meal_type = payload.meal_type.trim();
    if !settings.meals.iter().any(|value| value == meal_type) {
        return Err(ApiError::BadRequest("meal_type_invalid".to_string()));
    }

    if state.db.get_open_meal().await?.is_none() {
        let now = Utc::now().to_rfc3339();
        state.db.start_meal(meal_type, &now).await?;
    }

    Ok(Json(meal::today(&state.db, Utc::now()).await?))
}

async fn stop_meal(State(state): State<AppState>) -> ApiResult<Json<Today>> {
    require_settings(&state.db).await?;
    let now = Utc::now().to_rfc3339();
    state.db.stop_open_meal(&now).await?;
    Ok(Json(meal::today(&state.db, Utc::now()).await?))
}

async fn delete_meal(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<ApiResponse>> {
    require_settings(&state.db).await?;
    state.db.delete_meal(id).await?;
    Ok(Json(ApiResponse { ok: true }))
}

async fn pause_reminders(State(state): State<AppState>) -> ApiResult<Json<Today>> {
    require_settings(&state.db).await?;
    state.db.set_reminders_paused(true).await?;
    Ok(Json(meal::today(&state.db, Utc::now()).await?))
}

async fn resume_reminders(State(state): State<AppState>) -> ApiResult<Json<Today>> {
    require_settings(&state.db).await?;
    state.db.set_reminders_paused(false).await?;
    Ok(Json(meal::today(&state.db, Utc::now()).await?))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PushPublicKey {
    public_key: String,
    configured: bool,
}

async fn push_public_key(State(state): State<AppState>) -> Json<PushPublicKey> {
    Json(PushPublicKey {
        public_key: state.config.vapid_public_key.clone(),
        configured: !state.config.vapid_public_key.is_empty()
            && !state.config.vapid_private_key.is_empty(),
    })
}

#[derive(Deserialize)]
struct SubscribeRequest {
    endpoint: String,
    p256dh: String,
    auth: String,
}

#[derive(Deserialize)]
struct UnsubscribeRequest {
    endpoint: String,
}

#[derive(Serialize)]
struct ApiResponse {
    ok: bool,
}

async fn subscribe_push(
    State(state): State<AppState>,
    Json(payload): Json<SubscribeRequest>,
) -> ApiResult<Json<ApiResponse>> {
    require_settings(&state.db).await?;

    state
        .db
        .insert_subscription(&payload.endpoint, &payload.p256dh, &payload.auth)
        .await?;

    let vapid = vapid(&state.config);
    if vapid.configured() {
        let sub = Subscription {
            endpoint: payload.endpoint,
            p256dh: payload.p256dh,
            auth: payload.auth,
        };
        let _ = notify::send_one_sub(&sub, "Food reminders enabled", &vapid).await;
    }

    Ok(Json(ApiResponse { ok: true }))
}

async fn unsubscribe_push(
    State(state): State<AppState>,
    Json(payload): Json<UnsubscribeRequest>,
) -> ApiResult<Json<ApiResponse>> {
    state.db.delete_subscription(&payload.endpoint).await?;
    Ok(Json(ApiResponse { ok: true }))
}

async fn test_push(State(state): State<AppState>) -> ApiResult<Json<ApiResponse>> {
    require_settings(&state.db).await?;

    let vapid = vapid(&state.config);
    if !vapid.configured() {
        return Err(ApiError::BadRequest("push_not_configured".to_string()));
    }

    let subscriptions = state.db.list_subscriptions().await?;
    let results = notify::send_all(&subscriptions, "Testing testing...", &vapid).await;
    let success_count = results.iter().filter(|result| result.is_ok()).count();
    tracing::info!(
        "test push sent to {}/{} subscriptions",
        success_count,
        subscriptions.len()
    );

    Ok(Json(ApiResponse { ok: true }))
}

async fn require_settings(db: &Db) -> ApiResult<Settings> {
    db.get_settings().await?.ok_or(ApiError::SettingsRequired)
}

fn validate_settings(settings: &SaveSettings) -> ApiResult<()> {
    if settings.meal_interval_minutes <= 0 {
        return Err(ApiError::BadRequest(
            "meal_interval_minutes_invalid".to_string(),
        ));
    }
    if settings.reminder_offset_minutes < 0 {
        return Err(ApiError::BadRequest(
            "reminder_offset_minutes_invalid".to_string(),
        ));
    }
    if settings.meals.is_empty() || settings.meals.iter().any(|meal| meal.trim().is_empty()) {
        return Err(ApiError::BadRequest("meals_invalid".to_string()));
    }
    settings
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| ApiError::BadRequest("timezone_invalid".to_string()))?;

    Ok(())
}

fn normalize_meals(meals: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for meal in meals {
        let meal = meal.trim().to_string();
        if !meal.is_empty() {
            normalized.push(meal);
        }
    }
    normalized
}

fn vapid(config: &Config) -> VapidConfig {
    VapidConfig {
        subject: config.vapid_subject.clone(),
        public_key_b64: config.vapid_public_key.clone(),
        private_key_b64: config.vapid_private_key.clone(),
    }
}
