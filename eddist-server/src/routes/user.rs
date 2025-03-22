use axum::{
    body::Body,
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;

use crate::{
    services::{
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
}

async fn get_user_page(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let user_sid = jar.get("user-sid").map(|cookie| cookie.value().to_string());
    let Some(user_sid) = user_sid else {
        // TODO: more user-friendly error page
        return Response::builder()
            .status(400)
            .body(Body::from("User is not registered"))
            .unwrap();
    };

    let Ok(UserPageServiceOutput { user }) = state
        .services
        .user_page()
        .execute(UserPageServiceInput { user_sid })
        .await
    else {
        return Response::builder()
            .status(400)
            .body(Body::from("User not found"))
            .unwrap();
    };

    let html = state
        .template_engine
        .render(
            "user-page-simple.get",
            &serde_json::json!({
                "user_name": user.user_name,
            }),
        )
        .unwrap();

    Html(html).into_response()
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
                    &serde_json::json!(
                        {
                            "available_idps": available_idps,
                        }
                    ),
                )
                .unwrap();

            let mut res_builder = Response::builder();
            {
                let headers = res_builder.headers_mut().unwrap();
                headers.append(
                    "Set-Cookie",
                    format!("userreg-state-id={state_cookie}; Path=/; HttpOnly; Secure")
                        .parse()
                        .unwrap(),
                );
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
                    &serde_json::json!(
                        {
                            "available_idps": available_idps,
                        }
                    ),
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

    let UserAuthzIdpCallbackServiceOutput { user_sid } = state
        .services
        .user_authz_idp_callback()
        .execute(UserAuthzIdpCallbackServiceInput {
            code: query.code.clone(),
            state_id: state_cookie.to_string(),
            callback_kind,
        })
        .await
        .unwrap();

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
