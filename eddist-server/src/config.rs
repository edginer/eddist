use std::{fmt, path::PathBuf, sync::Arc};

use base64::Engine;
use http::HeaderName;
use jsonwebtoken::{DecodingKey, EncodingKey};

const DEFAULT_BBS_NAME: &str = "エッヂ掲示板";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} must be set")]
    Missing(&'static str),
    #[error("{name} is invalid: {reason}")]
    Invalid { name: &'static str, reason: String },
}

fn invalid(name: &'static str, reason: impl fmt::Display) -> ConfigError {
    ConfigError::Invalid {
        name,
        reason: reason.to_string(),
    }
}

/// Values only `main` needs to open connections; the app itself receives the connected clients.
pub struct ServerConfig {
    pub redis_url: String,
    pub database_url: String,
    pub port: u16,
    pub s3: S3Config,
    pub app: AppConfig,
}

pub struct S3Config {
    pub endpoint: String,
    pub bucket_name: String,
    pub access_key: String,
    pub secret_access_key: String,
}

#[derive(Clone)]
pub struct AppConfig {
    pub base_url: String,
    pub(crate) base_url_host: String,
    pub(crate) bbs_name: String,
    pub tinker_secret: TinkerSecret,
    pub asn_header_name: HeaderName,
    pub restrict_thread_creation_on_no_tinker: bool,
    pub client_secret_symmetric_encryption: bool,
    pub metrics: MetricsConfig,
    pub(crate) app_attestation: Option<AppAttestationConfig>,
}

#[derive(Clone)]
pub(crate) struct AppAttestationConfig {
    pub origin: String,
    pub token_key: [u8; 32],
    pub ios: Option<AppAttestConfig>,
    pub android: Option<PlayIntegrityConfig>,
}

impl AppAttestationConfig {
    fn from_lookup(
        lookup: &impl Fn(&str) -> Option<String>,
        base_url: &url::Url,
    ) -> Result<Option<Self>, ConfigError> {
        let ios = AppAttestConfig::from_lookup(lookup)?;
        let android = PlayIntegrityConfig::from_lookup(lookup)?;
        if ios.is_none() && android.is_none() {
            return Ok(None);
        }
        if base_url.scheme() != "https"
            || !base_url.username().is_empty()
            || base_url.password().is_some()
        {
            return Err(invalid(
                "BASE_URL",
                "app attestation requires an HTTPS origin",
            ));
        }
        let key_name = "APP_ATTESTATION_TOKEN_KEY";
        let token_key = lookup(key_name).ok_or(ConfigError::Missing(key_name))?;
        let token_key = base64::engine::general_purpose::STANDARD
            .decode(token_key.trim())
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| invalid(key_name, "must be 32 bytes encoded as base64"))?;
        Ok(Some(Self {
            origin: base_url.origin().ascii_serialization(),
            token_key,
            ios,
            android,
        }))
    }
}

#[derive(Clone)]
pub(crate) struct AppAttestConfig {
    pub app_id: String,
    pub development: bool,
}

impl AppAttestConfig {
    fn from_lookup(lookup: &impl Fn(&str) -> Option<String>) -> Result<Option<Self>, ConfigError> {
        let Some(app_id) = lookup("APP_ATTEST_APP_ID") else {
            return Ok(None);
        };
        let app_id = app_id.trim().to_owned();
        let valid_app_id = app_id.split_once('.').is_some_and(|(prefix, bundle)| {
            prefix.len() == 10
                && prefix.bytes().all(|c| c.is_ascii_alphanumeric())
                && !bundle.is_empty()
                && bundle.len() <= 255
                && bundle.split('.').all(|label| {
                    !label.is_empty()
                        && label
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
                })
        });
        if !valid_app_id {
            return Err(invalid(
                "APP_ATTEST_APP_ID",
                "expected App ID prefix and bundle ID",
            ));
        }
        let development = match lookup("APP_ATTEST_ENVIRONMENT").as_deref().map(str::trim) {
            None | Some("production") => false,
            Some("development") => true,
            Some(_) => {
                return Err(invalid(
                    "APP_ATTEST_ENVIRONMENT",
                    "expected production or development",
                ));
            }
        };
        Ok(Some(Self {
            app_id,
            development,
        }))
    }
}

#[derive(Clone)]
pub(crate) struct PlayIntegrityConfig {
    pub package_name: String,
    pub certificate_digests: Vec<[u8; 32]>,
    pub service_account_file: PathBuf,
    pub daily_registration_limit: u32,
}

