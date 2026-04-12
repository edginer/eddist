use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::StreamExt;
use sqlx::{MySqlPool, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

const BATCH_SIZE: usize = 500;

fn uid(s: &str) -> Uuid {
    Uuid::parse_str(s).expect("BIN_TO_UUID returned invalid UUID")
}

fn utc(dt: NaiveDateTime) -> DateTime<Utc> {
    dt.and_utc()
}

fn opt_utc(dt: Option<NaiveDateTime>) -> Option<DateTime<Utc>> {
    dt.map(|d| d.and_utc())
}

pub async fn run(mysql_url: &str, pg_url: &str) -> Result<()> {
    println!("Connecting to MySQL...");
    let mysql = MySqlPool::connect(mysql_url).await?;
    println!("Connecting to PostgreSQL...");
    let pg = PgPool::connect(pg_url).await?;
    println!("Migrating (archived_responses and archived_threads excluded)\n");

    // Insert authed_tokens first, with registered_user_id = NULL (circular FK with users).
    // After users are inserted, fix_registered_user_id() fills it in.
    migrate_authed_tokens(&mysql, &pg).await?;
    migrate_boards(&mysql, &pg).await?;
    migrate_boards_info(&mysql, &pg).await?;
    migrate_idps(&mysql, &pg).await?;
    migrate_users(&mysql, &pg).await?;
    fix_registered_user_id(&mysql, &pg).await?;
    migrate_admin_roles(&mysql, &pg).await?;
    migrate_admin_role_scopes(&mysql, &pg).await?;
    migrate_admin_users(&mysql, &pg).await?;
    migrate_caps(&mysql, &pg).await?;
    migrate_ng_words(&mysql, &pg).await?;
    migrate_threads(&mysql, &pg).await?;
    migrate_responses(&mysql, &pg).await?;
    migrate_boards_caps(&mysql, &pg).await?;
    migrate_boards_ng_words(&mysql, &pg).await?;
    migrate_user_idp_bindings(&mysql, &pg).await?;
    migrate_user_authed_tokens(&mysql, &pg).await?;
    migrate_user_restriction_rules(&mysql, &pg).await?;
    migrate_notices(&mysql, &pg).await?;
    migrate_terms(&mysql, &pg).await?;
    migrate_captcha_configs(&mysql, &pg).await?;
    migrate_server_settings(&mysql, &pg).await?;

    println!("\nDone.");
    Ok(())
}

// ─── authed_tokens ───────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlAuthedToken {
    id: String,
    token: String,
    origin_ip: String,
    reduced_origin_ip: String,
    writing_ua: String,
    authed_ua: Option<String>,
    auth_code: String,
    created_at: NaiveDateTime,
    authed_at: Option<NaiveDateTime>,
    validity: bool,
    last_wrote_at: Option<NaiveDateTime>,
    author_id_seed: Vec<u8>,
    require_user_registration: bool,
    asn_num: i32,
    additional_info: Option<String>,
    require_reauth: bool,
}

