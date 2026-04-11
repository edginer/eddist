use chrono::Utc;
use md5::Digest;
use openidconnect::{AuthorizationCode, Nonce, PkceCodeVerifier};
use rand::{RngExt, distr::Alphanumeric};
use redis::{AsyncCommands, aio::ConnectionManager};
use uuid::Uuid;

use crate::{
    domain::{
        authed_token::AuthedToken,
        service::oidc_client_service::OidcClientService,
        user::{
            user_login_state::UserLoginState,
            user_reg_state::{RegistrationSource, UserRegState},
        },
    },
    repositories::{
        Db, bbs_repository::{BbsRepository, CreatingAuthedToken},
        idp_repository::IdpRepository,
        user_repository::{CreatingUser, UserRepository},
    },
    utils::TransactionRepository,
};
use eddist_core::redis_keys::{
    user_login_oauth2_authreq_key, user_reg_oauth2_authreq_key, user_session_key,
};

use super::AppService;

#[derive(Clone)]
pub struct UserAuthzIdpCallbackService<I: IdpRepository, U: UserRepository, B: BbsRepository> {
    idp_repo: I,
    user_repo: U,
    bbs_repo: B,
    redis_conn: ConnectionManager,
}

impl<I: IdpRepository + Clone, U: UserRepository + Clone, B: BbsRepository + Clone>
    UserAuthzIdpCallbackService<I, U, B>
{
    pub fn new(idp_repo: I, user_repo: U, bbs_repo: B, redis_conn: ConnectionManager) -> Self {
        Self {
            idp_repo,
            user_repo,
            bbs_repo,
            redis_conn,
        }
    }
}

#[async_trait::async_trait]
impl<
    I: IdpRepository + Clone,
    U: UserRepository + TransactionRepository<Db> + Clone,
    B: BbsRepository + Clone,
> AppService<UserAuthzIdpCallbackServiceInput, UserAuthzIdpCallbackServiceOutput>
    for UserAuthzIdpCallbackService<I, U, B>
{
    async fn execute(
        &self,
        input: UserAuthzIdpCallbackServiceInput,
    ) -> anyhow::Result<UserAuthzIdpCallbackServiceOutput> {
        let mut redis_conn = self.redis_conn.clone();

        let redis_authreq_key = match input.callback_kind {
            CallbackKind::Register => user_reg_oauth2_authreq_key(&input.state_id),
            CallbackKind::Login => user_login_oauth2_authreq_key(&input.state_id),
        };

        // TODO: currently, get_del does not work well
        let user_state = redis_conn.get::<_, String>(&redis_authreq_key).await?;
        redis_conn.del::<_, ()>(&redis_authreq_key).await?;

        let (user_sid, edge_token) = match input.callback_kind {
            CallbackKind::Register => {
                let reg_state = serde_json::from_str::<UserRegState>(&user_state)?;
                let (user_sid, edge_token) = self
                    .register_user_with_idp(reg_state, input.code, input.browser_edge_token)
                    .await?;
                (user_sid, edge_token)
            }
            CallbackKind::Login => {
                let login_state = serde_json::from_str::<UserLoginState>(&user_state)?;
                let (user_sid, edge_token) = self
                    .login_user_with_idp(
                        login_state,
                        input.code,
                        input.browser_edge_token,
                        input.ip_addr,
                        input.user_agent,
                        input.asn_num,
                    )
                    .await?;
                (user_sid, edge_token)
            }
        };

        Ok(UserAuthzIdpCallbackServiceOutput {
            user_sid,
            edge_token,
        })
    }
}

impl<
    I: IdpRepository + Clone,
    U: UserRepository + TransactionRepository<Db> + Clone,
    B: BbsRepository + Clone,
