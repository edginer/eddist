//! Opt-in, disposable-MySQL measurements shared by the pre/post migration commits.
use super::*;
use eddist::repositories::bbs_repository::ThreadStatus;
use eddist_core::domain::tinker::Tinker;
use sha2::{Digest, Sha256};
use sqlx::{AssertSqlSafe, Executor, QueryBuilder, Row, SqlSafeStr};
use std::{future::Future, time::Instant};

const WARMUP: usize = 5;
const SAMPLES: usize = 101;
const PARALLEL_SAMPLES: usize = 31;

async fn measure<F, Fut, T>(mut operation: F, samples: usize) -> serde_json::Value
where
    F: FnMut() -> Fut,
    Fut: Future<Output = T>,
{
    let mut times = Vec::with_capacity(samples);
    for index in 0..WARMUP + samples {
        let start = Instant::now();
        let output = operation().await;
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        std::hint::black_box(output);
        if index >= WARMUP {
            times.push(elapsed);
        }
    }
    let mut sorted = times.clone();
    sorted.sort_by(f64::total_cmp);
    serde_json::json!({"samples_ms": times, "p50_ms": sorted[samples / 2], "p95_ms": sorted[(samples * 95).div_ceil(100) - 1]})
}

fn digest(bytes: impl AsRef<[u8]>) -> String {
    eddist_core::utils::to_hex(Sha256::digest(bytes.as_ref()))
}

async fn global_select_count(db: &TestDatabase) -> u64 {
    let (_, count): (String, String) = sqlx::query_as("SHOW GLOBAL STATUS LIKE 'Com_select'")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    count.parse().unwrap()
}

async fn seed(
    db: &TestDatabase,
    fixture: usize,
    threads: usize,
    responses: usize,
) -> (Uuid, String, Uuid) {
    let board = Uuid::from_u128(1 + fixture as u128);
    let key = format!("benchmark{fixture}");
    sqlx::query("INSERT INTO boards (id, name, board_key, default_name) VALUES (?, '計測板', ?, '名無しさん')")
        .bind(board).bind(&key).execute(&db.pool).await.unwrap();
    let token_id = |index: usize| Uuid::from_u128(100 + (index % 50) as u128);
    let thread_id = |index: usize| Uuid::from_u128(((fixture as u128 + 1) << 24) + index as u128);
    for chunk in (0..threads).collect::<Vec<_>>().chunks(500) {
        let mut query = QueryBuilder::new(
            "INSERT INTO threads (id, board_id, thread_number, last_modified_at, sage_last_modified_at, title, authed_token_id, metadent, response_count, active) ",
        );
        query.push_values(chunk, |mut row, index| {
            let at = time(1) + chrono::Duration::seconds(*index as i64);
            row.push_bind(thread_id(*index))
                .push_bind(board)
                .push_bind(1_700_000_000_i64 + *index as i64)
                .push_bind(at)
                .push_bind(at)
                .push_bind("日本語の計測スレッド &amp; タイトル")
                .push_bind(token_id(*index))
                .push_bind("vvv")
                .push_bind(responses as i32)
                .push_bind(responses < 1000);
        });
        query.build().execute(&db.pool).await.unwrap();
    }
    let body = "日本語の本文です。HTML &amp; Shift-JIS を含みます。<br>".repeat(4);
    let client = serde_json::to_value(ClientInfo {
        user_agent:
            "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 Chrome/130.0 Mobile Safari/537.36"
                .into(),
        asn_num: 64501,
        ip_addr: "2001:db8::1234".into(),
        tinker: Some(Box::new(Tinker::new(
            "benchmark-token".into(),
            time(1).and_utc(),
        ))),
    })
    .unwrap();
    for start in (0..threads * responses).step_by(500) {
        let mut query = QueryBuilder::new(
            "INSERT INTO responses (id, author_name, mail, body, created_at, author_id, ip_addr, authed_token_id, board_id, thread_id, is_abone, is_abone_keep_id, res_order, client_info) ",
        );
        query.push_values(
            start..(start + 500).min(threads * responses),
            |mut row, index| {
                let thread = index / responses;
                let order = index % responses + 1;
                let id = Uuid::from_u128(1_000_000 + fixture as u128 * 10_000_000 + index as u128);
                row.push_bind(id)
                    .push_bind(if order.is_multiple_of(3) {
                        "投稿者"
                    } else {
                        ""
                    })
                    .push_bind(if order.is_multiple_of(2) { "sage" } else { "" })
                    .push_bind(&body)
                    .push_bind(time(2) + chrono::Duration::seconds(order as i64))
                    .push_bind("authorID")
                    .push_bind("2001:db8::1234")
                    .push_bind(token_id(thread + order))
                    .push_bind(board)
                    .push_bind(thread_id(thread))
                    .push_bind(order.is_multiple_of(11))
                    .push_bind(order.is_multiple_of(2))
                    .push_bind(order as i32)
                    .push_bind(&client);
            },
        );
        query.build().execute(&db.pool).await.unwrap();
    }
    (board, key, thread_id(0))
}

