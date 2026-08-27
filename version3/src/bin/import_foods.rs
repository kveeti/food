use std::{env, error::Error, fs, io, path::Path};

use flate2::read::MultiGzDecoder;
use sqlx::{Connection, PgConnection};
use version3::imports::{
    database::{write_delta, write_import},
    fineli::read_fineli,
    open_food_facts::{read_open_food_facts, read_open_food_facts_delta},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let kind = args
        .next()
        .ok_or_else(|| usage_error("missing import kind"))?;
    let (import, delta) = match kind.as_str() {
        "fineli" => {
            let path = args
                .next()
                .ok_or_else(|| usage_error("missing Fineli directory"))?;
            if args.next().is_some() {
                return Err(usage_error("too many arguments").into());
            }
            (read_fineli(Path::new(&path))?, None)
        }
        "open-food-facts" | "open-food-facts-gzip" | "open-food-facts-delta-gzip" => {
            let compressed = kind != "open-food-facts";
            let is_delta = kind == "open-food-facts-delta-gzip";
            let path = args.next().unwrap_or_else(|| "-".to_owned());
            if args.next().is_some() {
                return Err(usage_error("too many arguments").into());
            }
            if is_delta {
                let delta = if path == "-" {
                    let stdin = io::stdin();
                    read_open_food_facts_delta(MultiGzDecoder::new(stdin.lock()))?
                } else {
                    read_open_food_facts_delta(MultiGzDecoder::new(fs::File::open(path)?))?
                };
                (delta.import, Some(delta.changed_source_ids))
            } else if path == "-" {
                let stdin = io::stdin();
                let import = if compressed {
                    read_open_food_facts(MultiGzDecoder::new(stdin.lock()))?
                } else {
                    read_open_food_facts(stdin.lock())?
                };
                (import, None)
            } else {
                let import = if compressed {
                    read_open_food_facts(MultiGzDecoder::new(fs::File::open(path)?))?
                } else {
                    read_open_food_facts(fs::File::open(path)?)?
                };
                (import, None)
            }
        }
        _ => return Err(usage_error("unknown import kind").into()),
    };

    let database_url =
        env::var("DATABASE_URL").map_err(|_| usage_error("DATABASE_URL is not set"))?;
    let mut connection = PgConnection::connect(&database_url).await?;
    sqlx::raw_sql(include_str!("../../migrations/001_initial.sql"))
        .execute(&mut connection)
        .await?;
    if let Some(changed_source_ids) = delta {
        write_delta(
            &mut connection,
            version3::imports::model::FoodDelta {
                import,
                changed_source_ids,
            },
        )
        .await?;
    } else {
        write_import(&mut connection, import).await?;
    }

    Ok(())
}

fn usage_error(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "{message}\nusage:\n  import-foods fineli DIRECTORY\n  import-foods open-food-facts [JSONL|-]\n  import-foods open-food-facts-gzip [JSONL.GZ|-]\n  import-foods open-food-facts-delta-gzip [JSON.GZ|-]"
        ),
    )
}