> UserAuthzIdpCallbackService<I, U, B>
{
    async fn register_user_with_idp(
        &self,
        user_reg_state: UserRegState,
        code: String,
        browser_edge_token: Option<String>,
    ) -> anyhow::Result<(String, Option<String>)> {
        let mut redis_conn = self.redis_conn.clone();

        let edge_token = user_reg_state.edge_token.clone();
        let authed_token_id = user_reg_state.authed_token.clone();

        // For AuthCode-initiated registrations the callback must arrive in the same browser.
        // BbsCgi registrations may legitimately hand off to a system browser (different cookie jar).
        if matches!(user_reg_state.source, RegistrationSource::AuthCode)
            && browser_edge_token.as_deref() != edge_token.as_deref()
        {
            return Err(anyhow::anyhow!(
                "edge-token mismatch: registration was initiated from auth-code page \
                 but callback arrived with a different browser token"
            ));
        }

        let idp_clients_svc = OidcClientService::new(self.idp_repo.clone());
        let idp_clients = idp_clients_svc.get_idp_clients().await?;

        let (idp, idp_client) = idp_clients
            .get(&user_reg_state.idp_name.clone().unwrap())
            .ok_or_else(|| {
                anyhow::anyhow!("idp client not found: {}", user_reg_state.idp_name.unwrap())
            })?;

        let id_token_claims = idp_client
            .exchange_code(
                AuthorizationCode::new(code),
                PkceCodeVerifier::new(user_reg_state.code_verifier.unwrap()),
                Nonce::new(user_reg_state.nonce.unwrap()),
            )
            .await;

        let sub = id_token_claims.subject().to_string();
        let authed_token_uuid = Uuid::parse_str(&authed_token_id)?;

        let user_id = if let Some(u) = self
            .user_repo
            .get_user_by_idp_sub(&idp.idp_name, &sub)
            .await?
        {
            // Already user is registered
            let tx = self.user_repo.begin().await?;
            let tx = self
                .user_repo
                .bind_user_authed_token(u.id, authed_token_uuid, tx)
                .await?;
            tx.commit().await?;

            if !u.enabled {
                return Err(anyhow::anyhow!("user is disabled"));
            }

            u.id
        } else {
            let user_id = Uuid::now_v7();

            let tx = self.user_repo.begin().await?;
            let tx = self
                .user_repo
                .create_user_with_idp(
                    CreatingUser {
                        user_id,
                        user_name: user_name_generator(),
                        idp_id: idp.id,
                        idp_sub: sub,
                    },
                    tx,
                )
                .await?;

            let tx = self
                .user_repo
                .bind_user_authed_token(user_id, authed_token_uuid, tx)
                .await?;
            tx.commit().await?;

            user_id
        };

        // Also bind the browser's current edge-token if it differs from the reg-state token
        if let Some(browser_token) = browser_edge_token
            && edge_token.as_deref() != Some(&browser_token)
            && let Ok(Some(token)) = self.bbs_repo.get_authed_token(&browser_token).await
            && token.validity
            && token.registered_user_id.is_none()
        {
            let tx = self.user_repo.begin().await?;
            let tx = self
                .user_repo
                .bind_user_authed_token(user_id, token.id, tx)
                .await?;
            tx.commit().await?;
        }

        let mut hasher = sha3::Sha3_512::new();
        hasher.update(Uuid::now_v7().to_string());
        let user_sid = format!("{:x}", hasher.finalize());

        redis_conn
            .set_ex::<_, _, ()>(
                user_session_key(&user_sid),
                user_id.to_string(),
                60 * 60 * 24 * 365,
            )
            .await?;

        Ok((user_sid, edge_token))
    }

    async fn login_user_with_idp(
        &self,
        user_login_state: UserLoginState,
        code: String,
        browser_edge_token: Option<String>,
        ip_addr: String,
        user_agent: String,
        asn_num: i32,
    ) -> anyhow::Result<(String, Option<String>)> {
        let mut redis_conn = self.redis_conn.clone();

        let idp_clients_svc = OidcClientService::new(self.idp_repo.clone());
        let idp_clients = idp_clients_svc.get_idp_clients().await?;

        let (idp, idp_client) = idp_clients
            .get(&user_login_state.idp_name.clone())
            .ok_or_else(|| {
                anyhow::anyhow!("idp client not found: {}", user_login_state.idp_name)
            })?;

        let id_token_claims = idp_client
            .exchange_code(
                AuthorizationCode::new(code),
                PkceCodeVerifier::new(user_login_state.code_verifier),
                Nonce::new(user_login_state.nonce),
            )
            .await;

        let user = self
            .user_repo
            .get_user_by_idp_sub(&idp.idp_name, &id_token_claims.subject().to_string())
            .await?;

        match user {
            Some(user) if !user.enabled => Err(anyhow::anyhow!("user is disabled")),
            Some(user) => {
                let mut hasher = sha3::Sha3_512::new();
                hasher.update(Uuid::now_v7().to_string());
                let user_sid = format!("{:x}", hasher.finalize());

                redis_conn
                    .set_ex::<_, _, ()>(
                        user_session_key(&user_sid),
                        user.id.to_string(),
                        60 * 60 * 24 * 365,
                    )
                    .await?;

                // Sweep browser token: try to bind the current browser's token if valid and
                // unbound. Otherwise issue a fresh token for this browser session — the user's
                // existing bound tokens likely belong to other browser sessions and should not
                // be shared across them.
                let edge_token = if let Some(browser_token) = browser_edge_token
                    && let Ok(Some(token)) = self.bbs_repo.get_authed_token(&browser_token).await
                    && token.validity
                    && token.registered_user_id.is_none()
                {
                    let tx = self.user_repo.begin().await?;
                    let tx = self
                        .user_repo
                        .bind_user_authed_token(user.id, token.id, tx)
                        .await?;
                    tx.commit().await?;
                    Some(browser_token)
                } else {
                    // No valid unbound browser token — create and activate a new one.
                    // IDP authentication already serves as verification so the token is
                    // immediately valid.
                    let new_token = AuthedToken::new(ip_addr, user_agent.clone(), asn_num);
                    let created_at = Utc::now();
                    self.bbs_repo
                        .create_authed_token(CreatingAuthedToken {
                            id: new_token.id,
                            token: new_token.token.clone(),
                            origin_ip: new_token.origin_ip,
                            asn_num: new_token.asn_num,
                            writing_ua: new_token.writing_ua.clone(),
                            auth_code: "000000".to_string(), // Use psuedo auth code since the token is valid immediately
                            created_at,
                            author_id_seed: new_token.author_id_seed,
                            require_user_registration: false,
                        })
                        .await?;
                    self.bbs_repo
                        .activate_authed_status(&new_token.token, &user_agent, created_at, None)
                        .await?;
                    let tx = self.user_repo.begin().await?;
                    let tx = self
                        .user_repo
                        .bind_user_authed_token(user.id, new_token.id, tx)
                        .await?;
                    tx.commit().await?;
                    Some(new_token.token)
                };

                Ok((user_sid, edge_token))
            }
            None => Err(anyhow::anyhow!("user not found")),
        }
    }
}

