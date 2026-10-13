use anyhow::{Result, ensure};
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<()> {
    let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&pool)
        .await?;
    ensure!(
        database.starts_with("food_e2e_"),
        "test catalog requires a disposable E2E database"
    );
    sqlx::raw_sql(include_str!("catalog.sql"))
        .execute(&pool)
        .await?;
    Ok(())
}
