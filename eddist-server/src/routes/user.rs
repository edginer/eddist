use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use eddist_core::utils::is_user_registration_enabled;
use http::{HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::json;

use crate::{
    services::{
        auth_with_code_user_page_service::AuthWithCodeUserPageServiceInput,
        user_authz_idp_callback_service::{
            CallbackKind, UserAuthzIdpCallbackServiceInput, UserAuthzIdpCallbackServiceOutput,
        },
        user_login_idp_redirection_service::UserLoginIdpRedirectionServiceInput,
        user_login_page_service::{UserLoginPageServiceInput, UserLoginPageServiceOutput},
        user_logout_service::UserLogoutServiceInput,
        user_page_service::{UserPageServiceInput, UserPageServiceOutput},
        user_reg_idp_redirection_service::UserRegIdpRedirectionServiceInput,
        user_reg_temp_url_service::{UserRegTempUrlServiceInput, UserRegTempUrlServiceOutput},
        AppService,
    },
    utils::{get_ua, CsrfState},
    AppState,
};

pub fn user_routes() -> Router<AppState> {
    let user_registration_enabled = is_user_registration_enabled();

    if user_registration_enabled {
        log::info!("User registration is enabled");
    } else {
        log::info!("User registration is disabled");
    }

    if user_registration_enabled {
        Router::new()
            .route("/", get(get_user_page))
            .route("/register/:tempUrlPath", get(get_user_register_temp_url))
            .route(
                "/register/authz/idp/:idpName",
                get(get_user_reg_redirect_to_idp_authz),
            )
            .route("/login", get(get_user_login))
            .route(
                "/login/authz/idp/:idpName",
                get(get_user_login_redirect_to_idp_authz),
            )
            .route("/logout", get(get_user_logout))
            .route("/auth/callback", get(get_user_authz_idp_callback))
            .route("/api/auth-code", post(post_auth_code_at_user_page))
            .layer(axum::middleware::from_fn(
                |req, next: axum::middleware::Next| async move {
                    let mut response = next.run(req).await;
                    response
                        .headers_mut()
                        .entry("Cache-Control")
                        .or_insert_with(|| HeaderValue::from_static("no-store"));
                    response
                },
            ))
    } else {
        Router::new()
    }
}

async fn get_user_page(
    State(state): State<AppState>,
    jar: CookieJar,
    csrf: Extension<CsrfState>,
) -> impl IntoResponse {
    let user_sid = jar.get("user-sid").map(|cookie| cookie.value().to_string());
    let Some(user_sid) = user_sid else {
        // TODO: more user-friendly error page
        return Response::builder()
            .status(302)
            .header("Location", "/user/login?utm_source=user-page")
            .body(Body::from("User is not registered"))
            .unwrap();
    };

    let Ok(UserPageServiceOutput { user }) = state
        .services
        .user_page()
        .execute(UserPageServiceInput {
            user_sid: user_sid.clone(),
        })
        .await
    else {
        return Response::builder()
            .status(302)
            .header(
                "Set-Cookie",
                HeaderValue::from_str(&reset_user_sid_cookie().to_string()).unwrap(),
            )
            .header("Location", "/user/login?utm_source=user-page")
            .body(Body::from("User not found"))
            .unwrap();
    };

    let csrf_token = csrf
        .generate_new_csrf_token("user-page", 60 * 60)
        .await
        .unwrap();

    log::info!("csrf token issued: {csrf_token}, user_sid: {user_sid}");

    let html = state
        .template_engine
        .render(
            "user-page-simple.get",
            &serde_json::json!({
                "user_name": user.user_name,
                "csrf_token": csrf_token,
            }),
        )
        .unwrap();

    Response::builder()
        .status(200)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "private, max-age=0")
        .header("X-Frame-Options", "DENY")
        .header("X-Content-Type-Options", "nosniff")
        .header("Referrer-Policy", "no-referrer")
        .body(Body::from(html))
        .unwrap()
}

async fn get_user_register_temp_url(
    State(state): State<AppState>,
    Path(temp_url_path): Path<String>,
    jar: CookieJar,
) -> impl IntoResponse {
    let user_sid = jar.get("user-sid").map(|cookie| cookie.value().to_string());

    let svc = state.get_container().user_reg_temp_url();
    let Ok(output) = svc
        .execute(UserRegTempUrlServiceInput {
            temp_url_path,
            user_sid,
        })
        .await
    else {
        return Response::builder()
            .status(200)
            .header("Cache-Control", "s-maxage=3600")
            .body(Body::from("User registration is not available"))
            .unwrap();
    };

    match output {
        UserRegTempUrlServiceOutput::NotFound => Response::builder()
            .status(404)
            .body(Body::from("Not found"))
            .unwrap(),
        UserRegTempUrlServiceOutput::Registered => Response::builder()
            .status(302)
            .header("Location", "/user/")
            .body(Body::empty())
            .unwrap(),
        UserRegTempUrlServiceOutput::NotRegistered {
            available_idps,
            state_cookie,
        } => {
            let html = state
                .template_engine
                .render(
                    "user-reg-temp-url.get",
                    &json!(
                        {
                            "available_idps": available_idps,
                        }
                    ),
                )
                .unwrap();

            let userreg_state_id_cookie = Cookie::build(("userreg-state-id", state_cookie))
                .path("/")
                .http_only(true)
                .secure(true)
                .max_age(time::Duration::seconds(60 * 3))
                .build();

            Response::builder()
                .header("Set-Cookie", reset_user_sid_cookie().to_string())
                .header("Set-Cookie", userreg_state_id_cookie.to_string())
                .status(200)
                .body(Body::from(html))
                .unwrap()
        }
    }
}

