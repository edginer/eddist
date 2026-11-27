use std::time::{Duration, Instant};

use anyhow::Context as _;
use base64::{
    Engine,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use chrono::Utc;
use eddist_core::redis_keys::play_integrity_registration_quota_key;
use jiff::{Timestamp, tz::TimeZone};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use redis::aio::ConnectionManager;
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::config::PlayIntegrityConfig;

const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const PLAY_INTEGRITY_SCOPE: &str = "https://www.googleapis.com/auth/playintegrity";
const MAX_AGE_MILLIS: i64 = 5 * 60 * 1000;
const FUTURE_TOLERANCE_MILLIS: i64 = 30 * 1000;

#[derive(Debug)]
pub(crate) enum PlayIntegrityError {
    Rejected,
    Unavailable,
    RegistrationQuotaExceeded,
    QuotaStore(redis::RedisError),
}

pub(crate) struct PlayIntegrityVerifier {
    http: Client,
    package_name: String,
    certificate_digests: Vec<[u8; 32]>,
    decode_url: Url,
    service_account: GoogleServiceAccount,
    access_token: Mutex<Option<CachedAccessToken>>,
    daily_registration_limit: u32,
}

impl PlayIntegrityVerifier {
    pub(crate) fn new(config: &PlayIntegrityConfig) -> anyhow::Result<Self> {
        let http = Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(4))
            .build()
            .context("could not initialize Play Integrity HTTP client")?;
        let decode_url = Url::parse(&format!(
            "https://playintegrity.googleapis.com/v1/{}:decodeIntegrityToken",
            config.package_name
        ))?;
        let service_account = GoogleServiceAccount::from_file(&config.service_account_file)?;
        Ok(Self {
            http,
            package_name: config.package_name.clone(),
            certificate_digests: config.certificate_digests.clone(),
            decode_url,
            service_account,
            access_token: Mutex::new(None),
            daily_registration_limit: config.daily_registration_limit,
        })
    }

    pub(crate) async fn verify(
        &self,
        integrity_token: &str,
        client_data_hash: &[u8; 32],
        registration_budget: Option<&mut ConnectionManager>,
    ) -> Result<(), PlayIntegrityError> {
        let access_token = self.google_access_token().await?;
        if let Some(redis) = registration_budget {
            let reserved = reserve_registration(
                redis,
                &self.service_account.project_id,
                self.daily_registration_limit,
                Timestamp::now(),
            )
            .await
            .map_err(PlayIntegrityError::QuotaStore)?;
            if !reserved {
                return Err(PlayIntegrityError::RegistrationQuotaExceeded);
            }
        }
        let response = self
            .http
            .post(self.decode_url.clone())
            .bearer_auth(&access_token)
            .json(&serde_json::json!({ "integrity_token": integrity_token }))
            .send()
            .await
            .map_err(unavailable)?;
        if response.status() == StatusCode::BAD_REQUEST {
            return Err(PlayIntegrityError::Rejected);
        }
        if matches!(
            response.status(),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ) {
            // The next request obtains new credentials. Standard Integrity proofs have replay protection.
            self.invalidate_access_token(&access_token).await;
        }
        let decoded: DecodeResponse = response
            .error_for_status()
            .map_err(unavailable)?
            .json()
            .await
            .map_err(unavailable)?;
        let expected_hash = URL_SAFE_NO_PAD.encode(client_data_hash);
        decoded.token_payload_external.validate(
            &self.package_name,
            &expected_hash,
            &self.certificate_digests,
            Utc::now().timestamp_millis(),
        )
    }

    async fn invalidate_access_token(&self, rejected: &str) {
        let mut cached = self.access_token.lock().await;
        if cached.as_ref().is_some_and(|token| token.value == rejected) {
            *cached = None;
        }
    }

    async fn google_access_token(&self) -> Result<String, PlayIntegrityError> {
        let mut cached = self.access_token.lock().await;
        if let Some(token) = cached
            .as_ref()
            .filter(|token| Instant::now() < token.refresh_at)
        {
            return Ok(token.value.clone());
        }

        let assertion = self
            .service_account
            .assertion(Utc::now().timestamp())
            .map_err(|_| PlayIntegrityError::Unavailable)?;
        let requested_at = Instant::now();
        let response: AccessTokenResponse = self
            .http
            .post(OAUTH_TOKEN_URL)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
            .map_err(unavailable)?
            .error_for_status()
            .map_err(unavailable)?
            .json()
            .await
            .map_err(unavailable)?;
        if response.access_token.is_empty()
            || response.expires_in == 0
            || !response.token_type.eq_ignore_ascii_case("bearer")
        {
            return Err(PlayIntegrityError::Unavailable);
        }
        let refresh_at =
            requested_at + Duration::from_secs(response.expires_in.min(3600).saturating_sub(30));
        *cached = Some(CachedAccessToken {
            value: response.access_token.clone(),
            refresh_at,
        });
        Ok(response.access_token)
    }
}

