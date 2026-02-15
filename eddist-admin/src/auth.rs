use std::env;

use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{self, request::Parts, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use chrono::{TimeDelta, Utc};
use jsonwebtoken::errors::ErrorKind;
use oauth2::{
    reqwest, AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RefreshToken,
    Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use tracing::info_span;

use crate::{
    models::auth::{NativeSessionRequest, NativeSessionResponse, NativeUserInfo},
    AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeycloakAccessToken {
    pub exp: i64,
    pub sub: String,
    pub email_verified: bool,
    pub preferred_username: String,
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Auth0UserInfo {
    pub sub: String,
    pub email_verified: bool,
    pub nickname: String,
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Auth0AccessToken {
    pub exp: i64,
}

static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap()
    })
}

pub async fn verify_access_token(
    access_token: &str,
    is_native: bool,
) -> Result<KeycloakAccessToken, ErrorKind> {
    if let Ok(userinfo_url) = env::var("EDDIST_USER_INFO_URL") {
        let pub_key = if is_native {
            std::env::var("EDDIST_ADMIN_NATIVE_JWT_PUB_KEY").map_err(|_| ErrorKind::InvalidToken)?
        } else {
            std::env::var("EDDIST_ADMIN_JWT_PUB_KEY").map_err(|_| ErrorKind::InvalidToken)?
        };
        let audience = env::var("EDDIST_AUDIENCE").map_err(|_| ErrorKind::InvalidToken)?;
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_audience(&[audience.as_str()]);
        let decoding_key = jsonwebtoken::DecodingKey::from_rsa_pem(pub_key.as_bytes())
            .map_err(|e| e.kind().clone())?;
        let token =
            jsonwebtoken::decode::<Auth0AccessToken>(access_token, &decoding_key, &validation);

        match token {
            Ok(t) => {
                let res = reqwest::Client::new()
                    .get(&userinfo_url)
                    .bearer_auth(access_token)
                    .send()
                    .await
                    .map_err(|e| {
                        log::error!("failed to fetch userinfo: {e:?}");
                        ErrorKind::InvalidToken
                    })?;
                let text = res.text().await.map_err(|e| {
                    log::error!("failed to read userinfo response: {e:?}");
                    ErrorKind::InvalidToken
                })?;
                let res = serde_json::from_str::<Auth0UserInfo>(&text);

                let info = match res {
                    Err(e) => {
                        log::error!("failed to get userinfo: {e:?}");
                        return Err(ErrorKind::ExpiredSignature);
                    }
                    Ok(info) => info,
                };

                Ok(KeycloakAccessToken {
                    exp: t.claims.exp,
                    sub: info.sub,
                    email_verified: info.email_verified,
                    preferred_username: info.nickname,
                    email: info.email,
                })
            }
            Err(e) => {
                log::error!("failed to verify access token: {e:?}");
                Err(e.kind().clone())
            }
        }
    } else {
        let pub_key =
            std::env::var("EDDIST_ADMIN_JWT_PUB_KEY").map_err(|_| ErrorKind::InvalidToken)?;
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_audience(&["account"]);
        let decoding_key = jsonwebtoken::DecodingKey::from_rsa_pem(pub_key.as_bytes())
            .map_err(|e| e.kind().clone())?;
        let token =
            jsonwebtoken::decode::<KeycloakAccessToken>(access_token, &decoding_key, &validation);

        match token {
            Ok(t) => Ok(t.claims),
            Err(e) => {
                log::error!("failed to verify access token: {e:?}");
                Err(e.kind().clone())
            }
        }
    }
}

pub async fn auth_simple_header(
    session: Session,
    State(AppState {
        oauth2_client: client,
        ..
    }): State<AppState>,
    admin_session: AdminSession,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    if let Some(userinfo) = admin_session.userinfo {
        if admin_session.next_refresh_at > Utc::now() {
            log::info!("no need to retrieve userinfo");
            req.extensions_mut().insert(userinfo);
            return next.run(req).await;
        }
    }
    if let Some(access_token) = &admin_session.access_token {
        let is_native = req
            .headers()
            .get("User-Agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .contains("eddist-manager");

        let access_token = verify_access_token(access_token, is_native).await;
        let access_token = match access_token {
            Ok(token) => {
                let new_session = AdminSession {
                    next_refresh_at: Utc::now() + TimeDelta::minutes(5),
                    userinfo: Some(token.clone()),
                    ..admin_session
                };
                if let Err(e) = session.insert("data", new_session).await {
                    log::error!("failed to insert session: {e:?}");
                    return Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap();
                }
                token
            }
            Err(ErrorKind::ExpiredSignature) => {
                let Some(refresh_token) = &admin_session.refresh_token else {
                    let _ = session.delete().await;
                    return Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Body::empty())
                        .unwrap();
                };

                let Ok(token) = client
                    .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
                    .add_scopes(
                        ["openid", "profile", "email", "offline_access"]
                            .iter()
                            .map(|s| Scope::new(s.to_string())),
                    )
                    .request_async(get_http_client())
                    .await
                else {
                    let _ = session.delete().await;

                    return Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Body::empty())
                        .unwrap();
                };
                let access_token = token.access_token().secret();

                match verify_access_token(access_token, is_native).await {
                    Ok(userinfo) => {
                        let new_session = AdminSession {
                            access_token: Some(access_token.to_string()),
                            refresh_token: token.refresh_token().map(|t| t.secret().to_string()),
                            next_refresh_at: Utc::now() + TimeDelta::minutes(5),
                            userinfo: Some(userinfo.clone()),
                            ..admin_session
                        };
                        if let Err(e) = session.insert("data", new_session).await {
                            log::error!("failed to insert session: {e:?}");
                            return Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::empty())
                                .unwrap();
                        }
                        log::info!("success to verify access token from refresh token");
                        userinfo
                    }
                    Err(e) => {
                        info_span!("failed to verify access token", error = ?e);
                        return Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .body(Body::empty())
                            .unwrap();
                    }
                }
            }
            Err(e) => {
                log::error!("unexpected token verification error: {e:?}");
                return Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Body::empty())
                    .unwrap();
            }
        };

        req.extensions_mut().insert(access_token);

        let mut res = next.run(req).await;
        res.headers_mut().insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        return res;
    }

    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(Body::empty())
        .unwrap()
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AdminSession {
    id: [u8; 16],
    created_at: chrono::DateTime<Utc>,
    logged_ip: String,
    logged_ua: String,
    user_id: Option<[u8; 16]>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    next_refresh_at: chrono::DateTime<Utc>,
    userinfo: Option<KeycloakAccessToken>,
}