/// Run this test alone: it temporarily enables the general log on its own
/// disposable MySQL container to capture actual executed SQL, never production.
/// EDDIST_BENCH_OUTPUT=/private/tmp/core-sqlx.json EDDIST_BENCH_LABEL=sqlx-core \
/// SQLX_OFFLINE=true cargo test -p eddist --test repository_read_test \
/// repository_benchmark::core_read_benchmark -- --ignored --exact --nocapture
#[tokio::test]
#[ignore = "opt-in repeatable MySQL latency, output, statement and EXPLAIN capture"]
async fn core_read_benchmark() {
    let output = std::env::var("EDDIST_BENCH_OUTPUT")
        .expect("set EDDIST_BENCH_OUTPUT to the JSON output path");
    let label = std::env::var("EDDIST_BENCH_LABEL")
        .expect("set EDDIST_BENCH_LABEL to identify the measured commit");
    let db = TestDatabase::with_max_connections(8).await;
    let repo = db.bbs();
    let version: String = sqlx::query_scalar("SELECT VERSION()")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    for index in 0..50 {
        let mut token = token_fixture(
            &format!("benchmark-{index}"),
            &format!("2001:db8::{:x}", index + 1),
            &format!("{index:06}"),
        );
        token.id = Uuid::from_u128(100 + index);
        repo.create_authed_token(token).await.unwrap();
    }
    let mut fixtures = Vec::new();
    for (fixture, (threads, responses)) in [(50, 1), (500, 30), (5000, 30), (50, 1000)]
        .into_iter()
        .enumerate()
    {
        let (board, key, thread_id) = seed(&db, fixture, threads, responses).await;
        // Avoid comparing stale row estimates immediately after bulk loading.
        db.pool
            .execute("ANALYZE TABLE threads, responses, authed_tokens")
            .await
            .unwrap();
        let number = 1_700_000_000;
        // Warm prepared statements before counting, so driver metadata checks are excluded.
        repo.get_threads(board, ThreadStatus::Unarchived)
            .await
            .unwrap();
        repo.get_thread_by_board_key_and_thread_number(&key, number)
            .await
            .unwrap();
        repo.get_responses(thread_id).await.unwrap();
        repo.get_threads_with_metadent(board).await.unwrap();
        db.pool
            .execute("SET GLOBAL general_log = 'OFF'")
            .await
            .unwrap();
        db.pool
            .execute("SET GLOBAL log_output = 'TABLE'")
            .await
            .unwrap();
        db.pool
            .execute("TRUNCATE TABLE mysql.general_log")
            .await
            .unwrap();
        let before = global_select_count(&db).await;
        db.pool
            .execute("SET GLOBAL general_log = 'ON'")
            .await
            .unwrap();
        let list = repo
            .get_threads(board, ThreadStatus::Unarchived)
            .await
            .unwrap();
        let single = repo
            .get_thread_by_board_key_and_thread_number(&key, number)
            .await
            .unwrap()
            .unwrap();
        let res = repo.get_responses(thread_id).await.unwrap();
        let meta = repo.get_threads_with_metadent(board).await.unwrap();
        db.pool
            .execute("SET GLOBAL general_log = 'OFF'")
            .await
            .unwrap();
        assert_eq!(
            global_select_count(&db).await - before,
            4,
            "each repository read must issue exactly one SELECT"
        );
        assert_eq!(list.len(), threads);
        assert_eq!(res.len(), responses);
        assert_eq!(meta.len(), threads);
        let sql: Vec<Vec<u8>> = sqlx::query_scalar("SELECT argument FROM mysql.general_log WHERE command_type = 'Execute' AND argument LIKE 'SELECT%' ORDER BY event_time")
            .fetch_all(&db.pool).await.unwrap();
        assert_eq!(sql.len(), 4, "capture actual prepared-statement executions");
        let mut shapes = Vec::new();
        for (index, sql) in sql.into_iter().enumerate() {
            let sql = String::from_utf8(sql).expect("captured synthetic SQL must be valid UTF-8");
            // Only the four fixed SELECTs above, with synthetic fixture values,
            // can be present in this disposable container's cleared general log.
            let columns = db
                .pool
                .describe(AssertSqlSafe(sql.clone()).into_sql_str())
                .await
                .unwrap()
                .columns()
                .len();
            assert_eq!(columns, [13, 13, 7, 28][index], "projection drift");
            let plan: Vec<String> = sqlx::query(AssertSqlSafe(format!("EXPLAIN ANALYZE {sql}")))
                .fetch_all(&db.pool)
                .await
                .unwrap()
                .iter()
                .map(|row| row.get(0))
                .collect();
            shapes
                .push(serde_json::json!({"sql": sql, "columns": columns, "explain_analyze": plan}));
        }
        let dat: Vec<u8> = res
            .iter()
            .enumerate()
            .flat_map(|(index, row)| {
                row.get_sjis_bytes("名無しさん", (index == 0).then_some(single.title.as_str()))
                    .get_inner()
            })
            .collect();
        let list_hash = digest(format!("{list:?}"));
        let single_hash = digest(format!("{single:?}"));
        let meta_debug = format!("{meta:?}");
        let outcomes = serde_json::json!({"thread_list_sha256": list_hash, "single_thread_sha256": single_hash, "metadent_sha256": digest(&meta_debug), "metadent_debug_bytes": meta_debug.len(), "dat_sha256": digest(&dat), "dat_bytes": dat.len()});
        let list_times = measure(
            || async {
                repo.get_threads(board, ThreadStatus::Unarchived)
                    .await
                    .unwrap()
            },
            SAMPLES,
        )
        .await;
        let single_times = measure(
            || async {
                repo.get_thread_by_board_key_and_thread_number(&key, number)
                    .await
                    .unwrap()
                    .unwrap()
            },
            SAMPLES,
        )
        .await;
        let res_times = measure(
            || async { repo.get_responses(thread_id).await.unwrap() },
            SAMPLES,
        )
        .await;
        let meta_times = measure(
            || async { repo.get_threads_with_metadent(board).await.unwrap() },
            SAMPLES,
        )
        .await;
        let concurrent = measure(
            || async {
                let results = futures::future::join_all(
                    (0..4).map(|_| repo.get_threads_with_metadent(board)),
                )
                .await;
                for result in results {
                    assert_eq!(result.unwrap().len(), threads);
                }
            },
            PARALLEL_SAMPLES,
        )
        .await;
        eprintln!("{label}: {threads} threads x {responses} responses captured");
        fixtures.push(serde_json::json!({"threads": threads, "responses_per_thread": responses, "outcomes": outcomes, "statements_per_read": 1, "queries": shapes,
            "thread_list": list_times, "single_thread": single_times, "responses": res_times, "metadent": meta_times, "metadent_parallel_4_batch": concurrent}));
    }
    let result = serde_json::json!({"label": label, "mysql_version": version, "pool_max_connections": 8, "acquire_timeout_seconds": 5, "token_count": 50, "analyze_after_bulk_load": true, "warmup": WARMUP, "samples": SAMPLES, "parallel_samples": PARALLEL_SAMPLES, "parallelism": 4, "units": "milliseconds; parallel measurements are the full batch of four reads", "fixtures": fixtures});
    std::fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
}
