use std::{fmt, sync::Arc};

use base64::Engine;
use http::HeaderName;
use jsonwebtoken::{DecodingKey, EncodingKey};

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
    pub tinker_secret: TinkerSecret,
    pub asn_header_name: HeaderName,
    pub restrict_thread_creation_on_no_tinker: bool,
    pub client_secret_symmetric_encryption: bool,
    pub metrics: MetricsConfig,
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
        url::Url::parse(&base_url).map_err(|e| invalid("BASE_URL", e))?;

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
            tinker_secret,
            asn_header_name,
            restrict_thread_creation_on_no_tinker: flag("RESTRICT_THREAD_CREATION_ON_NO_TINKER"),
            client_secret_symmetric_encryption,
            metrics: MetricsConfig {
                enabled: !flag("DISABLE_METRICS"),
                http_layer: flag("AXUM_METRICS"),
            },
        })
    }

    pub fn for_test() -> Self {
        let secret = base64::engine::general_purpose::STANDARD.encode(uuid::Uuid::now_v7());
        Self {
            base_url: "http://localhost:8000".to_string(),
            tinker_secret: TinkerSecret::new(secret).expect("generated secret is base64"),
            asn_header_name: HeaderName::from_static("x-asn-num"),
            restrict_thread_creation_on_no_tinker: false,
            client_secret_symmetric_encryption: false,
            metrics: MetricsConfig {
                enabled: false,
                http_layer: false,
            },
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
        assert_eq!(config.app.asn_header_name, "x-asn-num");
        assert!(config.app.metrics.enabled);
        assert!(!config.app.restrict_thread_creation_on_no_tinker);
        assert!(!config.app.client_secret_symmetric_encryption);
    }

    #[test]
    fn invalid_values_are_rejected_at_startup() {
        for (name, value) in [
            ("PORT", "eighty"),
            ("BASE_URL", "localhost"),
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