impl AdminSession {
    fn new(logged_ip: String, logged_ua: String) -> Self {
        Self {
            id: *uuid::Uuid::now_v7().as_bytes(),
            created_at: Utc::now(),
            logged_ip,
            logged_ua,
            user_id: None,
            access_token: None,
            refresh_token: None,
            next_refresh_at: Utc::now() + TimeDelta::minutes(1),
            userinfo: None,
        }
    }

    pub fn get_admin_email(&self) -> Option<String> {
        self.userinfo.as_ref().map(|info| info.email.clone())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct OAuthSession {
    csrf_state: CsrfToken,
    pkce_verifier: String,
}

// GET: /login
pub async fn get_login(
    State(AppState {
        oauth2_client: oauth_client,
        ..
    }): State<AppState>,
    session: Session,
    _: AdminSession,
) -> impl IntoResponse {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let authz_req = oauth_client.authorize_url(CsrfToken::new_random);
    let authz_req = if let Ok(audience) = env::var("EDDIST_AUDIENCE") {
        authz_req.add_extra_param("audience", audience)
    } else {
        authz_req
    };

    let (authorize_url, csrf_state) = authz_req
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("offline_access".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let _ = session
        .insert(
            "oauth",
            OAuthSession {
                csrf_state,
                pkce_verifier: pkce_verifier.secret().to_string(),
            },
        )
        .await;

    Redirect::to(authorize_url.as_str())
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginCallbackQuery {
    code: String,
    state: String,
}

// GET: /auth/callback
pub async fn get_login_callback(
    State(AppState {
        oauth2_client: oauth_client,
        ..
    }): State<AppState>,
    session: Session,
    admin_session: AdminSession,
    query: axum::extract::Query<LoginCallbackQuery>,
) -> impl IntoResponse {
    let oauth_session = session.remove::<OAuthSession>("oauth").await;
    let Some(oauth_session) = oauth_session.ok().flatten() else {
        info_span!("oauth_session is not found");

        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .unwrap();
    };

    let code = AuthorizationCode::new(query.code.clone());
    let csrf_state = CsrfToken::new(query.state.clone());

    if csrf_state.secret() != oauth_session.csrf_state.secret() {
        info_span!("csrf_state is not matched",
          server_state = ?oauth_session.csrf_state.secret(),
          client_state = ?csrf_state.secret()
        );

        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .unwrap();
    }

    let token = oauth_client
        .exchange_code(code)
        .set_pkce_verifier(PkceCodeVerifier::new(oauth_session.pkce_verifier))
        .request_async(get_http_client())
        .await;

    match token {
        Ok(token) => {
            let new_session = AdminSession {
                access_token: Some(token.access_token().secret().to_string()),
                refresh_token: token.refresh_token().map(|t| t.secret().to_string()),
                ..admin_session
            };

            log::info!("success to get token from auth server");

            let _ = session.insert("data", new_session).await;

            Redirect::to("/dashboard").into_response()
        }
        Err(e) => {
            info_span!("failed to get token from auth server", error = ?e);

            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::empty())
                .unwrap()
        }
    }
}

impl<S> FromRequestParts<S> for AdminSession
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(req, state).await?;

        if let Some(session) = session.get::<AdminSession>("data").await.unwrap_or(None) {
            Ok(session)
        } else {
            let ip = req
                .headers
                .get("CF-Connecting-IP")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("localhost")
                .to_string();
            let ua = req
                .headers
                .get("User-Agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("Mozilla/5.0")
                .to_string();
            let data = AdminSession::new(ip, ua);
            let _ = session.insert("data", data.clone()).await;

            Ok(data)
        }
    }
}

/// Extractor that pulls admin email from the session, returning 401 if not authenticated.
pub struct AdminEmail(pub String);

impl<S> FromRequestParts<S> for AdminEmail
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = AdminSession::from_request_parts(req, state).await?;
        session
            .get_admin_email()
            .map(AdminEmail)
            .ok_or((StatusCode::UNAUTHORIZED, "No user information available"))
    }
}