async fn migrate_authed_tokens(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let mut stream = sqlx::query_as::<_, MysqlAuthedToken>(
        "SELECT BIN_TO_UUID(id) AS id, token, origin_ip, reduced_origin_ip,
                writing_ua, authed_ua, auth_code, created_at, authed_at, validity,
                last_wrote_at, author_id_seed, require_user_registration, asn_num,
                CAST(additional_info AS CHAR) AS additional_info, require_reauth
         FROM authed_tokens",
    )
    .fetch(mysql);

    let mut total = 0u64;
    let mut batch: Vec<MysqlAuthedToken> = Vec::with_capacity(BATCH_SIZE);

    while let Some(row) = stream.next().await {
        batch.push(row?);
        if batch.len() >= BATCH_SIZE {
            insert_authed_tokens_batch(pg, &batch).await?;
            total += batch.len() as u64;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        total += batch.len() as u64;
        insert_authed_tokens_batch(pg, &batch).await?;
    }

    println!("authed_tokens:          {total}");
    Ok(())
}

async fn insert_authed_tokens_batch(pg: &PgPool, batch: &[MysqlAuthedToken]) -> Result<()> {
    // Pre-parse JSON outside the closure to allow ? propagation
    let additional_infos: Vec<Option<serde_json::Value>> = batch
        .iter()
        .map(|r| {
            r.additional_info
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
        })
        .collect();

    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO authed_tokens \
         (id, token, origin_ip, reduced_origin_ip, writing_ua, authed_ua, auth_code, \
          created_at, authed_at, validity, last_wrote_at, author_id_seed, \
          require_user_registration, asn_num, additional_info, require_reauth) ",
    );
    qb.push_values(
        batch.iter().zip(additional_infos.iter()),
        |mut b, (row, ai)| {
            b.push_bind(uid(&row.id))
                .push_bind(&row.token)
                .push_bind(&row.origin_ip)
                .push_bind(&row.reduced_origin_ip)
                .push_bind(&row.writing_ua)
                .push_bind(&row.authed_ua)
                .push_bind(&row.auth_code)
                .push_bind(utc(row.created_at))
                .push_bind(opt_utc(row.authed_at))
                .push_bind(row.validity)
                .push_bind(opt_utc(row.last_wrote_at))
                .push_bind(&row.author_id_seed)
                .push_bind(row.require_user_registration)
                .push_bind(row.asn_num)
                .push_bind(ai.clone())
                .push_bind(row.require_reauth);
        },
    );
    qb.push(" ON CONFLICT DO NOTHING");
    qb.build().execute(pg).await?;
    Ok(())
}

async fn fix_registered_user_id(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        registered_user_id: String,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT BIN_TO_UUID(id) AS id, BIN_TO_UUID(registered_user_id) AS registered_user_id
         FROM authed_tokens WHERE registered_user_id IS NOT NULL",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query("UPDATE authed_tokens SET registered_user_id = $1 WHERE id = $2")
            .bind(uid(&row.registered_user_id))
            .bind(uid(&row.id))
            .execute(pg)
            .await?;
    }
    println!("authed_tokens (reg_user_id fix): {count}");
    Ok(())
}

// ─── boards ──────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlBoard {
    id: String,
    name: String,
    board_key: String,
    default_name: String,
}

async fn migrate_boards(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlBoard>(
        "SELECT BIN_TO_UUID(id) AS id, name, board_key, default_name FROM boards",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO boards (id, name, board_key, default_name) VALUES ($1,$2,$3,$4)
             ON CONFLICT (id) DO UPDATE SET
               name=EXCLUDED.name, board_key=EXCLUDED.board_key, default_name=EXCLUDED.default_name",
        )
        .bind(uid(&row.id))
        .bind(&row.name)
        .bind(&row.board_key)
        .bind(&row.default_name)
        .execute(pg)
        .await?;
    }
    println!("boards:                 {count}");
    Ok(())
}

// ─── boards_info ─────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlBoardInfo {
    id: String,
    local_rules: String,
    base_thread_creation_span_sec: i32,
    base_response_creation_span_sec: i32,
    max_thread_name_byte_length: i32,
    max_author_name_byte_length: i32,
    max_email_byte_length: i32,
    max_response_body_byte_length: i32,
    max_response_body_lines: i32,
    threads_archive_cron: Option<String>,
    threads_archive_trigger_thread_count: Option<i32>,
    read_only: bool,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    force_metadent_type: Option<String>,
}

async fn migrate_boards_info(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlBoardInfo>(
        "SELECT BIN_TO_UUID(id) AS id, local_rules,
                base_thread_creation_span_sec, base_response_creation_span_sec,
                max_thread_name_byte_length, max_author_name_byte_length,
                max_email_byte_length, max_response_body_byte_length, max_response_body_lines,
                threads_archive_cron, threads_archive_trigger_thread_count,
                read_only, created_at, updated_at, force_metadent_type
         FROM boards_info",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO boards_info
             (id, local_rules, base_thread_creation_span_sec, base_response_creation_span_sec,
              max_thread_name_byte_length, max_author_name_byte_length, max_email_byte_length,
              max_response_body_byte_length, max_response_body_lines, threads_archive_cron,
              threads_archive_trigger_thread_count, read_only, created_at, updated_at, force_metadent_type)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
             ON CONFLICT (id) DO UPDATE SET
               local_rules=EXCLUDED.local_rules,
               threads_archive_cron=EXCLUDED.threads_archive_cron,
               threads_archive_trigger_thread_count=EXCLUDED.threads_archive_trigger_thread_count,
               read_only=EXCLUDED.read_only, updated_at=EXCLUDED.updated_at,
               force_metadent_type=EXCLUDED.force_metadent_type",
        )
        .bind(uid(&row.id))
        .bind(&row.local_rules)
        .bind(row.base_thread_creation_span_sec)
        .bind(row.base_response_creation_span_sec)
        .bind(row.max_thread_name_byte_length)
        .bind(row.max_author_name_byte_length)
        .bind(row.max_email_byte_length)
        .bind(row.max_response_body_byte_length)
        .bind(row.max_response_body_lines)
        .bind(&row.threads_archive_cron)
        .bind(row.threads_archive_trigger_thread_count)
        .bind(row.read_only)
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .bind(&row.force_metadent_type)
        .execute(pg)
        .await?;
    }
    println!("boards_info:            {count}");
    Ok(())
}

