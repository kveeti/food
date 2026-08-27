mod worker;

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::{fs as async_fs, io::AsyncWriteExt};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        IntoResponse, Response, RouterBuilder,
        content::{Form, multipart::Multipart},
        error::{bad_request, see_other},
        page, route,
    },
    view::view,
};
use url::Url;
use uuid::Uuid;

use crate::{auth, components::button, db};

#[derive(Clone)]
pub struct ImportConfig {
    directory: Arc<PathBuf>,
    http: reqwest::Client,
    automatic: bool,
}

impl ImportConfig {
    pub fn from_env(automatic: bool) -> Self {
        let directory = env::var_os("IMPORT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| env::temp_dir().join("version3-imports"));
        fs::create_dir_all(&directory).expect("IMPORT_DIR must be writable");
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .timeout(Duration::from_secs(60 * 60 * 6))
            .build()
            .expect("import HTTP client must build");
        Self {
            directory: Arc::new(directory),
            http,
            automatic,
        }
    }
}

fn config(cx: &Cx) -> &ImportConfig {
    app_context(cx)
}

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder
        .page(imports_page)
        .route(save_urls)
        .route(queue_delta_sync)
        .route(queue_full_sync)
        .route(upload_fineli)
        .route(upload_off_full)
        .route(upload_off_deltas)
}

pub async fn run_worker(pool: PgPool, config: ImportConfig) -> std::result::Result<(), String> {
    worker::run(pool, config).await
}

