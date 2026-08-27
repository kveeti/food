use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use flate2::read::MultiGzDecoder;
use futures_util::StreamExt;
use sqlx::PgPool;
use tokio::{fs as async_fs, io::AsyncWriteExt, time::sleep};
use url::Url;
use uuid::Uuid;

use super::{ImportConfig, delta_range, queue_sync, validate_http_url};
use version3::imports::{
    database::{write_delta, write_import},
    fineli::read_fineli,
    open_food_facts::{read_open_food_facts, read_open_food_facts_delta},
};

#[derive(sqlx::FromRow)]
struct ClaimedJob {
    id: Uuid,
    kind: String,
    file_path: Option<String>,
    delta_start: Option<i64>,
    delta_end: Option<i64>,
}

pub(super) async fn run(pool: PgPool, config: ImportConfig) -> std::result::Result<(), String> {
    let mut lock = pool.acquire().await.map_err(|error| error.to_string())?;
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(661014603109)")
        .fetch_one(&mut *lock)
        .await
        .map_err(|error| error.to_string())?;
    if !acquired {
        return Ok(());
    }
    sqlx::query(
        "UPDATE import_jobs
         SET status = 'queued', started_at = NULL,
             error = 'retrying after app stopped', finished_at = NULL
         WHERE status = 'running'",
    )
    .execute(&pool)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query(
        "UPDATE off_sync_state
         SET last_full_at = now(), updated_at = now()
         WHERE singleton AND last_full_at IS NULL
           AND EXISTS (
               SELECT 1 FROM foods
               WHERE source = 'open_food_facts' AND NOT is_archived
           )",
    )
    .execute(&pool)
    .await
    .map_err(|error| error.to_string())?;

    loop {
        if config.automatic
            && let Err(error) = queue_due_jobs(&pool).await
        {
            eprintln!("failed to schedule imports: {error}");
        }
        let job = claim_job(&pool).await.map_err(|error| error.to_string())?;
        if let Some(job) = job {
            let result = process_job(&pool, &config, &job).await;
            let (status, details, error) = match result {
                Ok(details) => ("succeeded", Some(details), None),
                Err(error) => ("failed", None, Some(error)),
            };
            sqlx::query(
                "UPDATE import_jobs
                 SET status = $2, details = $3, error = $4, finished_at = now()
                 WHERE id = $1",
            )
            .bind(job.id)
            .bind(status)
            .bind(details)
            .bind(error)
            .execute(&pool)
            .await
            .map_err(|error| error.to_string())?;
            if let Some(path) = job.file_path {
                let _ = async_fs::remove_file(path).await;
            }
        } else {
            sleep(Duration::from_secs(15)).await;
        }
    }
}

async fn claim_job(pool: &PgPool) -> sqlx::Result<Option<ClaimedJob>> {
    sqlx::query_as(
        "WITH next AS (
             SELECT id FROM import_jobs
             WHERE status = 'queued'
             ORDER BY created_at
             FOR UPDATE SKIP LOCKED
             LIMIT 1
         )
         UPDATE import_jobs jobs
         SET status = 'running', started_at = now(), error = NULL
         FROM next
         WHERE jobs.id = next.id
         RETURNING jobs.id, jobs.kind, jobs.file_path,
                   jobs.delta_start, jobs.delta_end",
    )
    .fetch_optional(pool)
    .await
}

async fn queue_due_jobs(pool: &PgPool) -> sqlx::Result<()> {
    let full_due: bool = sqlx::query_scalar(
        "SELECT (last_full_at IS NULL OR last_full_at < now() - interval '365 days')
           AND NOT EXISTS (
               SELECT 1 FROM import_jobs
               WHERE kind = 'off_full_sync' AND created_at > now() - interval '1 day'
           )
         FROM off_sync_state WHERE singleton",
    )
    .fetch_one(pool)
    .await?;
    if full_due {
        queue_sync(pool, "off_full_sync", None).await?;
    }
    let delta_due: bool = sqlx::query_scalar(
        "SELECT NOT EXISTS (
             SELECT 1 FROM import_jobs
             WHERE kind IN ('off_delta_sync', 'off_delta_upload')
               AND status = 'succeeded' AND finished_at > now() - interval '7 days'
         ) AND NOT EXISTS (
             SELECT 1 FROM import_jobs
             WHERE kind = 'off_delta_sync' AND created_at > now() - interval '1 day'
         )",
    )
    .fetch_one(pool)
    .await?;
    if delta_due {
        queue_sync(pool, "off_delta_sync", None).await?;
    }
    Ok(())
}

