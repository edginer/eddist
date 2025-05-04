use md5::Digest;
use openidconnect::{AuthorizationCode, Nonce, PkceCodeVerifier};
use rand::{distributions::Alphanumeric, Rng};
use redis::{aio::ConnectionManager, AsyncCommands};
use sqlx::MySql;
use uuid::Uuid;

use crate::{
    domain::{
        service::oidc_client_service::OidcClientService,
        user::{user_login_state::UserLoginState, user_reg_state::UserRegState},
    },
    repositories::{
        idp_repository::IdpRepository,
        user_repository::{CreatingUser, UserRepository},
    },
    utils::{
        redis::{user_login_oauth2_authreq_key, user_reg_oauth2_authreq_key, user_session_key},
        TransactionRepository,
    },
};

use super::AppService;

#[derive(Clone)]
pub struct UserAuthzIdpCallbackService<I: IdpRepository, U: UserRepository> {
    idp_repo: I,
    user_repo: U,
    redis_conn: ConnectionManager,
}

impl<I: IdpRepository + Clone, U: UserRepository + Clone> UserAuthzIdpCallbackService<I, U> {
    pub fn new(idp_repo: I, user_repo: U, redis_conn: ConnectionManager) -> Self {
        Self {
            idp_repo,
            user_repo,
            redis_conn,
        }
    }
}

#[async_trait::async_trait]
impl<I: IdpRepository + Clone, U: UserRepository + TransactionRepository<MySql> + Clone>
    AppService<UserAuthzIdpCallbackServiceInput, UserAuthzIdpCallbackServiceOutput>
    for UserAuthzIdpCallbackService<I, U>
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

        let user_sid = match input.callback_kind {
            CallbackKind::Register => {
                let reg_state = serde_json::from_str::<UserRegState>(&user_state)?;
                self.register_user_with_idp(reg_state, input.code).await?
            }
            CallbackKind::Login => {
                let login_state = serde_json::from_str::<UserLoginState>(&user_state)?;
                self.login_user_with_idp(login_state, input.code).await?
            }
        };

        Ok(UserAuthzIdpCallbackServiceOutput { user_sid })
    }
}

impl<I: IdpRepository + Clone, U: UserRepository + TransactionRepository<MySql> + Clone>
    UserAuthzIdpCallbackService<I, U>
{
    async fn register_user_with_idp(
        &self,
        user_reg_state: UserRegState,
        code: String,
    ) -> anyhow::Result<String> {
        let mut redis_conn = self.redis_conn.clone();

        let idp_clients_svc =
            OidcClientService::new(self.idp_repo.clone(), self.redis_conn.clone());
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

        let user_id = if let Some(u) = self
            .user_repo
            .get_user_by_idp_sub(&idp.idp_name, &sub)
            .await?
        {
            // Already user is registered
            let tx = self.user_repo.begin().await?;
            let tx = self
                .user_repo
                .bind_user_authed_token(u.id, Uuid::parse_str(&user_reg_state.authed_token)?, tx)
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
                .bind_user_authed_token(user_id, Uuid::parse_str(&user_reg_state.authed_token)?, tx)
                .await?;
            tx.commit().await?;

            user_id
        };

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

        Ok(user_sid)
    }

    async fn login_user_with_idp(
        &self,
        user_login_state: UserLoginState,
        code: String,
    ) -> anyhow::Result<String> {
        let mut redis_conn = self.redis_conn.clone();

        let idp_clients_svc =
            OidcClientService::new(self.idp_repo.clone(), self.redis_conn.clone());
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

                Ok(user_sid)
            }
            None => Err(anyhow::anyhow!("user not found")),
        }
    }
}

pub struct UserAuthzIdpCallbackServiceInput {
    pub code: String,
    pub state_id: String,
    pub callback_kind: CallbackKind,
}

pub enum CallbackKind {
    Register,
    Login,
}

pub struct UserAuthzIdpCallbackServiceOutput {
    pub user_sid: String,
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

    let mut rng = rand::thread_rng();

    let (left_index, right_family_index, right_first_index) = (
        rng.gen_range(0..left.len()),
        rng.gen_range(0..right.len()),
        rng.gen_range(0..right.len()),
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
