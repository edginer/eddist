use std::{env, str::FromStr, time::Duration};

use chrono::{TimeDelta, Timelike, Utc};
use cron::Schedule;
use eddist_core::{tracing::init_tracing, utils::is_prod};
use s3::{creds::Credentials, Bucket};
use sqlx::mysql::MySqlPoolOptions;
use tokio::time::sleep;

mod repository;

#[tokio::main]
async fn main() {
    if !is_prod() {
        dotenvy::dotenv().unwrap();
    }

    init_tracing();

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
        .after_connect(|conn, _| {
            use sqlx::Executor;

            Box::pin(async move {
                conn.execute(
                    "SET SESSION sql_mode = CONCAT(@@sql_mode, ',TIME_TRUNCATE_FRACTIONAL')",
                )
                .await
                .unwrap();
                log::info!("Set TIME_TRUNCATE_FRACTIONAL mode");
                Ok(())
            })
        })
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(25))
        .connect(&env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let repo = repository::Repository::new(pool);

    log::info!("Application started with args: {args:?}");

    // TODO: logging
    match args[1].as_str() {
        "inactivate" => {
            // inactivate and archive
            // - inactivate (set active to false, archived to true)

            let boards = repo.get_all_boards_info().await.unwrap();
            let mut tasks = Vec::new();

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
                        log::info!("`inactivate` Cronjob for board: {} is not executed, current time: {}, next time: {}", b.board_key, executed_time, next);
                        continue;
                    }

                    // Create parallel task for each board
                    let repo_clone = repo.clone();
                    let board_key = b.board_key.clone();

                    let task = tokio::spawn(async move {
                        // Randomize thread archive timing (0-59 seconds)
                        let random_delay = rand::random::<u64>() % 60;
                        sleep(Duration::from_secs(random_delay)).await;

                        match repo_clone
                            .update_threads_to_inactive(&board_key, trigger as u32)
                            .await
                        {
                            Ok(_) => {
                                log::info!(
                                    "`inactivate` Cronjob for board: {board_key} is executed"
                                );
                                Ok(())
                            }
                            Err(e) => {
                                log::error!(
                                    "`inactivate` Cronjob for board: {board_key} failed: {e}"
                                );
                                Err(e)
                            }
                        }
                    });

                    tasks.push(task);
                }
            }

            // Wait for all tasks to complete
            let results = futures::future::join_all(tasks).await;
            let mut success_count = 0;
            let mut error_count = 0;

            for result in results {
                match result {
                    Ok(Ok(())) => success_count += 1,
                    Ok(Err(_)) => error_count += 1,
                    Err(e) => {
                        log::error!("Task panicked: {}", e);
                        error_count += 1;
                    }
                }
            }

            if error_count > 0 {
                log::error!("Some tasks failed: Success: {success_count}, Errors: {error_count}");
            } else {
                log::info!("All tasks completed successfully");
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

                for (title, thread_number, id) in threads {
                    let mut admin_dat = Vec::new();
                    let mut dat = Vec::new();

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

                    // TODO: sjis to utf-8 workarounds for now
                    let admin_dat = encoding_rs::SHIFT_JIS.decode(&admin_dat).0.into_owned();
                    let dat = encoding_rs::SHIFT_JIS.decode(&dat).0.into_owned();

                    if retry(
                        &s3_client,
                        &board.board_key,
                        thread_number,
                        admin_dat.as_bytes(),
                        true,
                    )
                    .await
                    .is_err()
                    {
                        log::error!(
                            "Failed to upload admin.dat: {}/{}",
                            board.board_key,
                            thread_number
                        );
                        continue;
                    }

                    if retry(
                        &s3_client,
                        &board.board_key,
                        thread_number,
                        dat.as_bytes(),
                        false,
                    )
                    .await
                    .is_err()
                    {
                        log::error!(
                            "Failed to upload normal dat: {}/{}",
                            board.board_key,
                            thread_number
                        );
                        continue;
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
        "backfill-convert" => {
            let start = args[2].parse::<u64>().unwrap();
            let end = args[3].parse::<u64>().unwrap();

            // backfill-convert
            // - convert (to dat text file compressed by gzip and delete responses, and publish to S3 compatible storage)
            //   with only threads that are not converted yet because of the previous error
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
                    .get_archived_threads(&board.board_key, start, end)
                    .await
                    .unwrap();

                log::info!(
                    "target thread count for backfill-convert: {}",
                    threads.len()
                );
                let mut backfill_dat_count = 0;
                let mut backfill_admin_dat_count = 0;

                for (title, thread_number, id) in threads {
                    let mut admin_dat = Vec::new();
                    let mut dat = Vec::new();

                    let responses = repo.get_archived_thread_responses(id).await.unwrap();
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

                    // TODO: sjis to utf-8 workarounds for now
                    let admin_dat = encoding_rs::SHIFT_JIS.decode(&admin_dat).0.into_owned();
                    let dat = encoding_rs::SHIFT_JIS.decode(&dat).0.into_owned();

                    let admin_needs = if let Ok((_, code)) = s3_client
                        .head_object(format!(
                            "{}/{}/{}.dat",
                            board.board_key, "admin", thread_number
                        ))
                        .await
                    {
                        if code == 404 {
                            true
                        } else {
                            log::info!(
                                "admin.dat already exists: {}/{}",
                                board.board_key,
                                thread_number
                            );
                            false
                        }
                    } else {
                        true
                    };

                    if admin_needs {
                        backfill_admin_dat_count += 1;
                        log::info!(
                            "admin.dat needs to be uploaded: {}/{}",
                            board.board_key,
                            thread_number
                        );

                        if retry(
                            &s3_client,
                            &board.board_key,
                            thread_number,
                            admin_dat.as_bytes(),
                            true,
                        )
                        .await
                        .is_err()
                        {
                            log::error!(
                                "Failed to upload admin.dat: {}/{}",
                                board.board_key,
                                thread_number
                            );
                            continue;
                        }
                    }

                    let dat_needs = if let Ok((_, code)) = s3_client
                        .head_object(format!(
                            "{}/{}/{}.dat",
                            board.board_key, "dat", thread_number
                        ))
                        .await
                    {
                        if code == 404 {
                            true
                        } else {
                            log::info!(
                                "normal.dat already exists: {}/{}",
                                board.board_key,
                                thread_number
                            );
                            false
                        }
                    } else {
                        true
                    };

                    if dat_needs {
                        backfill_dat_count += 1;
                        log::info!(
                            "normal.dat needs to be uploaded: {}/{}",
                            board.board_key,
                            thread_number
                        );

                        if retry(
                            &s3_client,
                            &board.board_key,
                            thread_number,
                            dat.as_bytes(),
                            false,
                        )
                        .await
                        .is_err()
                        {
                            log::error!(
                                "Failed to upload normal dat: {}/{}",
                                board.board_key,
                                thread_number
                            );
                            continue;
                        }
                    }
                }

                log::info!(
                    "backfill-convert: admin: {}/dat: {} for board: {}",
                    backfill_admin_dat_count,
                    backfill_dat_count,
                    board.board_key
                );
            }
        }

        job => {
            log::error!("Unknown job: {job}");
            std::process::exit(1);
        }
    }
}

