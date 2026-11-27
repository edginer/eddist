use std::time::Duration;

use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Request, State, rejection::JsonRejection},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::post,
};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use http::{HeaderMap, HeaderValue, StatusCode, header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    app::{AppState, HTTP_REQUEST_TIMEOUT},
    repositories::app_credentials_repository::RegistrationClient,
    services::{
        app_attestation_service::{
            AttestationError, ChallengeSubject, Platform, RefreshEvidence, RegistrationEvidence,
            RegistrationProof,
        },
        server_settings_cache::ServerSettingKey,
    },
    utils::{get_asn_num, get_origin_ip, get_ua},
};

pub(crate) fn routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/api/app/attestation/challenges", post(challenge))
        .route("/api/app/attestation/register", post(register))
        .route("/api/app/attestation/refresh", post(refresh))
        .route_layer(DefaultBodyLimit::max(32 * 1024))
        .route_layer(from_fn_with_state(state.clone(), check_attestation_access))
        .route_layer(from_fn_with_state(
            HTTP_REQUEST_TIMEOUT.saturating_sub(Duration::from_secs(1)),
            attestation_timeout,
        ))
}

async fn attestation_timeout(
    State(timeout): State<Duration>,
    request: Request,
    next: Next,
) -> Result<Response, AttestationError> {
    tokio::time::timeout(timeout, next.run(request))
        .await
        .map_err(|_| AttestationError::TimedOut)
}

#[derive(Clone)]
struct AttestationIp(String);

async fn check_attestation_access(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AttestationError> {
    let service = state
        .active_app_attestations()
        .first()
        .ok_or(AttestationError::Disabled)?;
    let origin_ip = get_origin_ip(request.headers())
        .ok_or(AttestationError::MissingClientIp)?
        .to_owned();
    service.check_rate_limit(&origin_ip).await?;
    request.extensions_mut().insert(AttestationIp(origin_ip));
    Ok(next.run(request).await)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Purpose {
    Register,
    Refresh,
}

#[derive(Deserialize)]
struct ChallengeRequest {
    platform: Platform,
    purpose: Purpose,
    registration_key: Option<String>,
}

async fn challenge(
    State(state): State<AppState>,
    headers: HeaderMap,
    input: Result<Json<ChallengeRequest>, JsonRejection>,
) -> Result<Response, AttestationError> {
    let input = parse_json(input)?;
    let service = state
        .active_app_attestation(input.platform)
        .ok_or(AttestationError::Disabled)?;
    let subject = match (input.purpose, input.registration_key) {
        (Purpose::Register, Some(key)) => {
            if state
                .ctx
                .server_settings
                .snapshot()
                .is_enabled(ServerSettingKey::CloseNewAuthentication)
            {
                return Err(AttestationError::RegistrationClosed);
            }
            let key = decode_registration_key(&key)?;
            ChallengeSubject::Registration(Sha256::digest(key).into())
        }
        (Purpose::Refresh, None) => {
            let credential = service.find_credential(&bearer(&headers)?).await?;
            ChallengeSubject::Refresh(credential.authed_token_id)
        }
        _ => return Err(AttestationError::InvalidRequest),
    };
    Ok(json_response(
        StatusCode::OK,
        service.issue_challenge(&subject).await?,
    ))
}

#[derive(Deserialize)]
struct RegisterRequest {
    challenge_id: Uuid,
    registration_key: String,
    #[serde(flatten)]
    evidence: RegisterPayload,
}

#[derive(Deserialize)]
#[serde(tag = "platform", rename_all = "snake_case")]
enum RegisterPayload {
    Ios { key_id: String, attestation: String },
    Android { integrity_token: String },
}

async fn register(
    State(state): State<AppState>,
    Extension(AttestationIp(origin_ip)): Extension<AttestationIp>,
    headers: HeaderMap,
    request: Result<Json<RegisterRequest>, JsonRejection>,
) -> Result<Response, AttestationError> {
    let request = parse_json(request)?;
    let (platform, evidence) = match request.evidence {
        RegisterPayload::Ios {
            key_id,
            attestation,
        } => {
            let key_id = STANDARD
                .decode(key_id)
                .ok()
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or(AttestationError::InvalidRequest)?;
            let attestation = STANDARD
                .decode(attestation)
                .map_err(|_| AttestationError::InvalidRequest)?;
            (
                Platform::Ios,
                RegistrationEvidence::Ios {
                    key_id,
                    attestation,
                },
            )
        }
        RegisterPayload::Android { integrity_token } => (
            Platform::Android,
            RegistrationEvidence::Android(validate_integrity_token(integrity_token)?),
        ),
    };
    let service = state
        .active_app_attestation(platform)
        .ok_or(AttestationError::Disabled)?;
    let settings = state.ctx.server_settings.snapshot();
    let client = RegistrationClient {
        ip: &origin_ip,
        user_agent: get_ua(&headers).unwrap_or("unknown"),
        asn: get_asn_num(&headers, &state.ctx.config.asn_header_name).unwrap_or_default() as i32,
        require_user_registration: settings.is_enabled(ServerSettingKey::RequireIdpLinking),
    };
    let result = service
        .register(
            RegistrationProof {
                challenge_id: request.challenge_id,
                registration_key: decode_registration_key(&request.registration_key)?,
                evidence,
            },
            client,
            settings.is_enabled(ServerSettingKey::CloseNewAuthentication),
        )
        .await?;
    Ok(json_response(StatusCode::OK, result))
}

#[derive(Deserialize)]
struct RefreshRequest {
    challenge_id: Uuid,
    #[serde(flatten)]
    evidence: RefreshPayload,
}

#[derive(Deserialize)]
#[serde(tag = "platform", rename_all = "snake_case")]
enum RefreshPayload {
    Ios { assertion: String },
    Android { integrity_token: String },
}

async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<RefreshRequest>, JsonRejection>,
) -> Result<Response, AttestationError> {
    let request = parse_json(request)?;
    let (platform, evidence) = match request.evidence {
        RefreshPayload::Ios { assertion } => {
            let assertion = STANDARD
                .decode(assertion)
                .map_err(|_| AttestationError::InvalidRequest)?;
            (Platform::Ios, RefreshEvidence::Ios(assertion))
        }
        RefreshPayload::Android { integrity_token } => (
            Platform::Android,
            RefreshEvidence::Android(validate_integrity_token(integrity_token)?),
        ),
    };
    let service = state
        .active_app_attestation(platform)
        .ok_or(AttestationError::Disabled)?;
    let credential = service.find_credential(&bearer(&headers)?).await?;
    let deadline = service
        .refresh(credential, request.challenge_id, evidence)
        .await?;
    Ok(json_response(
        StatusCode::OK,
        serde_json::json!({"reattest_after": deadline}),
    ))
}

fn parse_json<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, AttestationError> {
    input.map(|Json(value)| value).map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            AttestationError::RequestTooLarge
        } else {
            AttestationError::InvalidRequest
        }
    })
}