impl PlayIntegrityConfig {
    fn from_lookup(lookup: &impl Fn(&str) -> Option<String>) -> Result<Option<Self>, ConfigError> {
        let Some(package_name) = lookup("PLAY_INTEGRITY_PACKAGE_NAME") else {
            return Ok(None);
        };
        let package_name = package_name.trim().to_owned();
        if package_name.len() > 255
            || !package_name.contains('.')
            || !package_name.split('.').all(|segment| {
                segment
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_alphabetic())
                    && segment
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            })
        {
            return Err(invalid(
                "PLAY_INTEGRITY_PACKAGE_NAME",
                "expected an Android application ID",
            ));
        }
        let digest_list = lookup("PLAY_INTEGRITY_CERT_SHA256")
            .ok_or(ConfigError::Missing("PLAY_INTEGRITY_CERT_SHA256"))?;
        let certificate_digests = digest_list
            .split(',')
            .map(parse_certificate_digest)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                invalid(
                    "PLAY_INTEGRITY_CERT_SHA256",
                    "expected comma-separated SHA-256 fingerprints in hex",
                )
            })?;
        let service_account_file = lookup("PLAY_INTEGRITY_SERVICE_ACCOUNT_FILE")
            .ok_or(ConfigError::Missing("PLAY_INTEGRITY_SERVICE_ACCOUNT_FILE"))?;
        if service_account_file.trim().is_empty() {
            return Err(invalid(
                "PLAY_INTEGRITY_SERVICE_ACCOUNT_FILE",
                "must be a file path",
            ));
        }
        let registration_limit = "PLAY_INTEGRITY_DAILY_REGISTRATION_LIMIT";
        let daily_registration_limit = lookup(registration_limit)
            .as_deref()
            .map(|value| value.trim().parse::<u32>())
            .transpose()
            .map_err(|error| invalid(registration_limit, error))?
            .unwrap_or(8000);
        Ok(Some(Self {
            package_name,
            certificate_digests,
            service_account_file: service_account_file.into(),
            daily_registration_limit,
        }))
    }
}

fn parse_certificate_digest(fingerprint: &str) -> Option<[u8; 32]> {
    let hex = fingerprint.trim().replace(':', "");
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut digest = [0; 32];
    for (byte, pair) in digest.iter_mut().zip(hex.as_bytes().chunks_exact(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(digest)
}

#[derive(Debug, Clone, Copy)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub http_layer: bool,
}

#[derive(Clone)]
pub struct TinkerSecret(Arc<TinkerSecretInner>);

struct TinkerSecretInner {
    raw: String,
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl TinkerSecret {
    pub fn new(raw: String) -> Result<Self, ConfigError> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw.trim())
            .map_err(|_| invalid("TINKER_SECRET", "must be base64"))?;
        if bytes.is_empty() {
            return Err(invalid("TINKER_SECRET", "must not be empty"));
        }
        Ok(Self(Arc::new(TinkerSecretInner {
            encoding: EncodingKey::from_secret(&bytes),
            decoding: DecodingKey::from_secret(&bytes),
            raw,
        })))
    }

    /// Cap hashes are derived from the untrimmed value, so changing this changes every cap.
    pub fn raw(&self) -> &str {
        &self.0.raw
    }

    pub fn encoding_key(&self) -> &EncodingKey {
        &self.0.encoding
    }

    pub fn decoding_key(&self) -> &DecodingKey {
        &self.0.decoding
    }
}

impl fmt::Debug for TinkerSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TinkerSecret(***)")
    }
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let required = |name: &'static str| lookup(name).ok_or(ConfigError::Missing(name));

        let port = match lookup("PORT") {
            Some(port) => port.parse::<u16>().map_err(|e| invalid("PORT", e))?,
            None => 8080,
        };
        let r2_account_id = required("R2_ACCOUNT_ID")?;

        Ok(Self {
            redis_url: required("REDIS_URL")?,
            database_url: required("DATABASE_URL")?,
            port,
            s3: S3Config {
                endpoint: format!("https://{}.r2.cloudflarestorage.com", r2_account_id.trim()),
                bucket_name: required("S3_BUCKET_NAME")?.trim().to_string(),
                access_key: required("S3_ACCESS_KEY")?.trim().to_string(),
                secret_access_key: required("S3_ACCESS_SECRET_KEY")?.trim().to_string(),
            },
            app: AppConfig::from_lookup(&lookup)?,
        })
    }
}