async fn retry(
    s3_client: &Bucket,
    board_key: &str,
    thread_number: u64,
    content: &[u8],
    is_admin: bool,
) -> Result<(), ()> {
    let mut retry_count = -1;
    let mut retry_delay = 2;
    let mut is_err = true;

    while is_err && retry_count < 3 {
        if retry_count >= 0 {
            tokio::time::sleep(Duration::from_secs(retry_delay)).await;
        }
        let result = s3_client
            .put_object(
                format!(
                    "{}/{}/{}.dat",
                    board_key,
                    if is_admin { "admin" } else { "dat" },
                    thread_number
                ),
                content,
            )
            .await;
        retry_count += 1;
        retry_delay *= 2;
        if let Ok(result) = result {
            if result.status_code() == 200 {
                if let Ok((_, code)) = s3_client
                    .head_object(format!(
                        "{}/{}/{}.dat",
                        board_key,
                        if is_admin { "admin" } else { "dat" },
                        thread_number
                    ))
                    .await
                {
                    if code != 404 {
                        is_err = false;
                    }
                }
            }
        }
    }

    if !is_err {
        log::info!(
            "Successfully uploaded {}.dat: {}/{}, retry count: {}",
            if is_admin { "admin" } else { "normal" },
            board_key,
            thread_number,
            retry_count
        );
    }

    if is_err {
        Err(())
    } else {
        Ok(())
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