async fn get_user_reg_redirect_to_idp_authz(
    State(state): State<AppState>,
    Path(idp_name): Path<String>,
    jar: CookieJar,
) -> Response {
    if jar.get("user-sid").is_some() {
        return Response::builder()
            .status(400)
            .body(Body::from("User is already registered"))
            .unwrap();
    }

    let Some(user_reg_state_id) = jar
        .get("userreg-state-id")
        .map(|cookie| cookie.value().to_string())
    else {
        return Response::builder()
            .status(400)
            .body(Body::from("User registration state session is not found"))
            .unwrap();
    };

    let svc = state.get_container().user_reg_idp_redirection();

    let output = match svc
        .execute(UserRegIdpRedirectionServiceInput {
            idp_name,
            user_reg_state_id,
        })
        .await
    {
        Ok(o) => o,
        Err(e) if e.to_string().contains("user_reg_state_id not found") => {
            return Response::builder()
                .header("Set-Cookie", reset_userreg_state_id_cookie().to_string())
                .status(400)
                .body(Body::from("User registration state session is not found"))
                .unwrap();
        }
        Err(_) => {
            return Response::builder()
                .status(400)
                .body(Body::from("Failed to redirect to IDP"))
                .unwrap();
        }
    };

    Response::builder()
        .status(302)
        .header("Location", output.authz_url)
        .body(Body::empty())
        .unwrap()
}

async fn get_user_login(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user_cookie = jar.get("user-sid").map(|cookie| cookie.value().to_string());

    let svc = state.get_container().user_login_page();
    let Ok(output) = svc
        .execute(UserLoginPageServiceInput {
            user_sid: user_cookie,
        })
        .await
    else {
        return Response::builder()
            .status(200)
            .body(Body::from("User registration is not available"))
            .unwrap();
    };

    match output {
        UserLoginPageServiceOutput::LoggedIn => Response::builder()
            .status(302)
            .header("Location", "/user/")
            .body(Body::empty())
            .unwrap(),
        UserLoginPageServiceOutput::NotLoggedIn { available_idps } => {
            let html = state
                .template_engine
                .render(
                    "login-idp-selection.get",
                    &json!({ "available_idps": available_idps }),
                )
                .unwrap();

            Response::builder()
                .header("Set-Cookie", reset_user_sid_cookie().to_string())
                .header("Set-Cookie", reset_user_login_state_id_cookie().to_string())
                .status(200)
                .body(Body::from(html))
                .unwrap()
        }
    }
}

async fn get_user_login_redirect_to_idp_authz(
    State(state): State<AppState>,
    Path(idp_name): Path<String>,
    jar: CookieJar,
) -> Response {
    if jar.get("user-sid").is_some() {
        return Response::builder()
            .status(400)
            .body(Body::from("User is already logged in"))
            .unwrap();
    }

    let svc = state.get_container().user_login_idp_redirection();

    let output = match svc
        .execute(UserLoginIdpRedirectionServiceInput { idp_name })
        .await
    {
        Ok(o) => o,
        Err(_) => {
            return Response::builder()
                .status(400)
                .body(Body::from("Failed to redirect to IDP"))
                .unwrap();
        }
    };

    let user_login_state_id_cookie =
        Cookie::build(("user-login-state-id", &output.user_login_state_id))
            .path("/")
            .http_only(true)
            .secure(true)
            .max_age(time::Duration::seconds(60 * 15))
            .build();

    Response::builder()
        .status(302)
        .header(
            "Set-Cookie",
            HeaderValue::from_str(&user_login_state_id_cookie.to_string()).unwrap(),
        )
        .header("Location", output.authz_url)
        .body(Body::empty())
        .unwrap()
}

async fn get_user_logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let user_sid = jar.get("user-sid").map(|cookie| cookie.value().to_string());

    if let Some(user_sid) = user_sid {
        state
            .get_container()
            .user_logout()
            .execute(UserLogoutServiceInput { user_sid })
            .await
            .unwrap();
    }

    Response::builder()
        .status(302)
        .header("Set-Cookie", reset_user_sid_cookie().to_string())
        .header("Location", "/?utm_source=user-logout")
        .body(Body::empty())
        .unwrap()
}

#[derive(Debug, Clone, Deserialize)]
struct AuthzIdpCallbackQuery {
    code: String,
    state: String,
}