fn registration_quota_key(project_id: &str, now: Timestamp) -> String {
    static PACIFIC: TimeZone = jiff::tz::get!("America/Los_Angeles");
    let day = now.to_zoned(PACIFIC.clone()).date();
    play_integrity_registration_quota_key(project_id, &day.to_string())
}

async fn reserve_registration(
    redis: &mut ConnectionManager,
    project_id: &str,
    limit: u32,
    now: Timestamp,
) -> redis::RedisResult<bool> {
    // The date in the key controls the quota reset. The expiry just removes old counters.
    redis::cmd("EVAL")
        .arg(
            "local count = tonumber(redis.call('GET', KEYS[1]) or '0')
             if count >= tonumber(ARGV[1]) then return 0 end
             redis.call('SET', KEYS[1], count + 1, 'EX', 172800)
             return 1",
        )
        .arg(1)
        .arg(registration_quota_key(project_id, now))
        .arg(limit)
        .query_async(redis)
        .await
}

fn unavailable(error: reqwest::Error) -> PlayIntegrityError {
    tracing::warn!(status = ?error.status(), timeout = error.is_timeout(), "Google Play Integrity request failed");
    PlayIntegrityError::Unavailable
}

struct CachedAccessToken {
    value: String,
    refresh_at: Instant,
}

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    expires_in: u64,
    token_type: String,
}

struct GoogleServiceAccount {
    project_id: String,
    email: String,
    signing_key: EncodingKey,
    header: Header,
}

#[derive(Deserialize)]
struct ServiceAccountFile {
    #[serde(rename = "type")]
    kind: String,
    project_id: String,
    client_email: String,
    private_key: String,
    private_key_id: Option<String>,
}

#[derive(Serialize)]
struct ServiceAccountAssertion<'a> {
    iss: &'a str,
    scope: &'static str,
    aud: &'static str,
    iat: i64,
    exp: i64,
}

