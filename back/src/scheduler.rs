use chrono::{DateTime, Utc};
use tracing::{error, info};

use crate::{
    config::Config,
    db::Db,
    meal,
    notify::{self, VapidConfig},
};

pub fn spawn(db: Db, config: Config) {
    tokio::spawn(async move {
        loop {
            if let Err(err) = run_check(&db, &config).await {
                error!("scheduler error: {err}");
            }

            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });
}

async fn run_check(db: &Db, config: &Config) -> anyhow::Result<()> {
    let now = Utc::now();
    let today = meal::today(db, now).await?;
    let Some(next_meal) = today.next_meal else {
        return Ok(());
    };

    if next_meal.already_sent_reminder {
        return Ok(());
    }

    let reminder_at = parse_utc(&next_meal.reminder_at)?;
    if now < reminder_at {
        return Ok(());
    }

    let vapid = VapidConfig {
        subject: config.vapid_subject.clone(),
        public_key_b64: config.vapid_public_key.clone(),
        private_key_b64: config.vapid_private_key.clone(),
    };

    if !vapid.configured() {
        tracing::debug!("VAPID is not configured, skipping meal reminder");
        return Ok(());
    }

    let subscriptions = db.list_subscriptions().await?;
    if subscriptions.is_empty() {
        tracing::debug!("no push subscriptions, skipping meal reminder");
        return Ok(());
    }

    let due_at = parse_utc(&next_meal.due_at)?;
    let message = if now >= due_at {
        "Time to eat".to_string()
    } else {
        let minutes = today
            .settings
            .as_ref()
            .map(|settings| settings.reminder_offset_minutes)
            .unwrap_or(10);
        format!("Time to eat in {minutes} minutes")
    };

    info!(
        "sending meal reminder for {} to {} subscriptions",
        next_meal.due_at,
        subscriptions.len()
    );

    let results = notify::send_all(&subscriptions, &message, &vapid).await;
    let success_count = results.iter().filter(|result| result.is_ok()).count();
    info!(
        "meal reminder sent to {}/{} subscriptions",
        success_count,
        subscriptions.len()
    );

    db.log_notification("meal_due", &next_meal.due_at, today.meals_started_today)
        .await?;

    Ok(())
}

fn parse_utc(value: &str) -> anyhow::Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.to_utc())
}