fn validate_integrity_token(token: String) -> Result<String, AttestationError> {
    if token.is_empty() || !token.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(AttestationError::InvalidRequest);
    }
    Ok(token)
}

fn decode_registration_key(encoded: &str) -> Result<[u8; 32], AttestationError> {
    decode_32_bytes(encoded).ok_or(AttestationError::InvalidRequest)
}

fn decode_32_bytes(encoded: &str) -> Option<[u8; 32]> {
    if encoded.len() != 43 {
        return None;
    }
    URL_SAFE_NO_PAD.decode(encoded).ok()?.try_into().ok()
}

fn bearer(headers: &HeaderMap) -> Result<[u8; 32], AttestationError> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let (scheme, token) = values
        .next()
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .ok_or(AttestationError::InvalidToken)?;
    if !scheme.eq_ignore_ascii_case("bearer") || values.next().is_some() {
        return Err(AttestationError::InvalidToken);
    }
    decode_32_bytes(token).ok_or(AttestationError::InvalidToken)
}

fn json_response(status: StatusCode, data: impl Serialize) -> Response {
    (status, [(header::CACHE_CONTROL, "no-store")], Json(data)).into_response()
}

impl IntoResponse for AttestationError {
    fn into_response(self) -> Response {
        use AttestationError::*;
        let (status, code) = match self {
            Disabled => (StatusCode::FORBIDDEN, "attestation_disabled"),
            RegistrationClosed => (StatusCode::SERVICE_UNAVAILABLE, "registration_closed"),
            InvalidRequest => (StatusCode::BAD_REQUEST, "invalid_request"),
            RequestTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "request_too_large"),
            InvalidToken => (StatusCode::UNAUTHORIZED, "invalid_token"),
            MissingClientIp => (StatusCode::FORBIDDEN, "missing_client_ip"),
            InvalidChallenge => (StatusCode::CONFLICT, "invalid_challenge"),
            RegistrationConflict => (StatusCode::CONFLICT, "registration_conflict"),
            TokenRevoked => (StatusCode::FORBIDDEN, "token_revoked"),
            Rejected => (StatusCode::FORBIDDEN, "attestation_rejected"),
            ProviderUnavailable | TimedOut => {
                (StatusCode::SERVICE_UNAVAILABLE, "attestation_unavailable")
            }
            RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            RegistrationQuotaExceeded => {
                (StatusCode::TOO_MANY_REQUESTS, "registration_quota_exceeded")
            }
            Redis(_) | Database(_) => {
                tracing::error!(error = %self, "App attestation unavailable");
                (StatusCode::SERVICE_UNAVAILABLE, "attestation_unavailable")
            }
        };
        let mut response = json_response(status, serde_json::json!({"error": code}));
        if status == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use serde_json::json;

