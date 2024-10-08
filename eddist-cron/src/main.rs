use std::{env, str::FromStr, time::Duration};

use chrono::{TimeDelta, Timelike, Utc};
use cron::Schedule;
use eddist_core::utils::is_prod;
use s3::{creds::Credentials, Bucket};
use sqlx::mysql::MySqlPoolOptions;

mod repository;

#[tokio::main]
async fn main() {
    if !is_prod() {
        dotenvy::dotenv().unwrap();
    }

    // Jobs:
    // - inactivate and archive (not to show thread list),
    // - archive (move to archive table)
    // - convert (to dat text file compressed by gzip and delete responses, and publish to S3 compatible storage)

    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        eprintln!("Usage: eddist-cron <job> [args...]");
        std::process::exit(1);
    }

    let executed_time = Utc::now();
    let pool = MySqlPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let repo = repository::Repository::new(pool);

    // TODO: logging
    match args[1].as_str() {
        "inactivate" => {
            // inactivate and archive
            // - inactivate (set active to false, archived to true)

            let boards = repo.get_all_boards_info().await.unwrap();
            for b in boards {
                if let (Some(cron), Some(trigger)) = (
                    b.threads_archive_cron,
                    b.threads_archive_trigger_thread_count,
                ) {
                    let schedule = Schedule::from_str(&cron).unwrap();
                    let next = schedule
                        .after(
                            &Utc::now()
                                .checked_sub_signed(TimeDelta::minutes(1))
                                .unwrap(),
                        )
                        .next()
                        .unwrap();

                    // Execute when current time is next(only minute)
                    let next = next.with_second(0).unwrap();
                    let executed_time = executed_time
                        .with_second(0)
                        .unwrap()
                        .with_nanosecond(0)
                        .unwrap();

                    if executed_time != next {
                        println!("`inactivate` Cronjob for board: {} is not executed, current time: {}, next time: {}", b.board_key, executed_time, next);
                        continue;
                    }

                    repo.update_threads_to_inactive(&b.board_key, trigger as u32)
                        .await
                        .unwrap();
                    println!(
                        "`inactivate` Cronjob for board: {} is executed",
                        b.board_key
                    );
                }
            }
        }
        "archive" => {
            // archive
            // - archive (move to archive table)
            let boards = repo.get_all_boards_info().await.unwrap();
            for board in boards {
                let threads = repo
                    .get_threads_with_archive_converted(&board.board_key, true)
                    .await
                    .unwrap();
                for (_, _, id) in threads {
                    repo.archive_thread_and_responses(id).await.unwrap();
                }
            }
        }
        "convert" => {
            // convert
            // - convert (to dat text file compressed by gzip and delete responses, and publish to S3 compatible storage)
            let boards = repo.get_all_boards_info().await.unwrap();
            let s3_client = s3::bucket::Bucket::new(
                env::var("S3_BUCKET_NAME").unwrap().trim(),
                s3::Region::R2 {
                    account_id: env::var("R2_ACCOUNT_ID").unwrap().trim().to_string(),
                },
                Credentials::new(
                    Some(env::var("S3_ACCESS_KEY").unwrap().trim()),
                    Some(env::var("S3_ACCESS_SECRET_KEY").unwrap().trim()),
                    None,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap();

            for board in boards {
                let threads = repo
                    .get_threads_with_archive_converted(&board.board_key, false)
                    .await
                    .unwrap();
                let mut admin_dat = Vec::new();
                let mut dat = Vec::new();
                for (title, thread_number, id) in threads {
                    let responses = repo.get_thread_responses(id).await.unwrap();
                    for (idx, (res, client_info, authed_token_id)) in responses.iter().enumerate() {
                        let admin_res = if idx == 0 {
                            res.get_sjis_admin_bytes(
                                &board.default_name,
                                Some(title.as_str()),
                                client_info,
                                *authed_token_id,
                            )
                        } else {
                            res.get_sjis_admin_bytes(
                                &board.default_name,
                                None,
                                client_info,
                                *authed_token_id,
                            )
                        };
                        let res = if idx == 0 {
                            res.get_sjis_bytes(&board.default_name, Some(title.as_str()))
                        } else {
                            res.get_sjis_bytes(&board.default_name, None)
                        };

                        dat.append(&mut res.get_inner());
                        admin_dat.append(&mut admin_res.get_inner());
                    }

                    let result = s3_client
                        .put_object(
                            format!("{}/admin/{}.dat", board.board_key, thread_number),
                            &admin_dat,
                        )
                        .await;
                    if let Err(e) = result {
                        let result =
                            retry(&s3_client, &board.board_key, thread_number, &admin_dat).await;
                        if !result {
                            eprintln!(
                                "Failed to upload admin.dat: {}/{}, reason: {e}",
                                board.board_key, thread_number
                            );
                            continue;
                        }
                    }

                    let result = s3_client
                        .put_object(
                            format!("{}/dat/{}.dat", board.board_key, thread_number),
                            &dat,
                        )
                        .await;
                    if let Err(e) = result {
                        let result = retry(&s3_client, &board.board_key, thread_number, &dat).await;
                        if !result {
                            log::error!(
                                "Failed to upload dat: {}/{}, reason: {e}",
                                board.board_key,
                                thread_number
                            );
                            continue;
                        }
                    }

                    let client = redis::Client::open(env::var("REDIS_URL").unwrap()).unwrap();
                    client
                        .get_multiplexed_async_connection()
                        .await
                        .unwrap()
                        .send_packed_command(&redis::Cmd::del(format!(
                            "thread:{}:{}",
                            board.board_key, thread_number
                        )))
                        .await
                        .unwrap();

                    repo.update_archive_converted(id).await.unwrap();
                }
            }
        }
        job => {
            log::error!("Unknown job: {job}");
            std::process::exit(1);
        }
    }
}

async fn retry(s3_client: &Bucket, board_key: &str, thread_number: u64, content: &[u8]) -> bool {
    let mut retry_count = 0;
    let mut retry_delay = 2;
    let mut is_err = true;

    while is_err && retry_count < 3 {
        tokio::time::sleep(Duration::from_secs(retry_delay)).await;
        let result = s3_client
            .put_object(
                format!("{}/admin/{}.dat", board_key, thread_number),
                content,
            )
            .await;
        retry_count += 1;
        retry_delay *= 2;
        if let Ok(result) = result {
            if result.status_code() == 200 {
                is_err = false;
            }
        }
    }

    !is_err
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