impl GoogleServiceAccount {
    fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let file =
            std::fs::read(path).context("could not read PLAY_INTEGRITY_SERVICE_ACCOUNT_FILE")?;
        let config: ServiceAccountFile = serde_json::from_slice(&file)
            .map_err(|_| anyhow::anyhow!("invalid Play Integrity service account JSON"))?;
        anyhow::ensure!(
            config.kind == "service_account"
                && !config.client_email.trim().is_empty()
                && !config.project_id.trim().is_empty(),
            "Play Integrity requires a Google service account key file"
        );
        let signing_key =
            EncodingKey::from_rsa_pem(config.private_key.as_bytes()).map_err(|_| {
                anyhow::anyhow!("invalid RSA key in Play Integrity service account file")
            })?;
        let mut header = Header::new(Algorithm::RS256);
        header.kid = config.private_key_id;
        let account = Self {
            project_id: config.project_id.trim().to_owned(),
            email: config.client_email,
            signing_key,
            header,
        };
        account.assertion(Utc::now().timestamp()).map_err(|_| {
            anyhow::anyhow!("could not sign with the Play Integrity service account key")
        })?;
        Ok(account)
    }

    fn assertion(&self, issued_at: i64) -> jsonwebtoken::errors::Result<String> {
        jsonwebtoken::encode(
            &self.header,
            &ServiceAccountAssertion {
                iss: &self.email,
                scope: PLAY_INTEGRITY_SCOPE,
                aud: OAUTH_TOKEN_URL,
                iat: issued_at,
                exp: issued_at + 300,
            },
            &self.signing_key,
        )
    }
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DecodeResponse {
    token_payload_external: PlayIntegrityVerdict,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct PlayIntegrityVerdict {
    request_details: RequestDetails,
    app_integrity: AppIntegrity,
    device_integrity: DeviceIntegrity,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RequestDetails {
    request_package_name: String,
    request_hash: String,
    timestamp_millis: String,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AppIntegrity {
    app_recognition_verdict: String,
    package_name: String,
    certificate_sha256_digest: Vec<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DeviceIntegrity {
    device_recognition_verdict: Vec<String>,
}

impl PlayIntegrityVerdict {
    fn validate(
        self,
        package_name: &str,
        request_hash: &str,
        certificate_digests: &[[u8; 32]],
        now_millis: i64,
    ) -> Result<(), PlayIntegrityError> {
        let request = self.request_details;
        let timestamp = request
            .timestamp_millis
            .parse::<i64>()
            .map_err(|_| PlayIntegrityError::Rejected)?;
        if request.request_package_name != package_name
            || request.request_hash != request_hash
            || !(now_millis - MAX_AGE_MILLIS..=now_millis + FUTURE_TOLERANCE_MILLIS)
                .contains(&timestamp)
        {
            return Err(PlayIntegrityError::Rejected);
        }

        let app = self.app_integrity;
        if !matches!(
            app.app_recognition_verdict.as_str(),
            "PLAY_RECOGNIZED" | "UNRECOGNIZED_VERSION"
        ) || app.package_name != package_name
            || app.certificate_sha256_digest.is_empty()
            || !app.certificate_sha256_digest.iter().all(|digest| {
                URL_SAFE_NO_PAD
                    .decode(digest)
                    .or_else(|_| URL_SAFE.decode(digest))
                    .ok()
                    .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
                    .is_some_and(|digest| certificate_digests.contains(&digest))
            })
        {
            return Err(PlayIntegrityError::Rejected);
        }

        let device = self.device_integrity;
        if !device
            .device_recognition_verdict
            .iter()
            .any(|verdict| verdict == "MEETS_DEVICE_INTEGRITY")
        {
            return Err(PlayIntegrityError::Rejected);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    const PACKAGE: &str = "com.example.eddist";
    const TIME: i64 = 1_800_000_000_000;
    const CERT: [u8; 32] = [7; 32];
    const CHALLENGE_HASH: [u8; 32] = [2; 32];

    fn decode_response() -> Value {
        json!({"tokenPayloadExternal": {
            "requestDetails": {
                "requestPackageName": PACKAGE,
                "requestHash": URL_SAFE_NO_PAD.encode(CHALLENGE_HASH),
                "timestampMillis": TIME.to_string(),
            },
            "appIntegrity": {
                "appRecognitionVerdict": "UNRECOGNIZED_VERSION",
                "packageName": PACKAGE,
                "certificateSha256Digest": [URL_SAFE_NO_PAD.encode(CERT)],
            },
            "accountDetails": { "appLicensingVerdict": "UNLICENSED" },
            "deviceIntegrity": { "deviceRecognitionVerdict": ["MEETS_DEVICE_INTEGRITY"] },
        }})
    }

    fn validate(response: Value) -> Result<(), PlayIntegrityError> {
        let response: DecodeResponse = serde_json::from_value(response).unwrap();
        response.token_payload_external.validate(
            PACKAGE,
            &URL_SAFE_NO_PAD.encode(CHALLENGE_HASH),
            &[CERT],
            TIME,
        )
    }

    #[test]
    fn accepts_directly_distributed_apk_with_expected_signature_and_device() {
        assert!(validate(decode_response()).is_ok());
        let mut response = decode_response();
        response["tokenPayloadExternal"]["appIntegrity"]["certificateSha256Digest"] =
            json!([URL_SAFE.encode(CERT)]);
        assert!(validate(response).is_ok());
    }

    #[test]
    fn rejects_missing_mismatched_and_stale_verdicts() {
        let cases = [
            (
                "requestDetails",
                "requestPackageName",
                json!("com.example.other"),
            ),
            (
                "requestDetails",
                "requestHash",
                json!(URL_SAFE_NO_PAD.encode([0; 32])),
            ),
            (
                "requestDetails",
                "timestampMillis",
                json!((TIME - MAX_AGE_MILLIS - 1).to_string()),
            ),
            (
                "requestDetails",
                "timestampMillis",
                json!((TIME + FUTURE_TOLERANCE_MILLIS + 1).to_string()),
            ),
            (
                "appIntegrity",
                "appRecognitionVerdict",
                json!("UNEVALUATED"),
            ),
            ("appIntegrity", "packageName", json!("com.example.other")),
            ("appIntegrity", "certificateSha256Digest", json!([])),
            (
                "appIntegrity",
                "certificateSha256Digest",
                json!([
                    URL_SAFE_NO_PAD.encode(CERT),
                    URL_SAFE_NO_PAD.encode([0; 32])
                ]),
            ),
            (
                "deviceIntegrity",
                "deviceRecognitionVerdict",
                json!(["MEETS_BASIC_INTEGRITY"]),
            ),
        ];
        for (section, field, value) in cases {
            let mut response = decode_response();
            response["tokenPayloadExternal"][section][field] = value;
            assert!(
                matches!(validate(response), Err(PlayIntegrityError::Rejected)),
                "{section}.{field}"
            );
        }
        assert!(matches!(
            validate(json!({"tokenPayloadExternal": {}})),
            Err(PlayIntegrityError::Rejected)
        ));
    }

    #[test]
    fn registrations_are_scoped_by_project_and_pacific_calendar_day() {
        for (before_midnight, midnight, day_before, day) in [
            (
                "2026-01-02T07:59:59Z",
                "2026-01-02T08:00:00Z",
                "2026-01-01",
                "2026-01-02",
            ),
            (
                "2026-07-02T06:59:59Z",
                "2026-07-02T07:00:00Z",
                "2026-07-01",
                "2026-07-02",
            ),
        ] {
            let now = midnight.parse().unwrap();
            assert_eq!(
                registration_quota_key("test", before_midnight.parse().unwrap()),
                play_integrity_registration_quota_key("test", day_before)
            );
            assert_eq!(
                registration_quota_key("test", now),
                play_integrity_registration_quota_key("test", day)
            );
            assert_ne!(
                registration_quota_key("test", now),
                registration_quota_key("other", now)
            );
        }
    }

    #[tokio::test]
    async fn uses_the_cached_oauth_token_and_checks_the_decode_response() {
        use axum::{
            Json, Router,
            http::{HeaderMap, header},
            routing::post,
        };

        let app = Router::new().route(
            "/v1/com.example.eddist:decodeIntegrityToken",
            post(|headers: HeaderMap, Json(body): Json<Value>| async move {
                assert_eq!(headers[header::AUTHORIZATION], "Bearer cached-test-token");
                let mut response = decode_response();
                response["tokenPayloadExternal"]["requestDetails"]["timestampMillis"] =
                    json!(Utc::now().timestamp_millis().to_string());
                match body["integrity_token"].as_str() {
                    Some("valid-proof") => (StatusCode::OK, Json(response)),
                    Some("invalid-proof") => (StatusCode::BAD_REQUEST, Json(json!({}))),
                    Some("unauthorized") => (StatusCode::UNAUTHORIZED, Json(json!({}))),
                    Some("forbidden") => (StatusCode::FORBIDDEN, Json(json!({}))),
                    _ => (StatusCode::SERVICE_UNAVAILABLE, Json(json!({}))),
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://{}/v1/{PACKAGE}:decodeIntegrityToken",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let cached_token = || CachedAccessToken {
            value: "cached-test-token".to_owned(),
            refresh_at: Instant::now() + Duration::from_secs(60),
        };
        let verifier = PlayIntegrityVerifier {
            http: Client::new(),
            package_name: PACKAGE.to_owned(),
            certificate_digests: vec![CERT],
            decode_url: Url::parse(&url).unwrap(),
            service_account: GoogleServiceAccount {
                project_id: String::from("test-project"),
                email: String::new(),
                signing_key: EncodingKey::from_secret(b"unused in the cached-token test"),
                header: Header::new(Algorithm::RS256),
            },
            access_token: Mutex::new(Some(cached_token())),
            daily_registration_limit: 8000,
        };
        assert!(
            verifier
                .verify("valid-proof", &CHALLENGE_HASH, None)
                .await
                .is_ok()
        );
        assert!(matches!(
            verifier
                .verify("invalid-proof", &CHALLENGE_HASH, None)
                .await,
            Err(PlayIntegrityError::Rejected)
        ));
        assert!(matches!(
            verifier
                .verify("upstream-failure", &CHALLENGE_HASH, None)
                .await,
            Err(PlayIntegrityError::Unavailable)
        ));
        assert!(verifier.access_token.lock().await.is_some());
        for proof in ["unauthorized", "forbidden"] {
            *verifier.access_token.lock().await = Some(cached_token());
            verifier.invalidate_access_token("previous-token").await;
            assert!(verifier.access_token.lock().await.is_some());
            assert!(matches!(
                verifier.verify(proof, &CHALLENGE_HASH, None).await,
                Err(PlayIntegrityError::Unavailable)
            ));
            assert!(verifier.access_token.lock().await.is_none());
        }
        server.abort();
    }
}