async fn process_job(
    pool: &PgPool,
    config: &ImportConfig,
    job: &ClaimedJob,
) -> std::result::Result<String, String> {
    match job.kind.as_str() {
        "fineli_upload" => import_fineli(pool, required_path(job)?.to_owned()).await,
        "off_full_upload" => import_off_full(pool, required_path(job)?.to_owned()).await,
        "off_delta_upload" => {
            import_off_delta(
                pool,
                required_path(job)?.to_owned(),
                job.delta_start.ok_or("delta start is missing")?,
                job.delta_end.ok_or("delta end is missing")?,
            )
            .await
        }
        "off_full_sync" => {
            let url: String =
                sqlx::query_scalar("SELECT off_full_url FROM import_settings WHERE singleton")
                    .fetch_one(pool)
                    .await
                    .map_err(|error| error.to_string())?;
            import_off_full_url(pool, url).await
        }
        "off_delta_sync" => sync_deltas(pool, config).await,
        _ => Err("unknown import job".to_owned()),
    }
}

fn required_path(job: &ClaimedJob) -> std::result::Result<&Path, String> {
    job.file_path
        .as_deref()
        .map(Path::new)
        .ok_or_else(|| "import file is missing".to_owned())
}

async fn import_fineli(pool: &PgPool, path: PathBuf) -> std::result::Result<String, String> {
    let directory = path.with_extension("files");
    let parse_directory = directory.clone();
    let parsed = tokio::task::spawn_blocking(move || {
        extract_zip(&path, &parse_directory)?;
        read_fineli(&parse_directory).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?;
    let _ = async_fs::remove_dir_all(&directory).await;
    let import = parsed?;
    let foods = import.foods.len();
    let values = import
        .foods
        .iter()
        .map(|food| food.nutrients.len())
        .sum::<usize>();
    let mut connection = pool.acquire().await.map_err(|error| error.to_string())?;
    write_import(&mut connection, import)
        .await
        .map_err(|error| error.to_string())?;
    Ok(format!("{foods} foods, {values} nutrient values"))
}

async fn import_off_full(pool: &PgPool, path: PathBuf) -> std::result::Result<String, String> {
    let import = tokio::task::spawn_blocking(move || {
        let file = File::open(path).map_err(|error| error.to_string())?;
        read_open_food_facts(MultiGzDecoder::new(file)).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;
    finish_off_full(pool, import).await
}

async fn import_off_full_url(pool: &PgPool, url: String) -> std::result::Result<String, String> {
    let import = tokio::task::spawn_blocking(move || {
        let http = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .timeout(Duration::from_secs(60 * 60 * 6))
            .build()
            .map_err(|error| error.to_string())?;
        let response = http
            .get(url)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| error.to_string())?;
        if response
            .content_length()
            .is_some_and(|size| size > 20 * 1024 * 1024 * 1024)
        {
            return Err("OFF full download is too large".to_owned());
        }
        read_open_food_facts(MultiGzDecoder::new(response)).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;
    finish_off_full(pool, import).await
}

async fn finish_off_full(
    pool: &PgPool,
    import: version3::imports::model::FoodImport,
) -> std::result::Result<String, String> {
    let foods = import.foods.len();
    let values = import
        .foods
        .iter()
        .map(|food| food.nutrients.len())
        .sum::<usize>();
    let mut connection = pool.acquire().await.map_err(|error| error.to_string())?;
    write_import(&mut connection, import)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query(
        "UPDATE off_sync_state
         SET last_full_at = now(), last_delta_end = NULL, updated_at = now()
         WHERE singleton",
    )
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    queue_sync(pool, "off_delta_sync", None)
        .await
        .map_err(|error| error.to_string())?;
    Ok(format!("{foods} foods, {values} nutrient values"))
}

async fn import_off_delta(
    pool: &PgPool,
    path: PathBuf,
    start: i64,
    end: i64,
) -> std::result::Result<String, String> {
    let cursor: Option<i64> =
        sqlx::query_scalar("SELECT last_delta_end FROM off_sync_state WHERE singleton")
            .fetch_one(pool)
            .await
            .map_err(|error| error.to_string())?;
    if cursor.is_some_and(|cursor| end <= cursor) {
        return Ok("already applied".to_owned());
    }
    if cursor.is_some_and(|cursor| start > cursor) {
        return Err("delta gap detected; run a full import".to_owned());
    }

    let delta = tokio::task::spawn_blocking(move || {
        let file = File::open(path).map_err(|error| error.to_string())?;
        read_open_food_facts_delta(MultiGzDecoder::new(file)).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;
    let changed = delta.changed_source_ids.len();
    let imported = delta.import.foods.len();
    let mut connection = pool.acquire().await.map_err(|error| error.to_string())?;
    write_delta(&mut connection, delta)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query(
        "UPDATE off_sync_state
         SET last_delta_end = GREATEST(COALESCE(last_delta_end, 0), $1), updated_at = now()
         WHERE singleton",
    )
    .bind(end)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(format!("{changed} changed products, {imported} imported"))
}

async fn sync_deltas(pool: &PgPool, config: &ImportConfig) -> std::result::Result<String, String> {
    let (index_url, full_at): (String, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        "SELECT settings.off_delta_index_url, state.last_full_at
         FROM import_settings settings CROSS JOIN off_sync_state state
         WHERE settings.singleton AND state.singleton",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let index = config
        .http
        .get(&index_url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?
        .text()
        .await
        .map_err(|error| error.to_string())?;
    let mut files = index
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|name| {
            delta_range(name)
                .map(|range| (range.0, range.1, name.to_owned()))
                .map_err(|error| error.to_string())
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    files.sort_by_key(|file| file.0);
    let cursor: Option<i64> =
        sqlx::query_scalar("SELECT last_delta_end FROM off_sync_state WHERE singleton")
            .fetch_one(pool)
            .await
            .map_err(|error| error.to_string())?;
    if cursor.is_none() && full_at.is_none() {
        queue_sync(pool, "off_full_sync", None)
            .await
            .map_err(|error| error.to_string())?;
        return Ok("full import required".to_owned());
    }
    if let (Some(cursor), Some(first)) = (cursor, files.first())
        && cursor < first.0
    {
        queue_sync(pool, "off_full_sync", None)
            .await
            .map_err(|error| error.to_string())?;
        return Err("delta history has a gap; queued a full import".to_owned());
    }

    let base = Url::parse(&index_url).map_err(|error| error.to_string())?;
    let mut applied = 0;
    for (start, end, name) in files {
        if cursor.is_some_and(|cursor| end <= cursor) {
            continue;
        }
        let url = base.join(&name).map_err(|error| error.to_string())?;
        let path = download(config, url.as_str(), 1024 * 1024 * 1024).await?;
        let result = import_off_delta(pool, path.clone(), start, end).await;
        let _ = async_fs::remove_file(path).await;
        result?;
        applied += 1;
    }
    Ok(format!("applied {applied} delta files"))
}

async fn download(
    config: &ImportConfig,
    url: &str,
    maximum: u64,
) -> std::result::Result<PathBuf, String> {
    validate_http_url(url).map_err(|error| error.to_string())?;
    let response = config
        .http
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?;
    if response.content_length().is_some_and(|size| size > maximum) {
        return Err("import download is too large".to_owned());
    }
    let path = config.directory.join(Uuid::now_v7().to_string());
    let result = async {
        let mut output = async_fs::File::create(&path)
            .await
            .map_err(|error| error.to_string())?;
        let mut stream = response.bytes_stream();
        let mut size = 0_u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| error.to_string())?;
            size += chunk.len() as u64;
            if size > maximum {
                return Err("import download is too large".to_owned());
            }
            output
                .write_all(&chunk)
                .await
                .map_err(|error| error.to_string())?;
        }
        output.flush().await.map_err(|error| error.to_string())?;
        Ok::<(), String>(())
    }
    .await;
    if let Err(error) = result {
        let _ = async_fs::remove_file(&path).await;
        return Err(error);
    }
    Ok(path)
}

fn extract_zip(path: &Path, directory: &Path) -> std::result::Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry
            .enclosed_name()
            .and_then(|path| path.file_name().map(PathBuf::from))
            .ok_or_else(|| "invalid ZIP path".to_owned())?;
        total += entry.size();
        if total > 250 * 1024 * 1024 {
            return Err("Fineli ZIP expands beyond 250 MB".to_owned());
        }
        let mut output = File::create(directory.join(name)).map_err(|error| error.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        output.flush().map_err(|error| error.to_string())?;
    }
    for required in [
        "food.csv",
        "component.csv",
        "component_value.csv",
        "foodname_FI.csv",
    ] {
        if !directory.join(required).is_file() {
            return Err(format!("Fineli ZIP is missing {required}"));
        }
    }
    Ok(())
}
