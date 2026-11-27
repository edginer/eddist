use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use eddist_core::redis_keys::{app_attestation_challenge_key, app_attestation_rate_limit_key};
use eddist_entity::{app_attest_key, app_credential, db_time::truncate_to_millis};
use hmac::{Hmac, KeyInit, Mac};
use redis::{AsyncCommands, aio::ConnectionManager};
use rustls_pki_types::UnixTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    config::AppAttestationConfig,
    external::{
        app_attest::{AppAttestVerifier, InvalidProof},
        play_integrity::{PlayIntegrityError, PlayIntegrityVerifier},
    },
    repositories::app_credentials_repository::{
        AppCredentialsRepository, RefreshResult, RegistrationClient,
    },
    utils::incr_fixed_window,
};

pub(crate) const PROTOCOL_VERSION: u8 = 1;
const CHALLENGE_TTL_SECONDS: u64 = 300;
const REGISTRATION_RECOVERY_WINDOW: TimeDelta = TimeDelta::minutes(10);
const REATTEST_INTERVAL: TimeDelta = TimeDelta::days(7);

#[derive(Debug, thiserror::Error)]
pub(crate) enum AttestationError {
    #[error("app attestation is disabled")]
    Disabled,
    #[error("new authentication is closed")]
    RegistrationClosed,
    #[error("invalid app attestation request")]
    InvalidRequest,
    #[error("app attestation request body is too large")]
    RequestTooLarge,
    #[error("invalid bearer token")]
    InvalidToken,
    #[error("client IP is unavailable")]
    MissingClientIp,
    #[error("invalid or expired challenge")]
    InvalidChallenge,
    #[error("registration conflicts with an existing credential")]
    RegistrationConflict,
    #[error("app token has been revoked")]
    TokenRevoked,
    #[error("app attestation proof was rejected")]
    Rejected,
    #[error("Play Integrity is unavailable")]
    ProviderUnavailable,
    #[error("app attestation request timed out")]
    TimedOut,
    #[error("too many app attestation requests")]
    RateLimited,
    #[error("Play Integrity registration quota exceeded")]
    RegistrationQuotaExceeded,
    #[error("attestation data store is unavailable")]
    Redis(#[from] redis::RedisError),
    #[error("attestation credentials store is unavailable")]
    Database(#[from] sea_orm::DbErr),
}

impl From<InvalidProof> for AttestationError {
    fn from(_: InvalidProof) -> Self {
        Self::Rejected
    }
}

impl From<PlayIntegrityError> for AttestationError {
    fn from(error: PlayIntegrityError) -> Self {
        match error {
            PlayIntegrityError::Rejected => Self::Rejected,
            PlayIntegrityError::Unavailable => Self::ProviderUnavailable,
            PlayIntegrityError::RegistrationQuotaExceeded => Self::RegistrationQuotaExceeded,
            PlayIntegrityError::QuotaStore(error) => Self::Redis(error),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Platform {
    Ios,
    Android,
}

impl Platform {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
        }
    }

    pub(crate) fn provider_name(self) -> &'static str {
        match self {
            Self::Ios => "app_attest",
            Self::Android => "play_integrity",
        }
    }
}

enum Verifier {
    Ios(AppAttestVerifier),
    Android(Box<PlayIntegrityVerifier>),
}

pub(crate) struct AppAttestationService {
    origin: String,
    token_key: [u8; 32],
    verifier: Verifier,
    scope_hash: [u8; 32],
    redis: ConnectionManager,
    repository: AppCredentialsRepository,
}

pub(crate) enum ChallengeSubject {
    Registration([u8; 32]),
    Refresh(Uuid),
}

impl ChallengeSubject {
    fn purpose(&self) -> &str {
        match self {
            Self::Registration(_) => "register",
            Self::Refresh(_) => "refresh",
        }
    }

    fn key_part(&self) -> String {
        match self {
            Self::Registration(hash) => format!("register:{}", URL_SAFE_NO_PAD.encode(hash)),
            Self::Refresh(id) => format!("refresh:{id}"),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct IssuedChallenge {
    challenge_id: Uuid,
    challenge: String,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub(crate) struct RegisteredApp {
    token: String,
    reattest_after: DateTime<Utc>,
}

pub(crate) struct RegistrationProof {
    pub challenge_id: Uuid,
    pub registration_key: [u8; 32],
    pub evidence: RegistrationEvidence,
}

pub(crate) enum RegistrationEvidence {
    Ios {
        key_id: [u8; 32],
        attestation: Vec<u8>,
    },
    Android(String),
}

pub(crate) enum RefreshEvidence {
    Ios(Vec<u8>),
    Android(String),
}

impl AppAttestationService {
    pub(crate) fn configured_services(
        config: Option<&AppAttestationConfig>,
        redis: ConnectionManager,
        repository: AppCredentialsRepository,
    ) -> anyhow::Result<Vec<Self>> {
        let Some(config) = config else {
            return Ok(Vec::new());
        };
        let new_service = |verifier, scope_hash| Self {
            origin: config.origin.clone(),
            token_key: config.token_key,
            verifier,
            scope_hash,
            redis: redis.clone(),
            repository: repository.clone(),
        };
        let scope = Sha256::new()
            .chain_update(&config.origin)
            .chain_update(b"\n");
        let mut services = Vec::new();
        if let Some(ios) = &config.ios {
            let verifier = AppAttestVerifier::new(&ios.app_id, ios.development)?;
            let scope_hash = scope
                .clone()
                .chain_update(&ios.app_id)
                .chain_update([u8::from(ios.development)])
                .finalize()
                .into();
            services.push(new_service(Verifier::Ios(verifier), scope_hash));
        }
        if let Some(android) = &config.android {
            let verifier = PlayIntegrityVerifier::new(android)?;
            let scope_hash = scope
                .chain_update(b"android\n")
                .chain_update(&android.package_name)
                .finalize()
                .into();
            services.push(new_service(
                Verifier::Android(Box::new(verifier)),
                scope_hash,
            ));
        }
        Ok(services)
    }

    pub(crate) fn platform(&self) -> Platform {
        match &self.verifier {
            Verifier::Ios(_) => Platform::Ios,
            Verifier::Android(_) => Platform::Android,
        }
    }

    pub(crate) async fn check_rate_limit(&self, ip: &str) -> Result<(), AttestationError> {
        let key = app_attestation_rate_limit_key(&URL_SAFE_NO_PAD.encode(&self.origin), ip);
        if incr_fixed_window(&mut self.redis.clone(), &key, 60).await? > 60 {
            return Err(AttestationError::RateLimited);
        }
        Ok(())
    }

    pub(crate) async fn issue_challenge(
        &self,
        subject: &ChallengeSubject,
    ) -> Result<IssuedChallenge, AttestationError> {
        let id = Uuid::new_v4();
        let challenge: [u8; 32] = rand::random();
        let expires_at = Utc::now() + TimeDelta::seconds(CHALLENGE_TTL_SECONDS as i64);
        self.redis
            .clone()
            .set_ex::<_, _, ()>(
                self.challenge_key(subject, id),
                challenge.as_slice(),
                CHALLENGE_TTL_SECONDS,
            )
            .await?;
        Ok(IssuedChallenge {
            challenge_id: id,
            challenge: URL_SAFE_NO_PAD.encode(challenge),
            expires_at,
        })
    }

    pub(crate) async fn find_credential(
        &self,
        bearer: &[u8; 32],
    ) -> Result<app_credential::Model, AttestationError> {
        let credential = self
            .repository
            .find_by_bearer(
                self.platform().as_str(),
                &self.scope_hash,
                &Sha256::digest(bearer).into(),
            )
            .await?
            .ok_or(AttestationError::InvalidToken)?;
        if !self.repository.is_valid(credential.authed_token_id).await? {
            return Err(AttestationError::TokenRevoked);
        }
        Ok(credential)
    }

    pub(crate) async fn register(
        &self,
        proof: RegistrationProof,
        client: RegistrationClient<'_>,
        registration_closed: bool,
    ) -> Result<RegisteredApp, AttestationError> {
        let platform = self.platform().as_str();
        let attempt_hash = Sha256::digest(proof.registration_key).into();
        let request_hash = Sha256::new().chain_update(proof.challenge_id.as_bytes());
        let request_hash = match &proof.evidence {
            RegistrationEvidence::Ios {
                key_id,
                attestation,
            } => request_hash.chain_update(key_id).chain_update(attestation),
            RegistrationEvidence::Android(token) => request_hash.chain_update(token),
        }
        .finalize();
        if let Some(credential) = self
            .repository
            .find_by_attempt(platform, &self.scope_hash, &attempt_hash)
            .await?
        {
            return self
                .recover_registration(credential, &proof.registration_key, &request_hash)
                .await;
        }
        if registration_closed {
            return Err(AttestationError::RegistrationClosed);
        }

        let subject = ChallengeSubject::Registration(attempt_hash);
        let client_data_hash = self.consume_challenge(&subject, proof.challenge_id).await?;
        let id = Uuid::now_v7();
        let ios_key = match (&self.verifier, proof.evidence) {
            (
                Verifier::Ios(verifier),
                RegistrationEvidence::Ios {
                    key_id,
                    attestation,
                },
            ) => {
                let public_key = verifier.verify_attestation(
                    &attestation,
                    &key_id,
                    &client_data_hash,
                    UnixTime::now(),
                )?;
                Some(app_attest_key::Model {
                    authed_token_id: id,
                    key_id: key_id.to_vec(),
                    public_key: public_key.to_vec(),
                    counter: 0,
                })
            }
            (Verifier::Android(verifier), RegistrationEvidence::Android(token)) => {
                verifier
                    .verify(&token, &client_data_hash, Some(&mut self.redis.clone()))
                    .await?;
                None
            }
            _ => return Err(AttestationError::InvalidRequest),
        };
        let token = self.derive_bearer(id, &proof.registration_key);
        let now = utc_now_millis();
        let reattest_after = now + REATTEST_INTERVAL;
        let credential = app_credential::Model {
            authed_token_id: id,
            platform: platform.to_owned(),
            scope_hash: self.scope_hash.to_vec(),
            bearer_hash: Sha256::digest(token).to_vec(),
            registration_key_hash: attempt_hash.to_vec(),
            registration_request_hash: request_hash.to_vec(),
            registered_at: now,
            verified_at: now,
            reattest_after,
        };
        if self.repository.issue(credential, ios_key, client).await? {
            return Ok(RegisteredApp {
                token: URL_SAFE_NO_PAD.encode(token),
                reattest_after,
            });
        }
        let existing = self
            .repository
            .find_by_attempt(platform, &self.scope_hash, &attempt_hash)
            .await?
            .ok_or(AttestationError::RegistrationConflict)?;
        self.recover_registration(existing, &proof.registration_key, &request_hash)
            .await
    }

    async fn recover_registration(
        &self,
        credential: app_credential::Model,
        key: &[u8; 32],
        request_hash: &[u8],
    ) -> Result<RegisteredApp, AttestationError> {
        let token = self.derive_bearer(credential.authed_token_id, key);
        if Utc::now() >= credential.registered_at + REGISTRATION_RECOVERY_WINDOW
            || credential.registration_request_hash != request_hash
            || credential.bearer_hash.as_slice() != &Sha256::digest(token)[..]
        {
            return Err(AttestationError::RegistrationConflict);
        }
        if !self.repository.is_valid(credential.authed_token_id).await? {
            return Err(AttestationError::TokenRevoked);
        }
        Ok(RegisteredApp {
            token: URL_SAFE_NO_PAD.encode(token),
            reattest_after: credential.reattest_after,
        })
    }

    pub(crate) async fn refresh(
        &self,
        credential: app_credential::Model,
        challenge_id: Uuid,
        evidence: RefreshEvidence,
    ) -> Result<DateTime<Utc>, AttestationError> {
        let id = credential.authed_token_id;
        let client_data_hash = self
            .consume_challenge(&ChallengeSubject::Refresh(id), challenge_id)
            .await?;
        let counter = match (&self.verifier, evidence) {
            (Verifier::Ios(verifier), RefreshEvidence::Ios(assertion)) => {
                let key = self
                    .repository
                    .find_app_attest_key(id)
                    .await?
                    .ok_or(AttestationError::InvalidToken)?;
                Some(verifier.verify_assertion(
                    &assertion,
                    &key.public_key,
                    &client_data_hash,
                    key.counter,
                )?)
            }
            (Verifier::Android(verifier), RefreshEvidence::Android(token)) => {
                verifier.verify(&token, &client_data_hash, None).await?;
                None
            }
            _ => return Err(AttestationError::InvalidRequest),
        };
        let now = utc_now_millis();
        match self
            .repository
            .refresh(id, counter, now, now + REATTEST_INTERVAL)
            .await?
        {
            RefreshResult::Updated(deadline) => Ok(deadline),
            RefreshResult::Revoked => Err(AttestationError::TokenRevoked),
            RefreshResult::Rejected => Err(AttestationError::Rejected),
        }
    }

    fn challenge_key(&self, subject: &ChallengeSubject, id: Uuid) -> String {
        app_attestation_challenge_key(
            &URL_SAFE_NO_PAD.encode(self.scope_hash),
            &subject.key_part(),
            &id.to_string(),
        )
    }

    async fn consume_challenge(
        &self,
        subject: &ChallengeSubject,
        id: Uuid,
    ) -> Result<[u8; 32], AttestationError> {
        let challenge: Option<Vec<u8>> = self
            .redis
            .clone()
            .get_del(self.challenge_key(subject, id))
            .await?;
        let challenge = challenge
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .ok_or(AttestationError::InvalidChallenge)?;
        Ok(client_data_hash(
            &self.origin,
            subject.purpose(),
            &challenge,
        ))
    }

    fn derive_bearer(&self, id: Uuid, registration_key: &[u8; 32]) -> [u8; 32] {
        Hmac::<Sha256>::new_from_slice(&self.token_key)
            .expect("HMAC accepts a 32-byte key")
            .chain_update(b"eddist-app-attestation-bearer-v1\0")
            .chain_update(self.scope_hash)
            .chain_update(id.as_bytes())
            .chain_update(registration_key)
            .finalize()
            .into_bytes()
            .into()
    }
}

fn client_data_hash(origin: &str, purpose: &str, challenge: &[u8; 32]) -> [u8; 32] {
    Sha256::new()
        .chain_update(b"eddist-app-attestation-v1\n")
        .chain_update(origin)
        .chain_update(b"\n")
        .chain_update(purpose)
        .chain_update(b"\n")
        .chain_update(challenge)
        .finalize()
        .into()
}

fn utc_now_millis() -> DateTime<Utc> {
    truncate_to_millis(Utc::now().naive_utc()).and_utc()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_the_v1_hash_to_origin_and_purpose() {
        let challenge = std::array::from_fn(|index| index as u8);
        let registered = client_data_hash("https://bbs.example", "register", &challenge);
        assert_eq!(
            URL_SAFE_NO_PAD.encode(registered),
            "QimepKmUBf1dRfGX6X8KWvKQbLwOjv9z09WI2XT66nM"
        );
        assert_ne!(
            registered,
            client_data_hash("https://other.example", "register", &challenge)
        );
        assert_ne!(
            registered,
            client_data_hash("https://bbs.example", "refresh", &challenge)
        );
    }
}
