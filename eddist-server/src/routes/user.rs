use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
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
        let reset_user_sid = Cookie::build(("user-sid", ""))
            .path("/")
            .http_only(true)
            .secure(true)
            .max_age(time::Duration::ZERO)
            .build();
        return Response::builder()
            .status(302)
            .header(
                "Set-Cookie",
                HeaderValue::from_str(&reset_user_sid.to_string()).unwrap(),
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
    let user_cookie = jar.get("user-sid").map(|cookie| cookie.value().to_string());

    let svc = state.get_container().user_reg_temp_url();
    let Ok(output) = svc
        .execute(UserRegTempUrlServiceInput {
            temp_url_path,
            user_cookie,
        })
        .await
    else {
        return Response::builder()
            .status(200)
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

            let mut res_builder = Response::builder();
            {
                let headers = res_builder.headers_mut().unwrap();

                let user_state_id_cookie = Cookie::build(("userreg-state-id", state_cookie))
                    .path("/")
                    .http_only(true)
                    .secure(true)
                    .max_age(time::Duration::seconds(60 * 3))
                    .build();
                let user_sid_cookie = Cookie::build(("user-sid", ""))
                    .path("/")
                    .http_only(true)
                    .secure(true)
                    .max_age(time::Duration::ZERO)
                    .build();

                headers.append(
                    "Set-Cookie",
                    HeaderValue::from_str(&user_sid_cookie.to_string()).unwrap(),
                );
                headers.append(
                    "Set-Cookie",
                    HeaderValue::from_str(&user_state_id_cookie.to_string()).unwrap(),
                );

                headers.append("Cache-Control", HeaderValue::from_static("no-store"));
            }

            res_builder.status(200).body(Body::from(html)).unwrap()
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
            let mut builder = Response::builder();
            let headers = builder.headers_mut().unwrap();

            // NOTE: このへんのCookie処理いい感じにうまくしたい
            headers.append(
                "Set-Cookie",
                "userreg-state-id=; Path=/; HttpOnly; Secure"
                    .parse()
                    .unwrap(),
            );

            return builder
                .status(400)
                .body(Body::from("User registration state session is not found"))
                .unwrap();
        }
        Err(_) => {
            return Response::builder()
                .status(400)
                .body(Body::from("Failed to redirect to IDP"))
                .unwrap()
        }
    };

    Response::builder()
        .status(302)
        .header("Cache-Control", "no-store")
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

            let mut res_builder = Response::builder();
            {
                let headers = res_builder.headers_mut().unwrap();
                headers.append(
                    "Set-Cookie",
                    "user-sid=; Path=/; HttpOnly; Secure; Max-Age=0"
                        .parse()
                        .unwrap(),
                );
            }

            res_builder.status(200).body(Body::from(html)).unwrap()
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
                .unwrap()
        }
    };

    Response::builder()
        .status(302)
        .header("Cache-Control", "no-store")
        .header(
            "Set-Cookie",
            format!(
                "user-login-state-id={}; Path=/; HttpOnly; Secure; Max-Age=900",
                output.user_login_state_id
            ),
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

    let mut builder = Response::builder();
    let headers = builder.headers_mut().unwrap();
    headers.append(
        "Set-Cookie",
        "user-sid=; Path=/; HttpOnly; Secure; Max-Age=0"
            .parse()
            .unwrap(),
    );

    builder
        .status(302)
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
                    .header(
                        "Set-Cookie",
                        "userreg-state-id=; Path=/; HttpOnly; Secure; Max-Age=0",
                    )
                    .header(
                        "Set-Cookie",
                        "user-login-state-id=; Path=/; HttpOnly; Secure; Max-Age=0",
                    )
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
                .header(
                    "Set-Cookie",
                    "userlogin-state-id=; Path=/; HttpOnly; Secure; Max-Age=0",
                )
                .status(400)
                .body(Body::from("Failed to get user (maybe unregistered)"))
                .unwrap();
        }
        Err(e) => {
            log::error!("Failed to get user: {e}");
            return Response::builder()
                .header(
                    "Set-Cookie",
                    "userlogin-state-id=; Path=/; HttpOnly; Secure; Max-Age=0",
                )
                .header(
                    "Set-Cookie",
                    "userreg-state-id=; Path=/; HttpOnly; Secure; Max-Age=0",
                )
                .status(400)
                .body(Body::from(format!("Failed to get user")))
                .unwrap();
        }
    };

    let mut builder = Response::builder();
    let headers = builder.headers_mut().unwrap();
    headers.append(
        "Set-Cookie",
        format!("user-sid={user_sid}; Path=/; HttpOnly; Secure; Max-Age=31536000")
            .parse()
            .unwrap(),
    );
    headers.append(
        "Set-Cookie",
        "userreg-state-id=; Path=/; HttpOnly; Secure; Max-Age=0"
            .parse()
            .unwrap(),
    );

    builder
        .status(302)
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

    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .header(
            "Set-Cookie",
            format!(
                "edge-token={}; Path=/; HttpOnly; Secure; Max-Age=31536000",
                result.token
            ),
        )
        .body(Body::from(
            json!({
                "token": result.token,
            })
            .to_string(),
        ))
        .unwrap()
}
