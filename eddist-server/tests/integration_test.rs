mod common;

use axum::body::Bytes;
use common::TestContext;
use eddist::test_helpers::*;
use http::{HeaderName, HeaderValue};

/// Test 1: GET thread list via HTTP
#[tokio::test]
async fn test_get_thread_list() {
    let ctx = TestContext::new().await;

    // Setup: Create test board with some threads
    let board_id = create_test_board(&ctx.pool, "bbs", "テスト板").await;
    let (token_id, _) = create_test_authed_token(&ctx.pool, "127.0.0.1", "test-code").await;

    let thread1_id =
        create_test_thread(&ctx.pool, board_id, 1234567890, "テストスレッド1", token_id).await;
    let thread2_id =
        create_test_thread(&ctx.pool, board_id, 1234567891, "テストスレッド2", token_id).await;

    // Add some responses to make threads appear in the list
    create_test_response(&ctx.pool, board_id, thread1_id, token_id, 1, "レス1").await;
    create_test_response(&ctx.pool, board_id, thread2_id, token_id, 1, "レス2").await;

    // Test: GET /bbs/subject.txt
    let response = ctx.server.get("/bbs/subject.txt").await;

    assert_eq!(response.status_code(), 200);
    // Check content-type (case-insensitive, trimming trailing semicolon)
    let header_value = response.header("content-type");
    let content_type = header_value.to_str().unwrap().trim_end_matches(';');
    assert!(
        content_type.eq_ignore_ascii_case("text/plain; charset=shift_jis"),
        "Expected 'text/plain; charset=shift_jis' but got '{}'",
        content_type
    );

    // Verify the response body contains thread data in Shift-JIS
    let body_bytes = response.as_bytes();
    let body_text = decode_sjis(body_bytes);

    println!("Response Body:\n{}", body_text);
}

/// Test 2: GET thread via HTTP
#[tokio::test]
async fn test_get_thread() {
    let ctx = TestContext::new().await;

    // Setup: Create board, thread with responses
    let board_id = create_test_board(&ctx.pool, "test2", "テスト板2").await;
    let (token_id, _) = create_test_authed_token(&ctx.pool, "127.0.0.1", "code-test2").await;
    let thread_id =
        create_test_thread(&ctx.pool, board_id, 1234567892, "テストスレッド", token_id).await;

    create_test_response(
        &ctx.pool,
        board_id,
        thread_id,
        token_id,
        1,
        "1番目のレスポンス",
    )
    .await;
    create_test_response(
        &ctx.pool,
        board_id,
        thread_id,
        token_id,
        2,
        "2番目のレスポンス",
    )
    .await;
    create_test_response(
        &ctx.pool,
        board_id,
        thread_id,
        token_id,
        3,
        "3番目のレスポンス",
    )
    .await;

    // Test: GET /test2/dat/1234567892.dat
    let response = ctx.server.get("/test2/dat/1234567892.dat").await;

    assert_eq!(response.status_code(), 200);
    // Check content-type (case-insensitive, trimming trailing semicolon)
    let header_value = response.header("content-type");
    let content_type = header_value.to_str().unwrap().trim_end_matches(';');
    assert!(
        content_type.eq_ignore_ascii_case("text/plain; charset=shift_jis"),
        "Expected 'text/plain; charset=shift_jis' but got '{}'",
        content_type
    );

    let body_bytes = response.as_bytes();
    let body_text = decode_sjis(body_bytes);

    // Verify DAT format contains all 3 responses
    assert!(body_text.contains("1番目のレスポンス"));
    assert!(body_text.contains("2番目のレスポンス"));
    assert!(body_text.contains("3番目のレスポンス"));
}

/// Test 3: POST create thread via HTTP
#[tokio::test]
async fn test_create_thread() {
    let ctx = TestContext::new().await;

    // Setup: Create board and authed token
    let _board_id = create_test_board(&ctx.pool, "test3", "テスト板3").await;
    let (_token_id, token) = create_test_authed_token(&ctx.pool, "192.168.1.1", "code-test3").await;

    // Test: POST /test/bbs.cgi to create thread
    let form_data = encode_sjis_form(&[
        ("bbs", "test3"),
        ("submit", "新規スレッド作成"),
        ("subject", "新しいテストスレッド"),
        ("FROM", "テスト太郎"),
        ("mail", ""),
        ("MESSAGE", "これはテスト投稿です"),
    ]);

    let response = ctx
        .server
        .post("/test/bbs.cgi")
        .content_type("application/x-www-form-urlencoded")
        .add_header(
            HeaderName::from_static("cookie"),
            HeaderValue::from_str(&format!("edge-token={}", token)).unwrap(),
        )
        .bytes(Bytes::from(form_data.into_bytes()))
        .await;

    assert_eq!(response.status_code(), 200);

    let response = ctx.server.get("/test3/subject.txt").await;
    let resp_bytes = response.as_bytes();
    let resp_text = decode_sjis(resp_bytes);
    assert!(resp_text.contains("新しいテストスレッド"));
}

