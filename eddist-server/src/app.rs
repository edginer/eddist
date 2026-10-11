use std::{sync::Arc, time::Duration};

use anyhow::Context as _;

use axum::{
    Extension, Json, Router,
    body::{Body, Bytes},
    extract::{MatchedPath, Path, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_prometheus::PrometheusMetricLayer;
use eddist_core::domain::board::{BoardInfo, validate_board_key};
use handlebars::Handlebars;
use http::{HeaderMap, Request, StatusCode};
use tokio::task::JoinHandle;
use tower_http::{
    catch_panic::CatchPanicLayer, classify::ServerErrorsFailureClass, timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{Span, info_span};

use crate::{
    config::AppConfig,
    context::AppContext,
    domain::service::{
        bbscgi_auth_service::UserCreationRateLimit, board_info_service::BoardInfoCache,
    },
    middleware::{
        ng_id_rate_limit::ng_id_rate_limit_middleware,
        not_found_rate_limit::{NotFoundPenaltyCache, not_found_rate_limit_middleware},
        user_restriction::user_restriction_middleware,
    },
    repositories::{
        bbs_pubsub_repository::{RedisCreationEventRepository, RedisPubRepository},
        bbs_repository::BbsRepositoryImpl,
        captcha_config_repository::CaptchaConfigRepositoryImpl,
        idp_repository::IdpRepositoryImpl,
        notice_repository::NoticeRepositoryImpl,
        server_settings_repository::ServerSettingsRepositoryImpl,
        stats_repository::StatsRepositoryImpl,
        terms_repository::TermsRepositoryImpl,
        user_repository::UserRepositoryImpl,
        user_restriction_repository::UserRestrictionRepositoryImpl,
    },
    routes::{
        auth_code::{get_auth_code, post_auth_code},
        bbs_cgi::post_bbs_cgi,
        dat_routing::{get_dat_txt, get_kako_dat_txt},
        ng_id::{
            delete_ng_ids, delete_thread_metadents, get_shared_ng, post_ng_id, post_thread_metadent,
        },
        notice::{get_latest_notices, get_notice_by_slug, get_notices_paginated},
        re_auth::{get_re_auth, post_re_auth},
        safe_mode::get_unsafe_thread_ids,
        stats::get_stats,
        subject_list::{get_subject_txt, get_subject_txt_with_metadent},
        terms::get_terms,
        user::user_routes,
    },
    services::{
        AppService, AppServiceContainer, BoardLookupError, PubSubRepos,
        board_info_service::{BoardInfoServiceInput, BoardInfoServiceOutput},
        captcha_config_cache::CaptchaConfigCache,
        kako_thread_retrieval_service::KakoThreadRetrievalService,
        server_settings_cache::{ServerSettingKey, ServerSettingsCache},
        snapshot_cache::spawn_refresh_task,
        stats_counter::StatsCounter,
        user_restriction_service::UserRestrictionCache,
    },
    shiftjis::{SJisResponseBuilder, SjisContentType},
    template::load_template_engine,
    utils::CsrfState,
};

const SETTINGS_REFRESH_INTERVAL: Duration = Duration::from_secs(300);
const STATS_FLUSH_INTERVAL: Duration = Duration::from_secs(30);

pub type Services = AppServiceContainer<
    BbsRepositoryImpl,
    UserRepositoryImpl,
    IdpRepositoryImpl,
    RedisPubRepository,
    RedisCreationEventRepository,
>;

#[derive(Clone)]
pub struct AppState {
    pub services: Services,
    pub notice_repo: NoticeRepositoryImpl,
    pub terms_repo: TermsRepositoryImpl,
    pub stats_repo: StatsRepositoryImpl,
    pub template_engine: Arc<Handlebars<'static>>,
    pub ctx: AppContext,
    pub redis_conn: redis::aio::ConnectionManager,
    pub not_found_penalty_cache: NotFoundPenaltyCache,
    _background_tasks: Arc<BackgroundTasks>,
}

impl AppState {
    pub fn get_container(&self) -> &Services {
        &self.services
    }
}

/// Refresh tasks stop once every clone of the owning `AppState` (and thus the router) is gone.
struct BackgroundTasks(Vec<JoinHandle<()>>);

impl Drop for BackgroundTasks {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

pub struct AppDependencies {
    pub pool: sqlx::MySqlPool,
    pub redis_conn: redis::aio::ConnectionManager,
    pub s3_client: aws_sdk_s3::Client,
    pub s3_bucket_name: String,
}

pub struct App {
    pub router: Router,
    pub state: AppState,
}

impl App {
    pub async fn flush_stats(&self) -> anyhow::Result<()> {
        self.state.ctx.stats.flush_now(&self.state.stats_repo).await
    }
}

/// Requests are only served after every cache has completed its first load; any initial
/// load failure aborts startup instead of serving with empty settings or rules.
pub async fn build_app(config: AppConfig, deps: AppDependencies) -> anyhow::Result<App> {
    let AppDependencies {
        pool,
        redis_conn,
        s3_client,
        s3_bucket_name,
    } = deps;

    let server_settings_repo = ServerSettingsRepositoryImpl::new(pool.clone());
    let captcha_config_repo = CaptchaConfigRepositoryImpl::new(pool.clone());
    let user_restriction_repo = UserRestrictionRepositoryImpl::new(pool.clone());
    let stats_repo = StatsRepositoryImpl::new(pool.clone());

    let (server_settings, captcha_configs, user_restrictions) = tokio::try_join!(
        async {
            ServerSettingsCache::load(&server_settings_repo)
                .await
                .context("failed to load server settings")
        },
        async {
            CaptchaConfigCache::load(&captcha_config_repo)
                .await
                .context("failed to load captcha configs")
        },
        async {
            UserRestrictionCache::load(&user_restriction_repo)
                .await
                .context("failed to load user restriction rules")
        },
    )?;

    let ctx = AppContext {
        config: Arc::new(config),
        server_settings,
        captcha_configs,
        user_restrictions,
        board_info: BoardInfoCache::default(),
        stats: StatsCounter::default(),
        user_creation_rate_limit: UserCreationRateLimit::default(),
    };

    let background_tasks = BackgroundTasks(vec![
        spawn_refresh_task("server_settings", SETTINGS_REFRESH_INTERVAL, {
            let cache = ctx.server_settings.clone();
            move || {
                let (cache, repo) = (cache.clone(), server_settings_repo.clone());
                async move { cache.refresh(&repo).await }
            }
        }),
        spawn_refresh_task("captcha_configs", SETTINGS_REFRESH_INTERVAL, {
            let cache = ctx.captcha_configs.clone();
            move || {
                let (cache, repo) = (cache.clone(), captcha_config_repo.clone());
                async move { cache.refresh(&repo).await }
            }
        }),
        spawn_refresh_task("user_restrictions", SETTINGS_REFRESH_INTERVAL, {
            let cache = ctx.user_restrictions.clone();
            move || {
                let (cache, repo) = (cache.clone(), user_restriction_repo.clone());
                async move { cache.refresh(&repo).await }
            }
        }),
        ctx.stats
            .spawn_flush_task(stats_repo.clone(), STATS_FLUSH_INTERVAL),
    ]);

    let services = AppServiceContainer::new(
        BbsRepositoryImpl::new(pool.clone()),
        UserRepositoryImpl::new(pool.clone()),
        IdpRepositoryImpl::new(pool.clone()),
        redis_conn.clone(),
        PubSubRepos {
            pub_repo: RedisPubRepository::new(redis_conn.clone()),
            event_repo: RedisCreationEventRepository::new(redis_conn.clone()),
        },
        KakoThreadRetrievalService::new(s3_client, s3_bucket_name),
        &ctx,
    );

    let state = AppState {
        services,
        notice_repo: NoticeRepositoryImpl::new(pool.clone()),
        terms_repo: TermsRepositoryImpl::new(pool.clone()),
        stats_repo,
        template_engine: Arc::new(load_template_engine()),
        ctx,
        redis_conn,
        not_found_penalty_cache: NotFoundPenaltyCache::new(),
        _background_tasks: Arc::new(background_tasks),
    };

    Ok(App {
        router: create_app(state.clone()),
        state,
    })
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

fn board_lookup_error_response(error: BoardLookupError) -> Response {
    match error {
        BoardLookupError::NotFound => Response::builder().status(404).body(Body::empty()).unwrap(),
        BoardLookupError::Other(e) => {
            tracing::error!(error = ?e, "failed to get board info");
            Response::builder().status(500).body(Body::empty()).unwrap()
        }
    }
}

async fn get_setting_txt(
    Path(board_key): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let BoardInfoServiceOutput {
        board_key,
        name,
        default_name,
        board_info:
            BoardInfo {
                max_thread_name_byte_length,
                max_author_name_byte_length,
                max_email_byte_length,
                max_response_body_byte_length,
                max_response_body_lines,
                ..
            },
        ..
    } = match state
        .services
        .board_info()
        .execute(BoardInfoServiceInput { board_key })
        .await
    {
        Ok(output) => output,
        Err(e) => return board_lookup_error_response(e),
    };
    let max_response_body_lines = max_response_body_lines / 2;

    let setting_txt = state
        .template_engine
        .render(
            "setting-txt.get",
            &serde_json::json!({
                "board_key": board_key,
                "name": name,
                "default_name": default_name,
                "max_thread_name_byte_length": max_thread_name_byte_length,
                "max_author_name_byte_length": max_author_name_byte_length,
                "max_email_byte_length": max_email_byte_length,
                "max_response_body_byte_length": max_response_body_byte_length,
                "max_response_body_lines": max_response_body_lines,
            }),
        )
        .unwrap();

    SJisResponseBuilder::new((&setting_txt as &str).into())
        .client_ttl(120)
        .server_ttl(300)
        .content_type(SjisContentType::TextPlain)
        .build()
        .into_response()
}

async fn get_head_txt(
    Path(board_key): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let BoardInfoServiceOutput {
        board_info: BoardInfo { local_rules, .. },
        ..
    } = match state
        .services
        .board_info()
        .execute(BoardInfoServiceInput { board_key })
        .await
    {
        Ok(output) => output,
        Err(e) => return board_lookup_error_response(e),
    };

    SJisResponseBuilder::new((&local_rules as &str).into())
        .client_ttl(120)
        .server_ttl(300)
        .content_type(SjisContentType::TextPlain)
        .build()
        .into_response()
}

async fn get_api_boards(State(state): State<AppState>) -> impl IntoResponse {
    let svc = state.get_container().list_boards();
    let boards = svc.execute(()).await.unwrap();

    let mut resp = Json(boards).into_response();
    resp.headers_mut()
        .insert("Cache-Control", "s-maxage=300".parse().unwrap());
    resp
}

async fn get_api_client_config(State(state): State<AppState>) -> impl IntoResponse {
    let settings = state.ctx.server_settings.snapshot();
    let enable_user_registration = settings.is_enabled(ServerSettingKey::EnableIdpLinking);
    let enable_safe_mode = settings.is_enabled(ServerSettingKey::EnableSafeMode);

    let mut resp = Json(serde_json::json!({
        "enable_user_registration": enable_user_registration,
        "enable_safe_mode": enable_safe_mode,
    }))
    .into_response();
    resp.headers_mut()
        .insert("Cache-Control", "s-maxage=60".parse().unwrap());
    resp
}

async fn get_robots_txt() -> impl IntoResponse {
    let robot_txt = "User-agent: *\nAllow: /\nDisallow: /auth-code\n";
    SJisResponseBuilder::new((robot_txt as &str).into())
        .client_ttl(60 * 60 * 24)
        .server_ttl(60 * 60 * 24)
        .content_type(SjisContentType::TextPlain)
        .build()
        .into_response()
}

fn create_app(app_state: AppState) -> Router {
    let metrics = app_state.ctx.config.metrics;
    let (prometheus_layer, metric_handle) = if !metrics.enabled {
        (None, None)
    } else {
        let (layer, handle) = PrometheusMetricLayer::pair();
        let layer = if metrics.http_layer {
            Some(layer)
        } else {
            None
        };
        (layer, Some(handle))
    };

    let ng_id_routes = Router::new()
        .route(
            "/api/{boardKey}/ng-ids",
            post(post_ng_id).delete(delete_ng_ids),
        )
        .route(
            "/api/{boardKey}/ng-thread-metadents",
            post(post_thread_metadent).delete(delete_thread_metadents),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            ng_id_rate_limit_middleware,
        ));

    let app = Router::new()
        .route("/health-check", get(health_check))
        .route(
            "/metrics",
            get(move || async move {
                metric_handle
                    .as_ref()
                    .map(|h| h.render())
                    .unwrap_or_default()
            }),
        )
        .route("/robots.txt", get(get_robots_txt))
        .route("/auth-code", get(get_auth_code).post(post_auth_code))
        .route("/re-auth", get(get_re_auth).post(post_re_auth))
        .route("/test/bbs.cgi", post(post_bbs_cgi))
        .route("/{boardKey}/subject.txt", get(get_subject_txt))
        .route(
            "/{boardKey}/subject-metadent.txt",
            get(get_subject_txt_with_metadent),
        )
        .route("/{boardKey}/head.txt", get(get_head_txt))
        .route("/{boardKey}/SETTING.TXT", get(get_setting_txt))
        .route("/{boardKey}/dat/{threadId}", get(get_dat_txt))
        .route(
            "/{boardKey}/kako/{th4}/{th5}/{threadId}",
            get(get_kako_dat_txt),
        )
        .route("/api/terms", get(get_terms))
        .route("/api/boards", get(get_api_boards))
        .route("/api/notices/latest", get(get_latest_notices))
        .route("/api/notices", get(get_notices_paginated))
        .route("/api/notices/{slug}", get(get_notice_by_slug))
        .route("/api/client-config", get(get_api_client_config))
        .route("/api/stats", get(get_stats))
        .route(
            "/api/{boardKey}/unsafe-thread-ids",
            get(get_unsafe_thread_ids),
        )
        // Kept out of `ng_id_routes`: that layer budgets mutations per IP, and reads
        // happen on every board page load.
        .route("/api/{boardKey}/shared-ng", get(get_shared_ng))
        .merge(ng_id_routes)
        .nest("/user", user_routes(&app_state))
        .route(
            "/{boardKey}",
            get(|| async move { Response::builder().status(404).body(Body::empty()).unwrap() }),
        )
        .route(
            "/",
            get(|| async move { Response::builder().status(404).body(Body::empty()).unwrap() }),
        )
        .route(
            "/{boardKey}/{threadId}",
            get(|| async move { Response::builder().status(404).body(Body::empty()).unwrap() }),
        )
        .route(
            "/test/read.cgi/{boardKey}/{threadId}",
            get(
                |Path((board_key, thread_id)): Path<(String, String)>| async move {
                    Redirect::permanent(&format!("/{}/{}", board_key, thread_id))
                },
            ),
        )
        .route(
            "/test/read.cgi/{boardKey}/{threadId}/{*pos}",
            get(
                |Path((board_key, thread_id, _)): Path<(String, String, String)>| async move {
                    Redirect::permanent(&format!("/{}/{}", board_key, thread_id))
                },
            ),
        );

    let app = app
        .with_state(app_state.clone())
        .layer(CatchPanicLayer::custom(|e| {
            tracing::error!("Panic: {e:?}");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal Server Error"))
                .unwrap()
        }))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(10),
        ))
        // NOTE: comment out for now (is is not confirmed in twinkle environment)
        // .layer(CompressionLayer::new())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        matched_path,
                        some_other_field = tracing::field::Empty,
                    )
                })
                .on_request(|_request: &Request<_>, _span: &Span| {
                    // You can use `_span.record("some_other_field", value)` in one of these
                    // closures to attach a value to the initially empty field in the info_span
                    // created above.
                })
                .on_response(|_response: &Response, _latency: Duration, _span: &Span| {
                    // ...
                })
                .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {
                    // ...
                })
                .on_eos(
                    |_trailers: Option<&HeaderMap>, _stream_duration: Duration, _span: &Span| {
                        // ...
                    },
                )
                .on_failure(
                    |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        // ...
                    },
                ),
        )
        .layer(Extension(CsrfState::new(app_state.redis_conn.clone())));

    // Add user restriction middleware
    let app = app.layer(axum::middleware::from_fn_with_state(
        app_state.clone(),
        user_restriction_middleware,
    ));

    let app = if let Some(layer) = prometheus_layer {
        app.layer(layer)
    } else {
        app
    };

    // Add 404 rate limit middleware as the outermost layer so penalized IPs are
    // rejected as cheaply as possible, before any other request processing.
    app.layer(axum::middleware::from_fn_with_state(
        app_state,
        not_found_rate_limit_middleware,
    ))
}
