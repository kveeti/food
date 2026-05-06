use anyhow::{Context, Result};
use chrono::{DateTime, Duration, LocalResult, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Serialize;

use crate::db::{Db, Meal, Settings};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Today {
    pub settings_required: bool,
    pub settings: Option<Settings>,
    pub current_meal: Option<Meal>,
    pub meals: Vec<Meal>,
    pub meals_started_today: i64,
    pub reminders_paused: bool,
    pub next_meal: Option<NextMeal>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MealHistory {
    pub days: Vec<MealDay>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MealDay {
    pub date: String,
    pub meals: Vec<Meal>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextMeal {
    pub due_at: String,
    pub reminder_at: String,
    pub already_sent_reminder: bool,
}

pub async fn today(db: &Db, now: DateTime<Utc>) -> Result<Today> {
    let Some(settings) = db.get_settings().await? else {
        return Ok(Today {
            settings_required: true,
            settings: None,
            current_meal: None,
            meals: Vec::new(),
            meals_started_today: 0,
            reminders_paused: false,
            next_meal: None,
        });
    };

    let tz = parse_timezone(&settings.timezone)?;
    let local_today = now.with_timezone(&tz).date_naive();
    let (day_start, day_end) = local_day_bounds(local_today, tz)?;
    let day_start = day_start.to_rfc3339();
    let day_end = day_end.to_rfc3339();
    let meals = db.list_meals_between(&day_start, &day_end).await?;
    let current_meal = db.get_open_meal().await?;
    let meals_started_today = meals.len() as i64;
    let reminders_paused = settings.reminders_paused;
    let next_meal = if reminders_paused {
        None
    } else {
        next_meal(db, &settings, &meals).await?
    };

    Ok(Today {
        settings_required: false,
        settings: Some(settings),
        current_meal,
        meals,
        meals_started_today,
        reminders_paused,
        next_meal,
    })
}

pub async fn history(db: &Db, settings: &Settings, limit: i64) -> Result<MealHistory> {
    let tz = parse_timezone(&settings.timezone)?;
    let mut days = Vec::<MealDay>::new();

    for meal in db.list_recent_meals(limit).await? {
        let started_at = DateTime::parse_from_rfc3339(&meal.started_at)
            .context("meal started_at must be RFC3339")?
            .with_timezone(&tz);
        let date = started_at.date_naive().to_string();

        if let Some(day) = days.iter_mut().find(|day| day.date == date) {
            day.meals.push(meal);
        } else {
            days.push(MealDay {
                date,
                meals: vec![meal],
            });
        }
    }

    Ok(MealHistory { days })
}

pub async fn next_meal(
    db: &Db,
    settings: &Settings,
    meals_today: &[Meal],
) -> Result<Option<NextMeal>> {
    if meals_today.is_empty() || meals_today.len() >= settings.meals.len() {
        return Ok(None);
    }

    let Some(last_meal) = meals_today.last() else {
        return Ok(None);
    };

    let started_at = DateTime::parse_from_rfc3339(&last_meal.started_at)
        .context("meal started_at must be RFC3339")?
        .to_utc();
    let due_at = started_at + Duration::minutes(settings.meal_interval_minutes);
    let reminder_at = due_at - Duration::minutes(settings.reminder_offset_minutes);
    let due_at = due_at.to_rfc3339();
    let reminder_at = reminder_at.to_rfc3339();
    let already_sent_reminder = db.already_notified("meal_due", &due_at).await?;

    Ok(Some(NextMeal {
        due_at,
        reminder_at,
        already_sent_reminder,
    }))
}

fn parse_timezone(timezone: &str) -> Result<Tz> {
    timezone
        .parse()
        .with_context(|| format!("invalid timezone: {timezone}"))
}

fn local_day_bounds(date: NaiveDate, tz: Tz) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let start = date.and_hms_opt(0, 0, 0).context("valid local day start")?;
    let next_start = date
        .succ_opt()
        .context("valid next local day")?
        .and_hms_opt(0, 0, 0)
        .context("valid next local day start")?;

    Ok((
        resolve_local(tz, start)?.to_utc(),
        resolve_local(tz, next_start)?.to_utc(),
    ))
}

fn resolve_local(tz: Tz, local: chrono::NaiveDateTime) -> Result<DateTime<Tz>> {
    match tz.from_local_datetime(&local) {
        LocalResult::Single(value) => Ok(value),
        LocalResult::Ambiguous(early, _) => Ok(early),
        LocalResult::None => anyhow::bail!("local time does not exist in timezone"),
    }
}