    #[test]
    fn deserializes_existing_ios_and_new_android_request_shapes() {
        let challenge_id = Uuid::new_v4();
        let ios: RegisterRequest = serde_json::from_value(json!({
            "platform": "ios", "challenge_id": challenge_id, "registration_key": "registration-key",
            "key_id": "ios-key", "attestation": "ios-proof",
        }))
        .unwrap();
        assert_eq!(ios.challenge_id, challenge_id);
        assert!(
            matches!(ios.evidence, RegisterPayload::Ios { key_id, attestation } if key_id == "ios-key" && attestation == "ios-proof")
        );

        let android: RegisterRequest = serde_json::from_value(json!({
            "platform": "android", "challenge_id": challenge_id, "registration_key": "registration-key",
            "integrity_token": "google-proof",
        })).unwrap();
        assert!(
            matches!(android.evidence, RegisterPayload::Android { integrity_token } if integrity_token == "google-proof")
        );

        let android: RefreshRequest = serde_json::from_value(json!({
            "platform": "android", "challenge_id": challenge_id, "integrity_token": "google-proof",
        }))
        .unwrap();
        assert_eq!(android.challenge_id, challenge_id);
        assert!(
            matches!(android.evidence, RefreshPayload::Android { integrity_token } if integrity_token == "google-proof")
        );
    }

    #[tokio::test]
    async fn attestation_timeout_returns_an_uncacheable_json_error() {
        let router = Router::new()
            .route("/", post(std::future::pending::<StatusCode>))
            .route_layer(from_fn_with_state(
                Duration::from_millis(1),
                attestation_timeout,
            ));
        let response = TestServer::new(router).post("/").await;
        assert_eq!(response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.header(header::CACHE_CONTROL), "no-store");
        assert_eq!(
            response.json::<serde_json::Value>(),
            json!({"error": "attestation_unavailable"})
        );
    }

    #[tokio::test]
    async fn json_rejections_have_the_attestation_error_format() {
        let router = Router::new()
            .route(
                "/",
                post(
                    |input: Result<Json<ChallengeRequest>, JsonRejection>| async {
                        parse_json(input).map(|_| StatusCode::OK)
                    },
                ),
            )
            .layer(DefaultBodyLimit::max(64));
        let server = TestServer::new(router);
        for (content_type, body, status, code) in [
            (
                "text/plain",
                "{}",
                StatusCode::BAD_REQUEST,
                "invalid_request",
            ),
            (
                "application/json",
                "{",
                StatusCode::BAD_REQUEST,
                "invalid_request",
            ),
            (
                "application/json",
                "{}",
                StatusCode::BAD_REQUEST,
                "invalid_request",
            ),
            (
                "application/json",
                &" ".repeat(65),
                StatusCode::PAYLOAD_TOO_LARGE,
                "request_too_large",
            ),
        ] {
            let response = server
                .post("/")
                .content_type(content_type)
                .bytes(body.to_owned().into())
                .await;
            assert_eq!(response.status_code(), status);
            assert_eq!(response.header(header::CACHE_CONTROL), "no-store");
            assert_eq!(response.json::<serde_json::Value>(), json!({"error": code}));
        }
    }
}