pub struct UserAuthzIdpCallbackServiceInput {
    pub code: String,
    pub state_id: String,
    pub callback_kind: CallbackKind,
    pub browser_edge_token: Option<String>,
    pub ip_addr: String,
    pub user_agent: String,
    pub asn_num: i32,
}

pub enum CallbackKind {
    Register,
    Login,
}

pub struct UserAuthzIdpCallbackServiceOutput {
    pub user_sid: String,
    /// The edge-token to set for the user (used in Register flow)
    pub edge_token: Option<String>,
}

fn user_name_generator() -> String {
    // This code is from moby/moby/pkg/namesgenerator/names-generator.go
    // This code is licensed under the Apache License 2.0
    let left = [
        "admiring",
        "adoring",
        "affectionate",
        "agitated",
        "amazing",
        "angry",
        "awesome",
        "beautiful",
        "blissful",
        "bold",
        "boring",
        "brave",
        "busy",
        "charming",
        "clever",
        "compassionate",
        "competent",
        "condescending",
        "confident",
        "cool",
        "cranky",
        "crazy",
        "dazzling",
        "determined",
        "distracted",
        "dreamy",
        "eager",
        "ecstatic",
        "elastic",
        "elated",
        "elegant",
        "eloquent",
        "epic",
        "exciting",
        "fervent",
        "festive",
        "flamboyant",
        "focused",
        "friendly",
        "frosty",
        "funny",
        "gallant",
        "gifted",
        "goofy",
        "gracious",
        "great",
        "happy",
        "hardcore",
        "heuristic",
        "hopeful",
        "hungry",
        "infallible",
        "inspiring",
        "intelligent",
        "interesting",
        "jolly",
        "jovial",
        "keen",
        "kind",
        "laughing",
        "loving",
        "lucid",
        "magical",
        "modest",
        "musing",
        "mystifying",
        "naughty",
        "nervous",
        "nice",
        "nifty",
        "nostalgic",
        "objective",
        "optimistic",
        "peaceful",
        "pedantic",
        "pensive",
        "practical",
        "priceless",
        "quirky",
        "quizzical",
        "recursing",
        "relaxed",
        "reverent",
        "romantic",
        "sad",
        "serene",
        "sharp",
        "silly",
        "sleepy",
        "stoic",
        "strange",
        "stupefied",
        "suspicious",
        "sweet",
        "tender",
        "thirsty",
        "trusting",
        "unruffled",
        "upbeat",
        "vibrant",
        "vigilant",
        "vigorous",
        "wizardly",
        "wonderful",
        "xenodochial",
        "youthful",
        "zealous",
        "zen",
    ];

    // List of home appliance
    let right = [
        ("air", "conditioner"),
        ("washing", "machine"),
        ("microwave", "oven"),
        ("vacuum", "cleaner"),
        ("toaster", "oven"),
        ("coffee", "maker"),
        ("rice", "cooker"),
        ("bread", "maker"),
        ("water", "heater"),
        ("electric", "kettle"),
        ("food", "processor"),
        ("slow", "cooker"),
        ("pressure", "cooker"),
        ("electric", "stove"),
        ("induction", "cooker"),
        ("gas", "oven"),
        ("desk", "fan"),
        ("space", "heater"),
        ("window", "fan"),
        ("food", "steamer"),
        ("ice", "maker"),
        ("hair", "dryer"),
        ("water", "dispenser"),
        ("clothes", "dryer"),
        ("clothes", "washer"),
        ("garment", "steamer"),
        ("electric", "grill"),
        ("electric", "skillet"),
        ("deep", "fryer"),
        ("convection", "oven"),
        ("wine", "cooler"),
        ("air", "purifier"),
        ("espresso", "machine"),
        ("induction", "range"),
        ("mixer", "grinder"),
    ];

    let mut rng = rand::rng();

    let (left_index, right_family_index, right_first_index) = (
        rng.random_range(0..left.len()),
        rng.random_range(0..right.len()),
        rng.random_range(0..right.len()),
    );

    let random_str = rng
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect::<String>();

    let user_name = format!(
        "{}-{}-{}-{random_str}",
        left[left_index], right[right_family_index].0, right[right_first_index].1
    );

    user_name
}
