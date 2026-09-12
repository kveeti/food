mod database;
mod fineli;
mod model;
mod nutrients;
mod open_food_facts;

use std::{env, fs::File, io, path::PathBuf};

use anyhow::{Context, Result, bail};
use chrono::DateTime;
use database::{ImportMode, ImportWriter};
use flate2::read::MultiGzDecoder;
use model::{ImportReport, SOURCE_FINELI, SOURCE_OFF};
use sqlx::{Connection, PgConnection};

#[tokio::main]
async fn main() -> Result<()> {
    let command = parse_command(env::args().skip(1).collect())?;
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
    let mut connection = PgConnection::connect(&database_url).await?;
    sqlx::migrate!().run(&mut connection).await?;

    let (report, writer) = match command {
        Command::Fineli { directory } => {
            let mut writer = ImportWriter::start(
                &mut connection,
                SOURCE_FINELI,
                ImportMode::Full { snapshot_end: None },
            )
            .await?;
            let report = fineli::import(&directory, &mut writer).await?;
            (report, writer)
        }
        Command::OffFull {
            snapshot_end,
            input,
        } => {
            let mut writer = ImportWriter::start(
                &mut connection,
                SOURCE_OFF,
                ImportMode::Full {
                    snapshot_end: Some(snapshot_end),
                },
            )
            .await?;
            let report = open_food_facts::import(open_gzip(&input)?, &mut writer).await?;
            (report, writer)
        }
        Command::OffDelta { start, end, input } => {
            let mut writer = ImportWriter::start(
                &mut connection,
                SOURCE_OFF,
                ImportMode::Delta { start, end },
            )
            .await?;
            let report = open_food_facts::import(open_gzip(&input)?, &mut writer).await?;
            (report, writer)
        }
    };
    writer.finish().await?;
    print_report(&report);
    Ok(())
}

enum Command {
    Fineli { directory: PathBuf },
    OffFull { snapshot_end: i64, input: String },
    OffDelta { start: i64, end: i64, input: String },
}

fn parse_command(arguments: Vec<String>) -> Result<Command> {
    match arguments.as_slice() {
        [command, directory] if command == "fineli" => Ok(Command::Fineli {
            directory: directory.into(),
        }),
        [command, flag, snapshot_end, input]
            if command == "off-full" && flag == "--snapshot-end" =>
        {
            Ok(Command::OffFull {
                snapshot_end: parse_timestamp(snapshot_end)?,
                input: input.clone(),
            })
        }
        [command, start_flag, start, end_flag, end, input]
            if command == "off-delta" && start_flag == "--start" && end_flag == "--end" =>
        {
            Ok(Command::OffDelta {
                start: parse_timestamp(start)?,
                end: parse_timestamp(end)?,
                input: input.clone(),
            })
        }
        _ => bail!(
            "usage:\n  import-foods fineli DIRECTORY\n  import-foods off-full --snapshot-end TIMESTAMP JSONL.GZ|-\n  import-foods off-delta --start TIMESTAMP --end TIMESTAMP JSONL.GZ|-"
        ),
    }
}

fn parse_timestamp(value: &str) -> Result<i64> {
    if let Ok(timestamp) = value.parse::<i64>() {
        return Ok(timestamp);
    }
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return Ok(timestamp.timestamp());
    }
    if let Ok(timestamp) = DateTime::parse_from_rfc2822(value) {
        return Ok(timestamp.timestamp());
    }
    bail!("invalid timestamp {value:?}; use Unix seconds, RFC 3339, or RFC 2822")
}

fn open_gzip(path: &str) -> Result<Box<dyn io::Read>> {
    if path == "-" {
        Ok(Box::new(MultiGzDecoder::new(io::stdin())))
    } else {
        Ok(Box::new(MultiGzDecoder::new(
            File::open(path).with_context(|| format!("could not open {path}"))?,
        )))
    }
}

fn print_report(report: &ImportReport) {
    let ignored_values = report.ignored_values.values().sum::<u64>();
    let invalid_values = report.invalid_values.values().sum::<u64>();
    let incompatible_values = report.incompatible_values.values().sum::<u64>();
    let unknown_values = report.unknown_values.values().sum::<u64>();
    println!(
        "scanned {} foods; imported {} foods and {} nutrient values; skipped {} for basis, {} for name, and {} for nutrients; ignored {} fields; rejected {} invalid and {} incompatible values; found {} unknown values across {} keys",
        report.scanned_foods,
        report.imported_foods,
        report.imported_values,
        report.skipped_basis,
        report.skipped_names,
        report.skipped_nutrients,
        ignored_values,
        invalid_values,
        incompatible_values,
        unknown_values,
        report.unknown_values.len(),
    );
    for (mapping, count) in &report.mapped_values {
        println!("mapped nutrient: {mapping} ({count})");
    }
    for (key, count) in &report.ignored_values {
        println!("ignored nutrient: {key} ({count})");
    }
    for (key, count) in &report.invalid_values {
        println!("invalid nutrient: {key} ({count})");
    }
    for (key, count) in &report.incompatible_values {
        println!("incompatible nutrient: {key} ({count})");
    }
    for (key, count) in &report.unknown_values {
        println!("unknown nutrient: {key} ({count})");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_accept_off_cursors_and_http_dates() {
        assert_eq!(parse_timestamp("1722686400").unwrap(), 1_722_686_400);
        assert_eq!(
            parse_timestamp("2024-08-03T12:00:00Z").unwrap(),
            1_722_686_400
        );
        assert_eq!(
            parse_timestamp("Sat, 03 Aug 2024 12:00:00 GMT").unwrap(),
            1_722_686_400
        );
    }
}