// GET: /auth/check
pub async fn get_check_auth(admin_session: AdminSession) -> impl IntoResponse {
    let result = admin_session.access_token.is_some();
    axum::Json(result)
}

// GET: /auth/logout
pub async fn get_logout(session: Session) -> impl IntoResponse {
    let _ = session.remove::<AdminSession>("data").await;
    Redirect::to("/login").into_response()
}

/// Exchange OAuth2 access token for session token
#[utoipa::path(
    post,
    path = "/auth/native/session",
    request_body = NativeSessionRequest,
    responses(
        (status = 200, description = "Session token created successfully", body = NativeSessionResponse),
        (status = 401, description = "Invalid access token")
    ),
    tag = "auth"
)]
pub async fn post_native_session(
    State(_state): State<AppState>,
    headers: http::HeaderMap,
    session: Session,
    axum::Json(req): axum::Json<NativeSessionRequest>,
) -> impl IntoResponse {
    // Validate the access token using existing function
    let user_info = match verify_access_token(&req.access_token, true).await {
        Ok(token) => token,
        Err(e) => {
            info_span!("failed to verify access token for native session", error = ?e);
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"error": "invalid_access_token"}"#))
                .unwrap();
        }
    };

    let ClientInfo {
        client_ip,
        client_ua,
    } = get_client_info(&headers);

    // Create AdminSession with the validated access token
    let admin_session = AdminSession {
        id: *uuid::Uuid::now_v7().as_bytes(),
        created_at: Utc::now(),
        logged_ip: client_ip,
        logged_ua: client_ua,
        user_id: None, // We don't have user_id mapping yet
        access_token: Some(req.access_token),
        refresh_token: None, // Native clients don't get refresh tokens in this flow
        next_refresh_at: Utc::now() + TimeDelta::minutes(5),
        userinfo: Some(user_info.clone()),
    };

    // Store the session
    let session_id = uuid::Uuid::from_bytes(admin_session.id).to_string();
    if let Err(e) = session.insert("data", admin_session).await {
        log::error!("failed to insert native session: {e:?}");
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"error": "session_error"}"#))
            .unwrap();
    }

    let response = NativeSessionResponse {
        session_token: session_id, // Return session ID instead of JWT token
        expires_at: Utc::now() + chrono::TimeDelta::hours(24),
        user_info: NativeUserInfo {
            sub: user_info.sub,
            email: user_info.email,
            preferred_username: user_info.preferred_username,
            email_verified: user_info.email_verified,
        },
    };

    axum::Json(response).into_response()
}

struct ClientInfo {
    client_ip: String,
    client_ua: String,
}

fn get_client_info(headers: &http::HeaderMap) -> ClientInfo {
    let client_ip = headers
        .get("Cf-Connecting-IP")
        .or_else(|| headers.get("X-Real-IP"))
        .or_else(|| headers.get("X-Forwarded-For"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let client_ua = headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    ClientInfo {
        client_ip,
        client_ua,
    }
}