// ─── idps ────────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlIdp {
    id: String,
    idp_name: String,
    idp_display_name: String,
    idp_logo_svg: Option<String>,
    oidc_config_url: String,
    client_id: String,
    client_secret: String,
    enabled: bool,
}

async fn migrate_idps(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlIdp>(
        "SELECT BIN_TO_UUID(id) AS id, idp_name, idp_display_name, idp_logo_svg,
                oidc_config_url, client_id, client_secret, enabled
         FROM idps",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO idps (id, idp_name, idp_display_name, idp_logo_svg,
                               oidc_config_url, client_id, client_secret, enabled)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(&row.idp_name)
        .bind(&row.idp_display_name)
        .bind(&row.idp_logo_svg)
        .bind(&row.oidc_config_url)
        .bind(&row.client_id)
        .bind(&row.client_secret)
        .bind(row.enabled)
        .execute(pg)
        .await?;
    }
    println!("idps:                   {count}");
    Ok(())
}

// ─── users ───────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlUser {
    id: String,
    user_name: String,
    enabled: bool,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

async fn migrate_users(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlUser>(
        "SELECT BIN_TO_UUID(id) AS id, user_name, enabled, created_at, updated_at FROM users",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO users (id, user_name, enabled, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(&row.user_name)
        .bind(row.enabled)
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .execute(pg)
        .await?;
    }
    println!("users:                  {count}");
    Ok(())
}

// ─── admin_roles ─────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlAdminRole {
    id: String,
    role_name: String,
    role_description: String,
}

async fn migrate_admin_roles(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlAdminRole>(
        "SELECT BIN_TO_UUID(id) AS id, role_name, role_description FROM admin_roles",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO admin_roles (id, role_name, role_description)
             VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(&row.role_name)
        .bind(&row.role_description)
        .execute(pg)
        .await?;
    }
    println!("admin_roles:            {count}");
    Ok(())
}

// ─── admin_role_scopes ───────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlAdminRoleScope {
    id: String,
    role_id: String,
    scope_key: String,
}

async fn migrate_admin_role_scopes(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlAdminRoleScope>(
        "SELECT BIN_TO_UUID(id) AS id, BIN_TO_UUID(role_id) AS role_id, scope_key
         FROM admin_role_scopes",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO admin_role_scopes (id, role_id, scope_key)
             VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(uid(&row.role_id))
        .bind(&row.scope_key)
        .execute(pg)
        .await?;
    }
    println!("admin_role_scopes:      {count}");
    Ok(())
}

// ─── admin_users ─────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlAdminUser {
    id: String,
    user_role_id: String,
}

