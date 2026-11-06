use std::{collections::HashMap, sync::Arc, time::Duration};

use eddist_core::symmetric;
use openidconnect::{ClientId, ClientSecret, IssuerUrl, core::CoreProviderMetadata};
use tokio::sync::RwLock;

use crate::{
    config::AppConfig, domain::user::idp::Idp, external::oidc_client::OidcClient,
    repositories::idp_repository::IdpRepository,
};

/// Only for redirects: callbacks always rediscover so ID tokens are verified
/// against the IdP's current JWKS.
const OIDC_METADATA_TTL: Duration = Duration::from_secs(300);
const OIDC_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The IdP row is read on every lookup so admin edits and deletions apply
/// immediately; only discovery metadata is cached, per IdP.
pub struct OidcClientRegistry {
    idp_repo: Arc<dyn IdpRepository>,
    config: Arc<AppConfig>,
    http_client: reqwest::Client,
    metadata: RwLock<HashMap<String, CachedMetadata>>,
}

struct CachedMetadata {
    oidc_config_url: String,
    metadata: CoreProviderMetadata,
    fetched_at: tokio::time::Instant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MetadataFreshness {
    /// Authorization requests only need the endpoints.
    Cached,
    /// Code exchange verifies the ID token with the JWKS.
    Fresh,
}

impl OidcClientRegistry {
    pub fn new(idp_repo: Arc<dyn IdpRepository>, config: Arc<AppConfig>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(OIDC_REQUEST_TIMEOUT)
            .build()
            .expect("failed to build the OIDC HTTP client");
        Self {
            idp_repo,
            config,
            http_client,
            metadata: RwLock::new(HashMap::new()),
        }
    }

    /// Errors mean the DB read failed, or discovery failed with no metadata
    /// from the same config URL to fall back on.
    pub async fn get(
        &self,
        idp_name: &str,
        freshness: MetadataFreshness,
    ) -> anyhow::Result<Option<(Idp, Arc<OidcClient>)>> {
        let Some(idp) = self
            .idp_repo
            .get_idps()
            .await?
            .into_iter()
            .find(|idp| idp.idp_name == idp_name)
        else {
            self.metadata.write().await.remove(idp_name);
            return Ok(None);
        };

        let metadata = self.metadata_for(&idp, freshness).await?;
        let client_secret = if self.config.client_secret_symmetric_encryption {
            decrypt_client_secret(&idp.client_secret)?
        } else {
            idp.client_secret.clone()
        };
        let client = Arc::new(OidcClient::new(
            ClientId::new(idp.client_id.clone()),
            ClientSecret::new(client_secret),
            metadata,
            &self.config.base_url,
            self.http_client.clone(),
        )?);
        Ok(Some((idp, client)))
    }

    async fn metadata_for(
        &self,
        idp: &Idp,
        freshness: MetadataFreshness,
    ) -> anyhow::Result<CoreProviderMetadata> {
        let cached = {
            let cache = self.metadata.read().await;
            cache
                .get(&idp.idp_name)
                .filter(|c| c.oidc_config_url == idp.oidc_config_url)
                .map(|c| (c.metadata.clone(), c.fetched_at))
        };
        if freshness == MetadataFreshness::Cached
            && let Some((metadata, fetched_at)) = &cached
            && fetched_at.elapsed() < OIDC_METADATA_TTL
        {
            return Ok(metadata.clone());
        }

        match self.discover(idp).await {
            Ok(metadata) => {
                self.metadata.write().await.insert(
                    idp.idp_name.clone(),
                    CachedMetadata {
                        oidc_config_url: idp.oidc_config_url.clone(),
                        metadata: metadata.clone(),
                        fetched_at: tokio::time::Instant::now(),
                    },
                );
                Ok(metadata)
            }
            Err(e) => match cached {
                Some((metadata, _)) => {
                    tracing::warn!(
                        idp = %idp.idp_name,
                        error = %format_args!("{e:#}"),
                        "failed to refresh OIDC metadata; using the previous one"
                    );
                    Ok(metadata)
                }
                None => Err(e),
            },
        }
    }

    async fn discover(&self, idp: &Idp) -> anyhow::Result<CoreProviderMetadata> {
        let issuer_url = IssuerUrl::new(
            idp.oidc_config_url
                .trim_end_matches("/.well-known/openid-configuration")
                .to_string(),
        )
        .map_err(|e| anyhow::anyhow!("invalid OIDC issuer URL for '{}': {e}", idp.idp_name))?;
        CoreProviderMetadata::discover_async(issuer_url, &self.http_client)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "failed to discover OIDC metadata for '{}': {e}",
                    idp.idp_name
                )
            })
    }
}