async fn get_user_authz_idp_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    query: axum::extract::Query<AuthzIdpCallbackQuery>,
) -> Response {
    let (state_cookie, callback_kind) =
        match (jar.get("userreg-state-id"), jar.get("user-login-state-id")) {
            (Some(reg_state_cookie), None) => (reg_state_cookie.value(), CallbackKind::Register),
            (None, Some(login_state_cookie)) => (login_state_cookie.value(), CallbackKind::Login),
            _ => {
                return Response::builder()
                    .status(400)
                    .header("Set-Cookie", reset_userreg_state_id_cookie().to_string())
                    .header("Set-Cookie", reset_user_login_state_id_cookie().to_string())
                    .body(Body::from("Invalid state"))
                    .unwrap();
            }
        };

    if state_cookie != query.state {
        return Response::builder()
            .status(400)
            .body(Body::from("Invalid state"))
            .unwrap();
    }

    let user_sid = match state
        .services
        .user_authz_idp_callback()
        .execute(UserAuthzIdpCallbackServiceInput {
            code: query.code.clone(),
            state_id: state_cookie.to_string(),
            callback_kind,
        })
        .await
    {
        Ok(UserAuthzIdpCallbackServiceOutput { user_sid }) => user_sid,
        Err(e) if e.to_string().contains("user not found") => {
            return Response::builder()
                .header("Set-Cookie", reset_user_login_state_id_cookie().to_string())
                .status(400)
                .body(Body::from("Failed to get user (maybe unregistered)"))
                .unwrap();
        }
        Err(e) => {
            log::error!("Failed to get user: {e}");
            return Response::builder()
                .header("Set-Cookie", reset_user_login_state_id_cookie().to_string())
                .header("Set-Cookie", reset_userreg_state_id_cookie().to_string())
                .status(400)
                .body(Body::from("Failed to get user".to_string()))
                .unwrap();
        }
    };

    let user_sid_cookie = Cookie::build(("user-sid", &user_sid))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::seconds(60 * 60 * 24 * 365))
        .build();

    Response::builder()
        .status(302)
        .header("Set-Cookie", user_sid_cookie.to_string())
        .header("Set-Cookie", reset_userreg_state_id_cookie().to_string())
        .header("Set-Cookie", reset_user_login_state_id_cookie().to_string())
        .header("Location", "/user/")
        .body(Body::empty())
        .unwrap()
}

#[derive(Debug, Clone, Deserialize)]
struct PostAuthCodeAtUserPage {
    auth_code: String,
}

async fn post_auth_code_at_user_page(
    headers: HeaderMap,
    State(state): State<AppState>,
    jar: CookieJar,
    csrf: Extension<CsrfState>,
    Json(PostAuthCodeAtUserPage { auth_code }): Json<PostAuthCodeAtUserPage>,
) -> impl IntoResponse {
    let csrf_token = headers.get("X-CSRF-Token").map(|x| x.to_str().unwrap());

    let Some(csrf_token) = csrf_token else {
        return Response::builder()
            .status(400)
            .body(Body::from("\"CSRF token is missing\""))
            .unwrap();
    };

    if !csrf.verify_csrf_token(csrf_token).await.unwrap_or(false) {
        return Response::builder()
            .status(400)
            .body(Body::from("\"Invalid CSRF token\""))
            .unwrap();
    }

    if auth_code.len() != 6 || auth_code.chars().any(|c| !c.is_ascii_digit()) {
        return Response::builder()
            .status(400)
            .body(Body::from("\"Invalid auth code\""))
            .unwrap();
    }

    let user_sid = jar.get("user-sid").map(|cookie| cookie.value().to_string());
    let Some(user_sid) = user_sid else {
        return Response::builder()
            .status(400)
            .body(Body::from("\"Invalid user session\""))
            .unwrap();
    };

    let ua = get_ua(&headers);

    let svc = state.get_container().auth_with_code_user_page();
    let Ok(result) = svc
        .execute(AuthWithCodeUserPageServiceInput {
            auth_code,
            user_sid,
            user_agent: ua.to_string(),
        })
        .await
    else {
        return Response::builder()
            .status(400)
            .body(Body::from("\"Failed to validate authed token\""))
            .unwrap();
    };

    let edge_token_cookie = Cookie::build(("edge-token", &result.token))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::seconds(60 * 60 * 24 * 365))
        .build();

    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .header("Set-Cookie", edge_token_cookie.to_string())
        .body(Body::from(
            json!({
                "token": result.token,
            })
            .to_string(),
        ))
        .unwrap()
}

fn reset_user_sid_cookie() -> Cookie<'static> {
    Cookie::build(("user-sid", ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::ZERO)
        .build()
}
fn reset_userreg_state_id_cookie() -> Cookie<'static> {
    Cookie::build(("userreg-state-id", ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::ZERO)
        .build()
}

fn reset_user_login_state_id_cookie() -> Cookie<'static> {
    Cookie::build(("user-login-state-id", ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(time::Duration::ZERO)
        .build()
}