async fn migrate_admin_users(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlAdminUser>(
        "SELECT BIN_TO_UUID(id) AS id, BIN_TO_UUID(user_role_id) AS user_role_id FROM admin_users",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO admin_users (id, user_role_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(uid(&row.user_role_id))
        .execute(pg)
        .await?;
    }
    println!("admin_users:            {count}");
    Ok(())
}

// ─── caps ────────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlCap {
    id: String,
    name: String,
    description: String,
    password_hash: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

async fn migrate_caps(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlCap>(
        "SELECT BIN_TO_UUID(id) AS id, name, description, password_hash, created_at, updated_at
         FROM caps",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO caps (id, name, description, password_hash, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(&row.name)
        .bind(&row.description)
        .bind(&row.password_hash)
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .execute(pg)
        .await?;
    }
    println!("caps:                   {count}");
    Ok(())
}

// ─── ng_words ────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlNgWord {
    id: String,
    name: String,
    word: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

async fn migrate_ng_words(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlNgWord>(
        "SELECT BIN_TO_UUID(id) AS id, name, word, created_at, updated_at FROM ng_words",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO ng_words (id, name, word, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(&row.name)
        .bind(&row.word)
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .execute(pg)
        .await?;
    }
    println!("ng_words:               {count}");
    Ok(())
}

// ─── threads ─────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlThread {
    id: String,
    board_id: String,
    thread_number: i64,
    last_modified_at: NaiveDateTime,
    sage_last_modified_at: NaiveDateTime,
    title: String,
    authed_token_id: String,
    metadent: String,
    response_count: i32,
    no_pool: bool,
    active: bool,
    archived: bool,
    archive_converted: bool,
}

async fn migrate_threads(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlThread>(
        "SELECT BIN_TO_UUID(id) AS id, BIN_TO_UUID(board_id) AS board_id,
                thread_number, last_modified_at, sage_last_modified_at, title,
                BIN_TO_UUID(authed_token_id) AS authed_token_id, metadent,
                response_count, no_pool, active, archived, archive_converted
         FROM threads",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO threads
             (id, board_id, thread_number, last_modified_at, sage_last_modified_at, title,
              authed_token_id, metadent, response_count, no_pool, active, archived, archive_converted)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
             ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(uid(&row.board_id))
        .bind(row.thread_number)
        .bind(utc(row.last_modified_at))
        .bind(utc(row.sage_last_modified_at))
        .bind(&row.title)
        .bind(uid(&row.authed_token_id))
        .bind(&row.metadent)
        .bind(row.response_count)
        .bind(row.no_pool)
        .bind(row.active)
        .bind(row.archived)
        .bind(row.archive_converted)
        .execute(pg)
        .await?;
    }
    println!("threads:                {count}");
    Ok(())
}

// ─── responses ───────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlResponse {
    id: String,
    author_name: String,
    mail: String,
    body: String,
    created_at: NaiveDateTime,
    author_id: String,
    ip_addr: String,
    authed_token_id: String,
    board_id: String,
    thread_id: String,
    is_abone: bool,
    res_order: i32,
    client_info: String,
}

async fn migrate_responses(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let mut stream = sqlx::query_as::<_, MysqlResponse>(
        "SELECT BIN_TO_UUID(id) AS id, author_name, mail, body, created_at, author_id, ip_addr,
                BIN_TO_UUID(authed_token_id) AS authed_token_id,
                BIN_TO_UUID(board_id) AS board_id,
                BIN_TO_UUID(thread_id) AS thread_id,
                is_abone, res_order, CAST(client_info AS CHAR) AS client_info
         FROM responses",
    )
    .fetch(mysql);

    let mut total = 0u64;
    let mut batch: Vec<MysqlResponse> = Vec::with_capacity(BATCH_SIZE);

    while let Some(row) = stream.next().await {
        batch.push(row?);
        if batch.len() >= BATCH_SIZE {
            insert_responses_batch(pg, &batch).await?;
            total += batch.len() as u64;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        total += batch.len() as u64;
        insert_responses_batch(pg, &batch).await?;
    }

    println!("responses:              {total}");
    Ok(())
}

async fn insert_responses_batch(pg: &PgPool, batch: &[MysqlResponse]) -> Result<()> {
    let client_infos = batch
        .iter()
        .map(|r| serde_json::from_str::<serde_json::Value>(&r.client_info))
        .collect::<Result<Vec<_>, _>>()?;

    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO responses \
         (id, author_name, mail, body, created_at, author_id, ip_addr, \
          authed_token_id, board_id, thread_id, is_abone, res_order, client_info) ",
    );
    qb.push_values(batch.iter().zip(client_infos.iter()), |mut b, (row, ci)| {
        b.push_bind(uid(&row.id))
            .push_bind(&row.author_name)
            .push_bind(&row.mail)
            .push_bind(&row.body)
            .push_bind(utc(row.created_at))
            .push_bind(&row.author_id)
            .push_bind(&row.ip_addr)
            .push_bind(uid(&row.authed_token_id))
            .push_bind(uid(&row.board_id))
            .push_bind(uid(&row.thread_id))
            .push_bind(row.is_abone)
            .push_bind(row.res_order)
            .push_bind(ci);
    });
    qb.push(" ON CONFLICT DO NOTHING");
    qb.build().execute(pg).await?;
    Ok(())
}

