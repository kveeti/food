#[path = "import_foods/database.rs"]
mod database;
#[path = "import_foods/fineli.rs"]
mod fineli;
#[path = "import_foods/model.rs"]
mod model;
#[path = "import_foods/nutrients.rs"]
mod nutrients;
#[path = "import_foods/open_food_facts.rs"]
mod open_food_facts;

use std::{env, error::Error, fs, io, path::Path};

use database::write_import;
use fineli::read_fineli;
use open_food_facts::read_open_food_facts;
use sqlx::{Connection, PgConnection};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let kind = args
        .next()
        .ok_or_else(|| usage_error("missing import kind"))?;
    let import = match kind.as_str() {
        "fineli" => {
            let path = args
                .next()
                .ok_or_else(|| usage_error("missing Fineli directory"))?;
            if args.next().is_some() {
                return Err(usage_error("too many arguments").into());
            }
            read_fineli(Path::new(&path))?
        }
        "open-food-facts" => {
            let path = args.next().unwrap_or_else(|| "-".to_owned());
            if args.next().is_some() {
                return Err(usage_error("too many arguments").into());
            }
            if path == "-" {
                let stdin = io::stdin();
                read_open_food_facts(stdin.lock())?
            } else {
                read_open_food_facts(fs::File::open(path)?)?
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
    write_import(&mut connection, import).await?;

    Ok(())
}

fn usage_error(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "{message}\nusage:\n  import-foods fineli DIRECTORY\n  import-foods open-food-facts [TSV|-]"
        ),
    )
}