impl AppConfig {
    fn from_lookup(lookup: &impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let flag = |name: &str| lookup(name).as_deref() == Some("true");

        let base_url = lookup("BASE_URL").ok_or(ConfigError::Missing("BASE_URL"))?;
        let parsed_base_url = url::Url::parse(&base_url).map_err(|e| invalid("BASE_URL", e))?;
        let base_url_host = parsed_base_url
            .host_str()
            .ok_or_else(|| invalid("BASE_URL", "must include a host"))?
            .to_owned();

        let tinker_secret = TinkerSecret::new(
            lookup("TINKER_SECRET").ok_or(ConfigError::Missing("TINKER_SECRET"))?,
        )?;

        let asn_header_name = match lookup("ASN_NUMBER_HEADER_NAME") {
            Some(name) => {
                HeaderName::try_from(name).map_err(|e| invalid("ASN_NUMBER_HEADER_NAME", e))?
            }
            None => HeaderName::from_static("x-asn-num"),
        };

        let client_secret_symmetric_encryption = match lookup("CLIENT_SECRET_SYMMETRIC_ENCRYPTION")
        {
            Some(value) => value
                .parse::<bool>()
                .map_err(|e| invalid("CLIENT_SECRET_SYMMETRIC_ENCRYPTION", e))?,
            None => false,
        };

        Ok(Self {
            base_url,
            base_url_host,
            bbs_name: lookup("BBS_NAME").unwrap_or_else(|| DEFAULT_BBS_NAME.to_string()),
            tinker_secret,
            asn_header_name,
            restrict_thread_creation_on_no_tinker: flag("RESTRICT_THREAD_CREATION_ON_NO_TINKER"),
            client_secret_symmetric_encryption,
            metrics: MetricsConfig {
                enabled: !flag("DISABLE_METRICS"),
                http_layer: flag("AXUM_METRICS"),
            },
            app_attestation: AppAttestationConfig::from_lookup(lookup, &parsed_base_url)?,
        })
    }

    pub fn for_test() -> Self {
        let secret = base64::engine::general_purpose::STANDARD.encode(uuid::Uuid::now_v7());
        Self {
            base_url: "http://localhost:8000".to_string(),
            base_url_host: "localhost".to_string(),
            bbs_name: DEFAULT_BBS_NAME.to_string(),
            tinker_secret: TinkerSecret::new(secret).expect("generated secret is base64"),
            asn_header_name: HeaderName::from_static("x-asn-num"),
            restrict_thread_creation_on_no_tinker: false,
            client_secret_symmetric_encryption: false,
            metrics: MetricsConfig {
                enabled: false,
                http_layer: false,
            },
            app_attestation: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup<'a>(vars: &'a HashMap<&str, &str>) -> impl Fn(&str) -> Option<String> + 'a {
        |name| vars.get(name).map(|v| v.to_string())
    }