// ─── boards_caps ─────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlBoardCap {
    id: String,
    board_id: String,
    cap_id: String,
}

async fn migrate_boards_caps(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlBoardCap>(
        "SELECT BIN_TO_UUID(id) AS id, BIN_TO_UUID(board_id) AS board_id,
                BIN_TO_UUID(cap_id) AS cap_id
         FROM boards_caps",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO boards_caps (id, board_id, cap_id) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(uid(&row.board_id))
        .bind(uid(&row.cap_id))
        .execute(pg)
        .await?;
    }
    println!("boards_caps:            {count}");
    Ok(())
}

// ─── boards_ng_words ─────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlBoardNgWord {
    id: String,
    board_id: String,
    ng_word_id: String,
}

async fn migrate_boards_ng_words(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlBoardNgWord>(
        "SELECT BIN_TO_UUID(id) AS id, BIN_TO_UUID(board_id) AS board_id,
                BIN_TO_UUID(ng_word_id) AS ng_word_id
         FROM boards_ng_words",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO boards_ng_words (id, board_id, ng_word_id)
             VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(uid(&row.board_id))
        .bind(uid(&row.ng_word_id))
        .execute(pg)
        .await?;
    }
    println!("boards_ng_words:        {count}");
    Ok(())
}

// ─── user_idp_bindings ───────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlUserIdpBinding {
    id: String,
    user_id: String,
    idp_id: String,
    idp_sub: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

async fn migrate_user_idp_bindings(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlUserIdpBinding>(
        "SELECT BIN_TO_UUID(id) AS id, BIN_TO_UUID(user_id) AS user_id,
                BIN_TO_UUID(idp_id) AS idp_id, idp_sub, created_at, updated_at
         FROM user_idp_bindings",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO user_idp_bindings (id, user_id, idp_id, idp_sub, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(uid(&row.user_id))
        .bind(uid(&row.idp_id))
        .bind(&row.idp_sub)
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .execute(pg)
        .await?;
    }
    println!("user_idp_bindings:      {count}");
    Ok(())
}

// ─── user_authed_tokens ──────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlUserAuthedToken {
    id: String,
    user_id: String,
    authed_token_id: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

async fn migrate_user_authed_tokens(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlUserAuthedToken>(
        "SELECT BIN_TO_UUID(id) AS id, BIN_TO_UUID(user_id) AS user_id,
                BIN_TO_UUID(authed_token_id) AS authed_token_id, created_at, updated_at
         FROM user_authed_tokens",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO user_authed_tokens (id, user_id, authed_token_id, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(uid(&row.user_id))
        .bind(uid(&row.authed_token_id))
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .execute(pg)
        .await?;
    }
    println!("user_authed_tokens:     {count}");
    Ok(())
}

// ─── user_restriction_rules ──────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlUserRestrictionRule {
    id: String,
    name: String,
    rule_type: String,
    rule_value: String,
    expires_at: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    created_by_email: String,
}

async fn migrate_user_restriction_rules(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlUserRestrictionRule>(
        "SELECT BIN_TO_UUID(id) AS id, name, rule_type, rule_value,
                expires_at, created_at, updated_at, created_by_email
         FROM user_restriction_rules",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        // rule_type must be cast to the PostgreSQL enum type
        sqlx::query(
            "INSERT INTO user_restriction_rules
             (id, name, rule_type, rule_value, expires_at, created_at, updated_at, created_by_email)
             VALUES ($1,$2,$3::restriction_rule_type,$4,$5,$6,$7,$8)
             ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(&row.name)
        .bind(&row.rule_type)
        .bind(&row.rule_value)
        .bind(opt_utc(row.expires_at))
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .bind(&row.created_by_email)
        .execute(pg)
        .await?;
    }
    println!("user_restriction_rules: {count}");
    Ok(())
}

// ─── notices ─────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlNotice {
    id: String,
    slug: String,
    title: String,
    content: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    published_at: NaiveDateTime,
    author_email: Option<String>,
}

