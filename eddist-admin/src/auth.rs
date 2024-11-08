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
    reqwest::async_http_client, AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RefreshToken, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use tracing::info_span;

use crate::{
    repository::{
        admin_archive_repository::AdminArchiveRepositoryImpl,
        admin_bbs_repository::AdminBbsRepositoryImpl,
    },
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

pub async fn verify_access_token(access_token: &str) -> Result<KeycloakAccessToken, ErrorKind> {
    if let Ok(userinfo_url) = env::var("EDDIST_USER_INFO_URL") {
        let pub_key = std::env::var("EDDIST_ADMIN_JWT_PUB_KEY").unwrap();
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_audience(&[env::var("EDDIST_AUDIENCE").unwrap().as_str()]);
        let token = jsonwebtoken::decode::<Auth0AccessToken>(
            access_token,
            &jsonwebtoken::DecodingKey::from_rsa_pem(pub_key.as_bytes()).unwrap(),
            &validation,
        );

        match token {
            Ok(t) => {
                let res = reqwest::Client::new()
                    .get(&userinfo_url)
                    .bearer_auth(access_token)
                    .send()
                    .await
                    .unwrap();
                let text = res.text().await.unwrap();
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
        let pub_key = std::env::var("EDDIST_ADMIN_JWT_PUB_KEY").unwrap();
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_audience(&["account"]);
        let token = jsonwebtoken::decode::<KeycloakAccessToken>(
            access_token,
            &jsonwebtoken::DecodingKey::from_rsa_pem(pub_key.as_bytes()).unwrap(),
            &validation,
        );

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
    }): State<AppState<AdminBbsRepositoryImpl, AdminArchiveRepositoryImpl>>,
    admin_session: AdminSession,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    if admin_session.next_refresh_at > Utc::now() && admin_session.userinfo.is_some() {
        log::info!("no need to retrieve userinfo");
        req.extensions_mut().insert(admin_session.userinfo.unwrap());
        return next.run(req).await;
    } else if let Some(access_token) = &admin_session.access_token {
        let access_token = verify_access_token(access_token).await;
        let access_token = match access_token {
            Ok(token) => {
                let new_session = AdminSession {
                    next_refresh_at: Utc::now()
                        .checked_add_signed(TimeDelta::minutes(5))
                        .unwrap(),
                    userinfo: Some(token.clone()),
                    ..admin_session
                };
                session.insert("data", new_session).await.unwrap();
                token
            }
            Err(ErrorKind::ExpiredSignature) => {
                let Some(refresh_token) = &admin_session.refresh_token else {
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
                    .request_async(oauth2::reqwest::async_http_client)
                    .await
                else {
                    session.delete().await.unwrap();

                    return Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Body::empty())
                        .unwrap();
                };
                let access_token = token.access_token().secret();

                match verify_access_token(access_token).await {
                    Ok(userinfo) => {
                        let new_session = AdminSession {
                            access_token: Some(access_token.to_string()),
                            refresh_token: token.refresh_token().map(|t| t.secret().to_string()),
                            next_refresh_at: Utc::now()
                                .checked_add_signed(TimeDelta::minutes(5))
                                .unwrap(),
                            userinfo: Some(userinfo.clone()),
                            ..admin_session
                        };
                        session.insert("data", new_session).await.unwrap();
                        info_span!("success to verify access token from refresh token", token = ?userinfo);
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
            Err(_) => panic!(),
        };

        req.extensions_mut().insert(access_token);

        // TODO: this only executes after getting access token or refresh token
        // sqlx::query_as::<_, (String, String, String, String, String)>(
        //     r#"
        //     SELECT
        //         au.id AS user_id,
        //         au.user_role_id AS user_role_id,
        //         ar.role_name AS role_name,
        //         ars.id AS scope_id,
        //         ars.scope_key AS scope_key
        //     FROM admin_users AS au
        //     JOIN admin_roles AS ar ON au.user_role_id = ar.id
        //     JOIN admin_role_scopes AS ars ON ar.id = ars.role_id
        //     WHERE au.id = UUID_TO_BIN(?)
        //     "#,
        // )
        // .fetch_all(&pool)
        // .await
        // .unwrap();

        return next.run(req).await;
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
            id: *uuid::Uuid::new_v4().as_bytes(),
            created_at: Utc::now(),
            logged_ip,
            logged_ua,
            user_id: None,
            access_token: None,
            refresh_token: None,
            next_refresh_at: Utc::now()
                .checked_add_signed(TimeDelta::minutes(1))
                .unwrap(),
            userinfo: None,
        }
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
    }): State<AppState<AdminBbsRepositoryImpl, AdminArchiveRepositoryImpl>>,
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

    session
        .insert(
            "oauth",
            OAuthSession {
                csrf_state,
                pkce_verifier: pkce_verifier.secret().to_string(),
            },
        )
        .await
        .unwrap();

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
    }): State<AppState<AdminBbsRepositoryImpl, AdminArchiveRepositoryImpl>>,
    session: Session,
    admin_session: AdminSession,
    query: axum::extract::Query<LoginCallbackQuery>,
) -> impl IntoResponse {
    let Some(oauth_session) = session.remove::<OAuthSession>("oauth").await.unwrap() else {
        info_span!("oauth_session is not found");

        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .unwrap();
    };

    let code = AuthorizationCode::new(query.code.clone());
    let csrf_state = CsrfToken::new(query.state.clone());

    if csrf_state.secret() != oauth_session.csrf_state.secret() {
        // HACK: this is for testing, should be removed in production (csrf_state)
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
        .request_async(async_http_client)
        .await;

    match token {
        Ok(token) => {
            let new_session = AdminSession {
                access_token: Some(token.access_token().secret().to_string()),
                refresh_token: token.refresh_token().map(|t| t.secret().to_string()),
                ..admin_session
            };

            // HACK: this is for testing, should be removed in production (access_token and refresh_token)
            info_span!("success to get token from auth server",
                refresh_token = ?new_session.refresh_token,
                access_token = ?new_session.access_token
            );

            session.insert("data", new_session).await.unwrap();

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

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for AdminSession
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(req, state).await?;

        if let Some(session) = session.get::<AdminSession>("data").await.unwrap() {
            Ok(session)
        } else {
            let data = AdminSession::new(
                req.headers
                    .get("CF-Connecting-IP")
                    .unwrap_or(&HeaderValue::from_str("localhost").unwrap()) // for testing
                    .to_str()
                    .unwrap()
                    .to_string(),
                req.headers
                    .get("User-Agent")
                    .unwrap_or(&HeaderValue::from_str("Mozilla/5.0").unwrap()) // for testing
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
            session.insert("data", data.clone()).await.unwrap();

            Ok(data)
        }
    }
}

// GET: /auth/check
pub async fn get_check_auth(admin_session: AdminSession) -> impl IntoResponse {
    let result = if admin_session.access_token.is_some() {
        serde_json::json!(true)
    } else {
        serde_json::json!(false)
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&result).unwrap()))
        .unwrap()
}

// GET: /auth/logout
pub async fn get_logout(session: Session) -> impl IntoResponse {
    session.remove::<AdminSession>("data").await.unwrap();
    Redirect::to("/login").into_response()
}