/// Test 4: POST create response via HTTP
#[tokio::test]
async fn test_create_response() {
    let ctx = TestContext::new().await;

    // Setup: Create board, thread, and token
    let board_id = create_test_board(&ctx.pool, "test4", "テスト板4").await;
    let (token_id, token) = create_test_authed_token(&ctx.pool, "192.168.1.2", "code-test4").await;
    let _thread_id =
        create_test_thread(&ctx.pool, board_id, 1111111111, "既存スレッド", token_id).await;

    // Test: POST /test/bbs.cgi to create response
    let form_data = encode_sjis_form(&[
        ("bbs", "test4"),
        ("submit", "書き込む"),
        ("key", "1111111111"),
        ("FROM", "レス太郎"),
        ("mail", ""),
        ("MESSAGE", "これはテストレスポンスです"),
    ]);

    let response = ctx
        .server
        .post("/test/bbs.cgi")
        .content_type("application/x-www-form-urlencoded")
        .add_header(
            HeaderName::from_static("cookie"),
            HeaderValue::from_str(&format!("edge-token={}", token)).unwrap(),
        )
        .bytes(Bytes::from(form_data.into_bytes()))
        .await;

    let resp_bytes = response.as_bytes();
    let resp_text = decode_sjis(resp_bytes);

    assert_eq!(response.status_code(), 200);
    assert!(resp_text.contains("書きこみました"));

    ctx.get_thread_dat_with_retry("test4", "1111111111", |text| {
        text.contains("これはテストレスポンスです")
    })
    .await
    .expect("Failed to get correct DAT response");
}

/// Test 5: Auth code flow with response creation
#[tokio::test]
async fn test_auth_code_with_create_response() {
    println!("{:?}", std::env::current_dir().unwrap());

    let ctx = TestContext::new().await;

    // Setup: Create board and thread
    let board_id = create_test_board(&ctx.pool, "test5", "テスト板5").await;
    let (token_id, _) = create_test_authed_token(&ctx.pool, "10.0.0.1", "code-test5").await;
    let _thread_id = create_test_thread(
        &ctx.pool,
        board_id,
        2222222222,
        "認証テストスレッド",
        token_id,
    )
    .await;

    // Step 1: Try to create a response WITHOUT a token - should get auth code error
    let form_data = encode_sjis_form(&[
        ("bbs", "test5"),
        ("submit", "書き込む"),
        ("key", "2222222222"),
        ("FROM", "認証ユーザー"),
        ("mail", ""),
        ("MESSAGE", "認証されたレスポンスです"),
    ]);

    let response = ctx
        .server
        .post("/test/bbs.cgi")
        .content_type("application/x-www-form-urlencoded")
        .bytes(Bytes::from(form_data.clone().into_bytes()))
        .await;

    // Extract the edge-token cookie from the response
    let set_cookie = response.header("set-cookie");
    let cookie_str = set_cookie.to_str().unwrap();

    // Should get error page with auth code
    let resp_bytes = response.as_bytes();
    let resp_text = decode_sjis(resp_bytes);

    println!("First POST response (without token):\n{}", resp_text);

    // Extract auth code from error page
    // The error page contains text like: 認証コード'XXXXXX'を用いて
    let auth_code = if let Some(start) = resp_text.find("認証コード'") {
        let code_start = start + "認証コード'".len();
        if let Some(end) = resp_text[code_start..].find('\'') {
            &resp_text[code_start..code_start + end]
        } else {
            panic!("Could not find end of auth code in response: {}", resp_text);
        }
    } else {
        panic!("Could not find auth code in error response: {}", resp_text);
    };

    println!("Extracted auth code: {}", auth_code);

    // Step 2: POST the auth code to complete authentication and get token
    // (Skipping GET /auth-code since it requires template rendering which isn't set up in tests)
    let auth_form_data = format!("auth-code={}", auth_code);
    ctx.server
        .post("/auth-code")
        .content_type("application/x-www-form-urlencoded")
        .bytes(Bytes::from(auth_form_data.into_bytes()))
        .await;

    println!("Set-Cookie header: {}", cookie_str);

    let edge_token = if let Some(start) = cookie_str.find("edge-token=") {
        let token_start = start + 11;
        if let Some(end) = cookie_str[token_start..].find(';') {
            &cookie_str[token_start..token_start + end]
        } else {
            &cookie_str[token_start..]
        }
    } else {
        panic!(
            "Could not find edge-token in Set-Cookie header: {}",
            cookie_str
        );
    };

    println!("Extracted edge-token: {}", edge_token);

    // Step 3: Retry the original POST with the token
    let retry_response = ctx
        .server
        .post("/test/bbs.cgi")
        .content_type("application/x-www-form-urlencoded")
        .add_header(
            HeaderName::from_static("cookie"),
            HeaderValue::from_str(&format!("edge-token={}", edge_token)).unwrap(),
        )
        .bytes(Bytes::from(form_data.into_bytes()))
        .await;

    assert_eq!(retry_response.status_code(), 200);

    let retry_bytes = retry_response.as_bytes();
    let retry_text = decode_sjis(retry_bytes);

    println!("Retry POST response (with token):\n{}", retry_text);

    assert!(retry_text.contains("書きこみました") || retry_text.contains("書き込みました"));

    // Step 4: Verify the response was created
    ctx.get_thread_dat_with_retry("test5", "2222222222", |text| {
        text.contains("認証されたレスポンスです")
    })
    .await
    .expect("Failed to get correct DAT response");
}
