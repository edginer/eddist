use std::{env, str::FromStr, time::Duration};

use chrono::{Timelike, Utc};
use cron::Schedule;
use sqlx::mysql::MySqlPoolOptions;

mod repository;

#[tokio::main]
async fn main() {
    // Jobs:
    // - inactivate and archive (not to show thread list),
    // - archive (move to archive table)
    // - convert (to dat text file compressed by gzip and delete responses, and publish to Cloudflare R2)

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        eprintln!("Usage: eddist-cron <job> [args...]");
        std::process::exit(1);
    }

    let executed_time = Utc::now();

    // TODO: logging
    match args[1].as_str() {
        "inactivate" => {
            // inactivate and archive
            // - inactivate (set active to false)
            let pool = MySqlPoolOptions::new()
                .max_connections(8)
                .acquire_timeout(Duration::from_secs(5))
                .connect(&env::var("DATABASE_URL").unwrap())
                .await
                .unwrap();
            let repo = repository::Repository::new(pool);
            let boards = repo.get_all_boards_info().await.unwrap();
            for b in boards {
                if let (Some(cron), Some(trigger)) = (
                    b.threads_archive_cron,
                    b.threads_archive_trigger_thread_count,
                ) {
                    let schedule = Schedule::from_str(&cron).unwrap();
                    let next = schedule.upcoming(Utc).next().unwrap();

                    // Execute when current time is next(only minute)
                    let next = next.with_second(0).unwrap();
                    if executed_time.with_second(0).unwrap() != next {
                        continue;
                    }

                    repo.update_threads_to_inactive(&b.board_key, trigger as u32)
                        .await
                        .unwrap();
                }
            }
        }
        "archive" => {
            // archive
            // - archive (move to archive table)
        }
        "convert" => {
            // convert
            // - convert (to dat text file compressed by gzip and delete responses, and publish to Cloudflare R2)
            todo!();
        }
        job => {
            eprintln!("Unknown job: {job}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let cron = "0 */1 * * * *";
        let schedule = Schedule::from_str(cron).unwrap();
        for datetime in schedule.upcoming(Utc).take(10) {
            println!("{datetime}");
        }
    }
}