fn decrypt_client_secret(b64_secret: &str) -> anyhow::Result<String> {
    symmetric::decrypt(b64_secret)
        .map_err(|e| anyhow::anyhow!("failed to decrypt client_secret: {e:#}"))
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;
    use crate::repositories::idp_repository::IdpRepository;
    use eddist_core::test_utils::EnvGuard;
    use uuid::Uuid;

    #[derive(Default)]
    struct FakeIdpRepo {
        idps: Mutex<Vec<Idp>>,
        fail: std::sync::atomic::AtomicBool,
    }

    impl FakeIdpRepo {
        fn with(idps: Vec<Idp>) -> Arc<Self> {
            Arc::new(Self {
                idps: Mutex::new(idps),
                ..Default::default()
            })
        }
    }

    #[async_trait::async_trait]
    impl IdpRepository for FakeIdpRepo {
        async fn get_idps(&self) -> anyhow::Result<Vec<Idp>> {
            anyhow::ensure!(!self.fail.load(Ordering::SeqCst), "database unavailable");
            Ok(self.idps.lock().unwrap().clone())
        }
    }

    fn idp(oidc_config_url: String) -> Idp {
        Idp {
            id: Uuid::now_v7(),
            idp_name: "test-idp".to_string(),
            idp_display_name: "Test IdP".to_string(),
            idp_logo_svg: None,
            oidc_config_url,
            client_id: "client-id".to_string(),
            client_secret: "client-secret".to_string(),
            enabled: true,
        }
    }

    fn registry(repo: Arc<FakeIdpRepo>) -> OidcClientRegistry {
        OidcClientRegistry::new(repo, Arc::new(AppConfig::for_test()))
    }

    async fn serve_discovery() -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        use axum::{Json, Router, routing::get};

        let requests = Arc::new(AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let app = Router::new()
            .route(
                "/.well-known/openid-configuration",
                get({
                    let requests = requests.clone();
                    let issuer = issuer.clone();
                    move || {
                        let requests = requests.clone();
                        let issuer = issuer.clone();
                        async move {
                            requests.fetch_add(1, Ordering::SeqCst);
                            Json(serde_json::json!({
                                "issuer": issuer,
                                "authorization_endpoint": format!("{issuer}/authorize"),
                                "token_endpoint": format!("{issuer}/token"),
                                "jwks_uri": format!("{issuer}/jwks"),
                                "response_types_supported": ["code"],
                                "subject_types_supported": ["public"],
                                "id_token_signing_alg_values_supported": ["RS256"],
                            }))
                        }
                    }
                }),
            )
            .route(
                "/jwks",
                get(|| async { Json(serde_json::json!({ "keys": [] })) }),
            );
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (issuer, requests, server)
    }

    #[test]
    fn test_decrypt_client_secret_round_trip() {
        let _env = EnvGuard::set(&[("TINKER_SECRET", "a_very_secret_key_that_is_not_32_bytes!")]);
        let secret = "my_secret_client_secret";
        let encrypted = symmetric::encrypt(secret);
        assert_eq!(decrypt_client_secret(&encrypted).unwrap(), secret);
    }

    #[test]
    fn test_decrypt_client_secret_errors_do_not_echo_the_input() {
        let _env = EnvGuard::set(&[("TINKER_SECRET", "a_very_secret_key_that_is_not_32_bytes!")]);
        let err = decrypt_client_secret("plain-secret-value").unwrap_err();
        assert!(!err.to_string().contains("plain-secret-value"));
    }

    #[tokio::test]
    async fn unknown_idp_is_not_an_error() {
        let registry = registry(FakeIdpRepo::with(Vec::new()));
        assert!(
            registry
                .get("missing", MetadataFreshness::Cached)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn discovery_failure_without_cache_is_an_error() {
        // Port 1 refuses connections, so discovery fails instead of panicking.
        let registry = registry(FakeIdpRepo::with(vec![idp(
            "http://127.0.0.1:1".to_string()
        )]));
        assert!(
            registry
                .get("test-idp", MetadataFreshness::Cached)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn redirects_reuse_metadata_but_callbacks_rediscover() {
        let (issuer, requests, server) = serve_discovery().await;
        let registry = registry(FakeIdpRepo::with(vec![idp(issuer)]));

        for _ in 0..2 {
            registry
                .get("test-idp", MetadataFreshness::Cached)
                .await
                .unwrap()
                .unwrap();
        }
        assert_eq!(requests.load(Ordering::SeqCst), 1);

        registry
            .get("test-idp", MetadataFreshness::Fresh)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(requests.load(Ordering::SeqCst), 2);

        server.abort();
    }

    #[tokio::test]
    async fn deleted_idp_is_not_served_from_cache() {
        let (issuer, _, server) = serve_discovery().await;
        let repo = FakeIdpRepo::with(vec![idp(issuer)]);
        let registry = registry(repo.clone());
        registry
            .get("test-idp", MetadataFreshness::Cached)
            .await
            .unwrap()
            .unwrap();

        repo.idps.lock().unwrap().clear();
        assert!(
            registry
                .get("test-idp", MetadataFreshness::Cached)
                .await
                .unwrap()
                .is_none()
        );

        repo.fail.store(true, Ordering::SeqCst);
        assert!(
            registry
                .get("test-idp", MetadataFreshness::Cached)
                .await
                .is_err()
        );

        server.abort();
    }
}