#[derive(sqlx::FromRow)]
struct JobRow {
    kind: String,
    status: String,
    original_name: Option<String>,
    details: Option<String>,
    error: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[page("/admin/imports")]
async fn imports_page(cx: &Cx) -> Result {
    auth::require_admin(cx).await?;
    let (off_full_url, off_delta_index_url): (String, String) = sqlx::query_as(
        "SELECT off_full_url, off_delta_index_url FROM import_settings WHERE singleton",
    )
    .fetch_one(db(cx))
    .await?;
    let (last_full_at, last_delta_end): (Option<chrono::DateTime<chrono::Utc>>, Option<i64>) =
        sqlx::query_as("SELECT last_full_at, last_delta_end FROM off_sync_state WHERE singleton")
            .fetch_one(db(cx))
            .await?;
    let jobs = sqlx::query_as::<_, JobRow>(
        "SELECT kind, status, original_name, details, error, created_at
         FROM import_jobs ORDER BY created_at DESC LIMIT 30",
    )
    .fetch_all(db(cx))
    .await?;

    view! {
        <main class="mx-auto w-full max-w-(--page-width) px-3 pb-[calc(var(--nav-height)+2rem)] pt-8 sm:px-6 sm:pb-12 sm:pt-12">
            <header class="mb-8">
                <p class="mb-1 text-sm text-gray-600">"Admin"</p>
                <h1 class="text-xl font-semibold tracking-tight text-gray-1000">"Food imports"</h1>
            </header>

            <section class="space-y-4" aria-labelledby="off-settings-heading">
                <h2 id="off-settings-heading" class="text-base font-medium text-gray-1000">"Open Food Facts sources"</h2>
                <div class="text-sm text-gray-600">
                    <p>"Last full import: " (last_full_at.map(|date| date.format("%Y-%m-%d %H:%M UTC").to_string()).unwrap_or_else(|| "never".to_owned()))</p>
                    <p>"Delta cursor: " (last_delta_end.map(|value| value.to_string()).unwrap_or_else(|| "none".to_owned()))</p>
                </div>
                <form method="post" action="/admin/imports/settings" class="space-y-3">
                    import_url_input(name: "off_full_url", label: "Full export URL", value: &off_full_url)
                    import_url_input(name: "off_delta_index_url", label: "Delta index URL", value: &off_delta_index_url)
                    <button class=(button::OUTLINE) type="submit">"Save URLs"</button>
                </form>
                <div class="flex flex-wrap gap-2">
                    <form method="post" action="/admin/imports/off/deltas/sync">
                        <button class=(button::OUTLINE) type="submit">"Sync deltas now"</button>
                    </form>
                    <form method="post" action="/admin/imports/off/full/sync">
                        <button class=(button::OUTLINE) type="submit">"Import full export now"</button>
                    </form>
                </div>
            </section>

            <section class="mt-10 space-y-5" aria-labelledby="uploads-heading">
                <h2 id="uploads-heading" class="text-base font-medium text-gray-1000">"Upload files"</h2>
                upload_form(action: "/admin/imports/fineli/upload", label: "Fineli ZIP", accept: ".zip", multiple: false)
                upload_form(action: "/admin/imports/off/full/upload", label: "OFF full JSONL gzip", accept: ".jsonl.gz,.gz", multiple: false)
                upload_form(action: "/admin/imports/off/deltas/upload", label: "OFF delta files", accept: ".json.gz,.gz", multiple: true)
            </section>

            <section class="mt-10" aria-labelledby="jobs-heading">
                <h2 id="jobs-heading" class="mb-3 text-base font-medium text-gray-1000">"Recent jobs"</h2>
                if jobs.is_empty() {
                    <p class="text-sm text-gray-600">"No imports yet."</p>
                } else {
                    <ul class="divide-y divide-gray-200 rounded-xl border border-gray-200">
                        for job in jobs {
                            <li class="px-4 py-3 text-sm">
                                <div class="flex items-center justify-between gap-3">
                                    <span class="font-medium text-gray-900">(job_label(&job.kind))</span>
                                    <span class="text-gray-600">(job.status)</span>
                                </div>
                                if let Some(name) = &job.original_name {
                                    <p class="mt-1 truncate text-gray-600">(name)</p>
                                }
                                if let Some(details) = &job.details {
                                    <p class="mt-1 text-gray-600">(details)</p>
                                }
                                if let Some(error) = &job.error {
                                    <p class="mt-1 text-danger-700">(error)</p>
                                }
                                <time class="mt-1 block text-xs text-gray-500">(job.created_at.format("%Y-%m-%d %H:%M UTC").to_string())</time>
                            </li>
                        }
                    </ul>
                }
            </section>
        </main>
    }
}

#[topcoat::view::component]
async fn import_url_input(name: &str, label: &str, value: &str) -> Result {
    view! {
        <label class="block text-sm text-gray-700">
            <span class="mb-1 block">(label)</span>
            <input class="w-full rounded-lg border border-gray-300 bg-form px-3 py-2.5 text-base text-gray-1000" type="url" name=(name) value=(value) required="true">
        </label>
    }
}

#[topcoat::view::component]
async fn upload_form(action: &str, label: &str, accept: &str, multiple: bool) -> Result {
    view! {
        <form method="post" action=(action) enctype="multipart/form-data" class="rounded-xl border border-gray-200 p-4">
            <label class="block text-sm text-gray-700">
                <span class="mb-2 block font-medium text-gray-900">(label)</span>
                <input type="file" name="files" accept=(accept) multiple=(multiple) required="true" class="block w-full text-sm">
            </label>
            <button class=(format!("mt-3 {}", button::OUTLINE)) type="submit">"Upload and queue"</button>
        </form>
    }
}

fn job_label(kind: &str) -> &str {
    match kind {
        "fineli_upload" => "Fineli upload",
        "off_full_upload" => "OFF full upload",
        "off_delta_upload" => "OFF delta upload",
        "off_full_sync" => "OFF full sync",
        "off_delta_sync" => "OFF delta sync",
        _ => "Import",
    }
}

#[derive(serde::Deserialize)]
struct UrlSettingsForm {
    off_full_url: String,
    off_delta_index_url: String,
}

#[route(POST "/admin/imports/settings")]
async fn save_urls(cx: &Cx, Form(input): Form<UrlSettingsForm>) -> Result<Response> {
    auth::require_admin(cx).await?;
    validate_http_url(&input.off_full_url)?;
    validate_http_url(&input.off_delta_index_url)?;
    sqlx::query(
        "UPDATE import_settings
         SET off_full_url = $1, off_delta_index_url = $2, updated_at = now()
         WHERE singleton",
    )
    .bind(input.off_full_url)
    .bind(input.off_delta_index_url)
    .execute(db(cx))
    .await?;
    see_other("/admin/imports").into_response(cx)
}

#[route(POST "/admin/imports/off/deltas/sync")]
async fn queue_delta_sync(cx: &Cx) -> Result<Response> {
    let user = auth::require_admin(cx).await?;
    queue_sync(db(cx), "off_delta_sync", Some(user.id)).await?;
    see_other("/admin/imports").into_response(cx)
}

#[route(POST "/admin/imports/off/full/sync")]
async fn queue_full_sync(cx: &Cx) -> Result<Response> {
    let user = auth::require_admin(cx).await?;
    queue_sync(db(cx), "off_full_sync", Some(user.id)).await?;
    see_other("/admin/imports").into_response(cx)
}

#[route(POST "/admin/imports/fineli/upload")]
async fn upload_fineli(cx: &Cx, multipart: Multipart) -> Result<Response> {
    let user = auth::require_admin(cx).await?;
    queue_uploads(cx, multipart, "fineli_upload", Some(user.id), false).await
}

#[route(POST "/admin/imports/off/full/upload")]
async fn upload_off_full(cx: &Cx, multipart: Multipart) -> Result<Response> {
    let user = auth::require_admin(cx).await?;
    queue_uploads(cx, multipart, "off_full_upload", Some(user.id), false).await
}

#[route(POST "/admin/imports/off/deltas/upload")]
async fn upload_off_deltas(cx: &Cx, multipart: Multipart) -> Result<Response> {
    let user = auth::require_admin(cx).await?;
    queue_uploads(cx, multipart, "off_delta_upload", Some(user.id), true).await
}

struct Upload {
    path: PathBuf,
    name: String,
    sha256: String,
    delta: Option<(i64, i64)>,
}

async fn queue_uploads(
    cx: &Cx,
    mut multipart: Multipart,
    kind: &str,
    requested_by: Option<Uuid>,
    multiple: bool,
) -> Result<Response> {
    let mut uploads = Vec::new();
    while let Some(mut field) = multipart.next_field().await? {
        if field.name() != Some("files") {
            continue;
        }
        if !multiple && !uploads.is_empty() {
            return Err(bad_request("upload one file").into());
        }
        let name = field
            .file_name()
            .and_then(|name| Path::new(name).file_name())
            .and_then(|name| name.to_str())
            .ok_or_else(|| bad_request("file name is required"))?
            .to_owned();
        validate_upload_name(kind, &name)?;
        let path = config(cx).directory.join(Uuid::now_v7().to_string());
        let mut output = async_fs::File::create(&path).await?;
        let mut digest = Sha256::new();
        let mut size = 0_u64;
        while let Some(chunk) = field.chunk().await? {
            size += chunk.len() as u64;
            if size > upload_limit(kind) {
                let _ = async_fs::remove_file(&path).await;
                return Err(bad_request("uploaded file is too large").into());
            }
            digest.update(&chunk);
            output.write_all(&chunk).await?;
        }
        output.flush().await?;
        uploads.push(Upload {
            path,
            delta: (kind == "off_delta_upload")
                .then(|| delta_range(&name))
                .transpose()?,
            name,
            sha256: format!("{:x}", digest.finalize()),
        });
    }
    if uploads.is_empty() {
        return Err(bad_request("file is required").into());
    }
    uploads.sort_by_key(|upload| upload.delta.map(|delta| delta.0).unwrap_or(0));
    for upload in uploads {
        if kind == "fineli_upload" {
            let duplicate: bool = sqlx::query_scalar(
                "SELECT EXISTS (
                     SELECT 1 FROM import_jobs
                     WHERE kind = 'fineli_upload' AND status = 'succeeded' AND sha256 = $1
                 )",
            )
            .bind(&upload.sha256)
            .fetch_one(db(cx))
            .await?;
            if duplicate {
                let _ = async_fs::remove_file(&upload.path).await;
                continue;
            }
        }
        sqlx::query(
            "INSERT INTO import_jobs (
                 kind, file_path, original_name, sha256, requested_by, delta_start, delta_end
             ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(kind)
        .bind(upload.path.to_string_lossy())
        .bind(upload.name)
        .bind(upload.sha256)
        .bind(requested_by)
        .bind(upload.delta.map(|delta| delta.0))
        .bind(upload.delta.map(|delta| delta.1))
        .execute(db(cx))
        .await?;
    }
    see_other("/admin/imports").into_response(cx)
}

fn upload_limit(kind: &str) -> u64 {
    match kind {
        "fineli_upload" => 100 * 1024 * 1024,
        "off_delta_upload" => 1024 * 1024 * 1024,
        _ => 20 * 1024 * 1024 * 1024,
    }
}

fn validate_upload_name(kind: &str, name: &str) -> Result<()> {
    let valid = match kind {
        "fineli_upload" => name.ends_with(".zip"),
        "off_delta_upload" => delta_range(name).is_ok(),
        _ => name.ends_with(".jsonl.gz") || name.ends_with(".json.gz") || name.ends_with(".gz"),
    };
    if !valid {
        return Err(bad_request("unsupported import file name").into());
    }
    Ok(())
}

pub(super) fn validate_http_url(value: &str) -> Result<()> {
    let url = Url::parse(value).map_err(|_| bad_request("invalid URL"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(bad_request("URL must use HTTP or HTTPS").into());
    }
    Ok(())
}

pub(super) fn delta_range(name: &str) -> Result<(i64, i64)> {
    let range = name
        .strip_prefix("openfoodfacts_products_")
        .and_then(|name| name.strip_suffix(".json.gz"))
        .ok_or_else(|| bad_request("invalid OFF delta filename"))?;
    let (start, end) = range
        .split_once('_')
        .ok_or_else(|| bad_request("invalid OFF delta filename"))?;
    let start = start
        .parse()
        .map_err(|_| bad_request("invalid OFF delta filename"))?;
    let end = end
        .parse()
        .map_err(|_| bad_request("invalid OFF delta filename"))?;
    if start >= end {
        return Err(bad_request("invalid OFF delta range").into());
    }
    Ok((start, end))
}

pub(super) async fn queue_sync(
    pool: &PgPool,
    kind: &str,
    requested_by: Option<Uuid>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO import_jobs (kind, requested_by)
         VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
    )
    .bind(kind)
    .bind(requested_by)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_off_delta_filename_ranges() {
        assert_eq!(
            delta_range("openfoodfacts_products_100_200.json.gz").unwrap(),
            (100, 200)
        );
        assert!(delta_range("delta.json.gz").is_err());
        assert!(delta_range("openfoodfacts_products_200_100.json.gz").is_err());
    }
}