async fn migrate_notices(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlNotice>(
        "SELECT BIN_TO_UUID(id) AS id, slug, title, content,
                created_at, updated_at, published_at, author_email
         FROM notices",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO notices (id, slug, title, content, created_at, updated_at, published_at, author_email)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(&row.slug)
        .bind(&row.title)
        .bind(&row.content)
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .bind(utc(row.published_at))
        .bind(&row.author_email)
        .execute(pg)
        .await?;
    }
    println!("notices:                {count}");
    Ok(())
}

// ─── terms ───────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlTerm {
    id: String,
    content: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    updated_by: Option<String>,
}

async fn migrate_terms(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlTerm>(
        "SELECT BIN_TO_UUID(id) AS id, content, created_at, updated_at, updated_by FROM terms",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO terms (id, content, created_at, updated_at, updated_by)
             VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT (id) DO UPDATE SET
               content=EXCLUDED.content, updated_at=EXCLUDED.updated_at, updated_by=EXCLUDED.updated_by",
        )
        .bind(uid(&row.id))
        .bind(&row.content)
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .bind(&row.updated_by)
        .execute(pg)
        .await?;
    }
    println!("terms:                  {count}");
    Ok(())
}

// ─── captcha_configs ─────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlCaptchaConfig {
    id: String,
    name: String,
    provider: String,
    site_key: String,
    secret: String,
    base_url: Option<String>,
    widget_form_field_name: Option<String>,
    widget_script_url: Option<String>,
    widget_html: Option<String>,
    widget_script_handler: Option<String>,
    capture_fields: Option<String>,
    verification: Option<String>,
    is_active: bool,
    display_order: i32,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    updated_by: Option<String>,
    endpoint_usage: String,
}

async fn migrate_captcha_configs(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlCaptchaConfig>(
        "SELECT BIN_TO_UUID(id) AS id, name, provider, site_key, secret, base_url,
                widget_form_field_name, widget_script_url, widget_html, widget_script_handler,
                CAST(capture_fields AS CHAR) AS capture_fields,
                CAST(verification AS CHAR) AS verification,
                is_active, display_order, created_at, updated_at, updated_by, endpoint_usage
         FROM captcha_configs",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        let capture_fields: Option<serde_json::Value> = row
            .capture_fields
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());
        let verification: Option<serde_json::Value> = row
            .verification
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());

        sqlx::query(
            "INSERT INTO captcha_configs
             (id, name, provider, site_key, secret, base_url,
              widget_form_field_name, widget_script_url, widget_html, widget_script_handler,
              capture_fields, verification, is_active, display_order,
              created_at, updated_at, updated_by, endpoint_usage)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
             ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(&row.name)
        .bind(&row.provider)
        .bind(&row.site_key)
        .bind(&row.secret)
        .bind(&row.base_url)
        .bind(&row.widget_form_field_name)
        .bind(&row.widget_script_url)
        .bind(&row.widget_html)
        .bind(&row.widget_script_handler)
        .bind(capture_fields)
        .bind(verification)
        .bind(row.is_active)
        .bind(row.display_order)
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .bind(&row.updated_by)
        .bind(&row.endpoint_usage)
        .execute(pg)
        .await?;
    }
    println!("captcha_configs:        {count}");
    Ok(())
}

// ─── server_settings ─────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MysqlServerSetting {
    id: String,
    setting_key: String,
    value: String,
    description: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

async fn migrate_server_settings(mysql: &MySqlPool, pg: &PgPool) -> Result<()> {
    let rows = sqlx::query_as::<_, MysqlServerSetting>(
        "SELECT BIN_TO_UUID(id) AS id, setting_key, value, description, created_at, updated_at
         FROM server_settings",
    )
    .fetch_all(mysql)
    .await?;

    let count = rows.len();
    for row in &rows {
        sqlx::query(
            "INSERT INTO server_settings (id, setting_key, value, description, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING",
        )
        .bind(uid(&row.id))
        .bind(&row.setting_key)
        .bind(&row.value)
        .bind(&row.description)
        .bind(utc(row.created_at))
        .bind(utc(row.updated_at))
        .execute(pg)
        .await?;
    }
    println!("server_settings:        {count}");
    Ok(())
}
