use std::{env, time::Duration};

use axum::{
    body::{Body, Bytes},
    extract::{MatchedPath, Path, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use axum_prometheus::PrometheusMetricLayer;
use eddist_core::domain::board::{validate_board_key, BoardInfo};
use handlebars::Handlebars;
use http::{HeaderMap, Request, StatusCode};
use tower_http::{
    catch_panic::CatchPanicLayer, classify::ServerErrorsFailureClass, timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{info_span, Span};

use crate::{
    middleware::user_restriction::user_restriction_middleware,
    repositories::{
        bbs_pubsub_repository::{RedisCreationEventRepository, RedisPubRepository},
        bbs_repository::BbsRepositoryImpl,
        idp_repository::IdpRepositoryImpl,
        notice_repository::NoticeRepositoryImpl,
        terms_repository::TermsRepositoryImpl,
        user_repository::UserRepositoryImpl,
        user_restriction_repository::UserRestrictionRepositoryImpl,
    },
    routes::{
        auth_code::{get_auth_code, post_auth_code},
        bbs_cgi::post_bbs_cgi,
        dat_routing::{get_dat_txt, get_kako_dat_txt},
        notice::{get_latest_notices, get_notice_by_slug, get_notices_paginated},
        subject_list::{get_subject_txt, get_subject_txt_with_metadent},
        terms::get_terms,
        user::user_routes,
    },
    services::{
        board_info_service::{BoardInfoServiceInput, BoardInfoServiceOutput},
        AppService, AppServiceContainer,
    },
    shiftjis::{SJisResponseBuilder, SjisContentType},
    utils::CsrfState,
};

#[derive(Clone)]
pub struct AppState {
    pub services: AppServiceContainer<
        BbsRepositoryImpl,
        UserRepositoryImpl,
        IdpRepositoryImpl,
        RedisPubRepository,
        UserRestrictionRepositoryImpl,
        RedisCreationEventRepository,
    >,
    pub notice_repo: NoticeRepositoryImpl,
    pub terms_repo: TermsRepositoryImpl,
    pub tinker_secret: String,
    pub template_engine: Handlebars<'static>,
}

impl AppState {
    pub fn get_container(
        &self,
    ) -> &AppServiceContainer<
        BbsRepositoryImpl,
        UserRepositoryImpl,
        IdpRepositoryImpl,
        RedisPubRepository,
        UserRestrictionRepositoryImpl,
        RedisCreationEventRepository,
    > {
        &self.services
    }

    pub fn tinker_secret(&self) -> &str {
        &self.tinker_secret
    }
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn get_setting_txt(
    Path(board_key): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let Ok(BoardInfoServiceOutput {
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
    }) = state
        .services
        .board_info()
        .execute(BoardInfoServiceInput { board_key })
        .await
    else {
        return Response::builder().status(404).body(Body::empty()).unwrap();
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

    let Ok(BoardInfoServiceOutput {
        board_info: BoardInfo { local_rules, .. },
        ..
    }) = state
        .services
        .board_info()
        .execute(BoardInfoServiceInput { board_key })
        .await
    else {
        return Response::builder().status(404).body(Body::empty()).unwrap();
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

async fn get_robots_txt() -> impl IntoResponse {
    let robot_txt = "User-agent: *\nAllow: /\nDisallow: /auth-code\n";
    SJisResponseBuilder::new((robot_txt as &str).into())
        .client_ttl(60 * 60 * 24)
        .server_ttl(60 * 60 * 24)
        .content_type(SjisContentType::TextPlain)
        .build()
        .into_response()
}

pub fn create_app(app_state: AppState, conn_mgr: redis::aio::ConnectionManager) -> Router {
    let enable_metrics = env::var("AXUM_METRICS") == Ok("true".to_string());
    let (prometheus_layer, metric_handle) = if enable_metrics {
        let (layer, handle) = PrometheusMetricLayer::pair();
        (Some(layer), Some(handle))
    } else {
        (None, None)
    };

    let app = Router::new()
        .route("/health-check", get(health_check))
        .route("/robots.txt", get(get_robots_txt))
        .route("/auth-code", get(get_auth_code).post(post_auth_code))
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
        .nest("/user", user_routes())
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

    // Add metrics route if enabled
    let app = if let Some(handle) = metric_handle {
        app.route("/metrics", get(move || async move { handle.render() }))
    } else {
        app
    };

    let app = app
        .with_state(app_state.clone())
        .layer(CatchPanicLayer::custom(|e| {
            tracing::error!("Panic: {e:?}");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal Server Error"))
                .unwrap()
        }))
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
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
        .layer(Extension(CsrfState::new(conn_mgr)));

    // Add user restriction middleware
    let app = app.layer(axum::middleware::from_fn_with_state(
        app_state.clone(),
        user_restriction_middleware,
    ));

    if let Some(layer) = prometheus_layer {
        app.layer(layer)
    } else {
        app
    }
}
