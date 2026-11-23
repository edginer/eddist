use chrono::Utc;
use sqlx::MySqlPool;
use uuid::Uuid;

/// Create a test board directly in the database
pub async fn create_test_board(pool: &MySqlPool, board_key: &str, name: &str) -> Uuid {
    let board_id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO boards (id, name, board_key, default_name)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(board_id)
    .bind(name)
    .bind(board_key)
    .bind("名無しさん")
    .execute(pool)
    .await
    .expect("Failed to create test board");

    sqlx::query(
        r#"
        INSERT INTO boards_info (id, local_rules, created_at, updated_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(board_id)
    .bind("テストルール")
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(pool)
    .await
    .expect("Failed to create board info");

    board_id
}

/// Create a test authed token
pub async fn create_test_authed_token(
    pool: &MySqlPool,
    origin_ip: &str,
    auth_code: &str,
) -> (Uuid, String) {
    let token_id = Uuid::now_v7();
    let token = format!("test-token-{}", Uuid::now_v7());

    sqlx::query(
        r#"
        INSERT INTO authed_tokens
        (id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua, auth_code, created_at, authed_at, validity, last_wrote_at, additional_info)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(token_id)
    .bind(&token)
    .bind(origin_ip)
    .bind(origin_ip)
    .bind(0i32)
    .bind("Test User Agent")
    .bind("Test User Agent")
    .bind(auth_code)
    .bind(Utc::now())
    .bind(Utc::now())
    .bind(true)
    .bind(Option::<chrono::DateTime<Utc>>::None)
    .bind(Option::<serde_json::Value>::None)
    .execute(pool)
    .await
    .expect("Failed to create authed token");

    (token_id, token)
}

/// Create a test thread
pub async fn create_test_thread(
    pool: &MySqlPool,
    board_id: Uuid,
    thread_number: i64,
    title: &str,
    authed_token_id: Uuid,
) -> Uuid {
    let thread_id = Uuid::now_v7();

    sqlx::query(
        r#"
        INSERT INTO threads
        (id, board_id, thread_number, last_modified_at, sage_last_modified_at,
         title, authed_token_id, metadent, response_count, no_pool, active, archived, archive_converted)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(thread_id)
    .bind(board_id)
    .bind(thread_number)
    .bind(Utc::now())
    .bind(Utc::now())
    .bind(title)
    .bind(authed_token_id)
    .bind("")
    .bind(0)
    .bind(false)
    .bind(true)
    .bind(false)
    .bind(false)
    .execute(pool)
    .await
    .expect("Failed to create thread");

    thread_id
}

/// Create a test response
pub async fn create_test_response(
    pool: &MySqlPool,
    board_id: Uuid,
    thread_id: Uuid,
    authed_token_id: Uuid,
    res_order: i32,
    body: &str,
) -> Uuid {
    let res_id = Uuid::now_v7();
    let client_info = serde_json::json!({
        "user_agent": "Test UA",
        "ip_addr": "127.0.0.1",
        "asn_num": 0,
        "tinker": null
    });

    sqlx::query(
        r#"
        INSERT INTO responses
        (id, author_name, mail, body, created_at, author_id, ip_addr,
         authed_token_id, board_id, thread_id, is_abone, res_order, client_info)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(res_id)
    .bind("名無しさん")
    .bind("")
    .bind(body)
    .bind(Utc::now())
    .bind("test-id")
    .bind("127.0.0.1")
    .bind(authed_token_id)
    .bind(board_id)
    .bind(thread_id)
    .bind(false)
    .bind(res_order)
    .bind(client_info.to_string())
    .execute(pool)
    .await
    .expect("Failed to create response");

    // Update thread response count
    sqlx::query(
        r#"
        UPDATE threads SET response_count = response_count + 1, last_modified_at = ?
        WHERE id = ?
        "#,
    )
    .bind(Utc::now())
    .bind(thread_id)
    .execute(pool)
    .await
    .expect("Failed to update thread response count");

    res_id
}

/// Encode form data to Shift-JIS URL encoding
pub fn encode_sjis_form(params: &[(&str, &str)]) -> String {
    use encoding_rs::SHIFT_JIS;

    params
        .iter()
        .map(|(key, value)| {
            let (encoded_value, _, _) = SHIFT_JIS.encode(value);
            let url_encoded: String = encoded_value
                .iter()
                .map(|&b| {
                    if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~'
                    {
                        (b as char).to_string()
                    } else {
                        format!("%{:02X}", b)
                    }
                })
                .collect();
            format!("{key}={url_encoded}")
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Decode Shift-JIS response
pub fn decode_sjis(bytes: &[u8]) -> String {
    use encoding_rs::SHIFT_JIS;
    let (decoded, _, _) = SHIFT_JIS.decode(bytes);
    decoded.into_owned()
}