    fn valid_vars() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("REDIS_URL", "redis://localhost:6379"),
            ("DATABASE_URL", "mysql://root@localhost/eddist"),
            ("R2_ACCOUNT_ID", " account "),
            ("S3_BUCKET_NAME", "bucket\n"),
            ("S3_ACCESS_KEY", "key"),
            ("S3_ACCESS_SECRET_KEY", "secret"),
            ("BASE_URL", "http://localhost:8000"),
            ("TINKER_SECRET", "cHJvYmUtdGlua2VyLXNlY3JldC0zMi1ieXRlcyEh"),
        ])
    }

    #[test]
    fn defaults_apply_when_optional_values_are_unset() {
        let vars = valid_vars();
        let config = ServerConfig::from_lookup(lookup(&vars)).unwrap();
        assert_eq!(config.port, 8080);
        assert_eq!(
            config.s3.endpoint,
            "https://account.r2.cloudflarestorage.com"
        );
        assert_eq!(config.s3.bucket_name, "bucket");
        assert_eq!(config.app.base_url_host, "localhost");
        assert_eq!(config.app.bbs_name, "エッヂ掲示板");
        assert_eq!(config.app.asn_header_name, "x-asn-num");
        assert!(config.app.metrics.enabled);
        assert!(!config.app.restrict_thread_creation_on_no_tinker);
        assert!(!config.app.client_secret_symmetric_encryption);
        assert!(config.app.app_attestation.is_none());
    }

    #[test]
    fn app_attest_config_requires_an_https_origin_and_a_token_key() {
        let mut vars = valid_vars();
        vars.insert("APP_ATTEST_APP_ID", "ABC1234567.com.example.app");
        assert!(ServerConfig::from_lookup(lookup(&vars)).is_err());
        vars.insert("BASE_URL", "https://BBS.Example:443/");
        assert!(ServerConfig::from_lookup(lookup(&vars)).is_err());
        vars.insert(
            "APP_ATTESTATION_TOKEN_KEY",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        );
        let config = ServerConfig::from_lookup(lookup(&vars))
            .unwrap()
            .app
            .app_attestation
            .unwrap();
        assert_eq!(config.origin, "https://bbs.example");
        assert!(!config.ios.unwrap().development);
        vars.insert("APP_ATTEST_ENVIRONMENT", "development\n");
        assert!(
            ServerConfig::from_lookup(lookup(&vars))
                .unwrap()
                .app
                .app_attestation
                .unwrap()
                .ios
                .unwrap()
                .development
        );
    }

    #[test]
    fn configured_bbs_name_is_used() {
        let mut vars = valid_vars();
        vars.insert("BBS_NAME", "テスト掲示板");
        let config = ServerConfig::from_lookup(lookup(&vars)).unwrap();
        assert_eq!(config.app.bbs_name, "テスト掲示板");
    }

    #[test]
    fn play_integrity_can_be_configured_without_ios() {
        let mut vars = valid_vars();
        let fingerprint = ["aB"; 32].join(":");
        vars.extend([
            ("BASE_URL", "https://bbs.example"),
            (
                "APP_ATTESTATION_TOKEN_KEY",
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
            ),
            ("PLAY_INTEGRITY_PACKAGE_NAME", "com.example.eddist"),
            (
                "PLAY_INTEGRITY_SERVICE_ACCOUNT_FILE",
                "/run/secrets/play-integrity.json",
            ),
            ("PLAY_INTEGRITY_CERT_SHA256", fingerprint.as_str()),
        ]);
        let config = ServerConfig::from_lookup(lookup(&vars))
            .unwrap()
            .app
            .app_attestation
            .unwrap();
        assert!(config.ios.is_none());
        let android = config.android.unwrap();
        assert_eq!(android.package_name, "com.example.eddist");
        assert_eq!(android.certificate_digests, vec![[0xab; 32]]);
        assert_eq!(android.daily_registration_limit, 8000);

        vars.insert("PLAY_INTEGRITY_DAILY_REGISTRATION_LIMIT", "3000");
        let config = ServerConfig::from_lookup(lookup(&vars)).unwrap();
        assert_eq!(
            config
                .app
                .app_attestation
                .unwrap()
                .android
                .unwrap()
                .daily_registration_limit,
            3000
        );

        vars.insert("PLAY_INTEGRITY_CERT_SHA256", "ab");
        assert!(ServerConfig::from_lookup(lookup(&vars)).is_err());
    }

    #[test]
    fn invalid_values_are_rejected_at_startup() {
        for (name, value) in [
            ("PORT", "eighty"),
            ("BASE_URL", "localhost"),
            ("BASE_URL", "file:///eddist"),
            ("TINKER_SECRET", "not base64!"),
            ("TINKER_SECRET", ""),
            ("ASN_NUMBER_HEADER_NAME", "bad header"),
            ("CLIENT_SECRET_SYMMETRIC_ENCRYPTION", "yes"),
        ] {
            let mut vars = valid_vars();
            vars.insert(name, value);
            let err = ServerConfig::from_lookup(lookup(&vars))
                .err()
                .unwrap_or_else(|| panic!("{name}={value:?} should be rejected"));
            assert!(
                matches!(err, ConfigError::Invalid { name: n, .. } if n == name),
                "{name}: {err}"
            );
        }
    }

    #[test]
    fn missing_required_values_are_named() {
        let mut vars = valid_vars();
        vars.remove("TINKER_SECRET");
        assert!(matches!(
            ServerConfig::from_lookup(lookup(&vars)),
            Err(ConfigError::Missing("TINKER_SECRET"))
        ));
    }

    #[test]
    fn invalid_secret_errors_do_not_echo_the_value() {
        let mut vars = valid_vars();
        vars.insert("TINKER_SECRET", "super-secret-value!");
        let err = ServerConfig::from_lookup(lookup(&vars)).err().unwrap();
        assert!(!err.to_string().contains("super-secret-value"));
    }
}
