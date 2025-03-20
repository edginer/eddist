use md5::Digest;
use openidconnect::{AuthorizationCode, Nonce, PkceCodeVerifier};
use rand::{distributions::Alphanumeric, Rng};
use redis::{aio::ConnectionManager, AsyncCommands};
use sqlx::MySql;
use uuid::Uuid;

use crate::{
    domain::{service::oidc_client_service::OidcClientService, user::user_reg_state::UserRegState},
    repositories::{
        idp_repository::IdpRepository,
        user_repository::{CreatingUser, UserRepository},
    },
    utils::TransactionRepository,
};

use super::AppService;

#[derive(Clone)]
pub struct UserRegAuthzIdpCallbackService<I: IdpRepository, U: UserRepository> {
    idp_repo: I,
    user_repo: U,
    redis_conn: ConnectionManager,
}

impl<I: IdpRepository + Clone, U: UserRepository + Clone> UserRegAuthzIdpCallbackService<I, U> {
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
    AppService<UserRegAuthzIdpCallbackServiceInput, UserRegAuthzIdpCallbackServiceOutput>
    for UserRegAuthzIdpCallbackService<I, U>
{
    async fn execute(
        &self,
        input: UserRegAuthzIdpCallbackServiceInput,
    ) -> anyhow::Result<UserRegAuthzIdpCallbackServiceOutput> {
        let mut redis_conn = self.redis_conn.clone();

        let user_reg_state = redis_conn
            .get_del::<_, String>(format!(
                "userreg:oauth2:authreq:{}",
                input.user_reg_state_id
            ))
            .await?;
        let user_reg_state = serde_json::from_str::<UserRegState>(&user_reg_state)?;

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
                AuthorizationCode::new(input.code),
                PkceCodeVerifier::new(user_reg_state.code_verifier.unwrap()),
                Nonce::new(user_reg_state.nonce.unwrap()),
            )
            .await;

        let user_id = Uuid::now_v7();

        let tx = self.user_repo.begin().await?;
        let tx = self
            .user_repo
            .create_user_with_idp(
                CreatingUser {
                    user_id,
                    user_name: user_name_generator(),
                    idp_id: idp.id,
                    idp_sub: id_token_claims.subject().to_string(),
                },
                tx,
            )
            .await?;

        let tx = self
            .user_repo
            .bind_user_authed_token(user_id, Uuid::parse_str(&user_reg_state.authed_token)?, tx)
            .await?;
        tx.commit().await?;

        let mut hasher = sha3::Sha3_512::new();
        hasher.update(Uuid::now_v7().to_string());
        let user_sid = format!("{:x}", hasher.finalize());

        redis_conn
            .set_ex::<_, _, ()>(
                format!("user:session:{user_sid}"),
                user_id.to_string(),
                60 * 60 * 24 * 365,
            )
            .await?;

        Ok(UserRegAuthzIdpCallbackServiceOutput { user_sid })
    }
}

pub struct UserRegAuthzIdpCallbackServiceInput {
    pub code: String,
    pub state: String,
    pub user_reg_state_id: String,
}

pub struct UserRegAuthzIdpCallbackServiceOutput {
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

    // List of japanese prime minister's family names after WWII (1945/9/2)
    let right = [
        ("shidehara", "kijuro"),
        ("yoshida", "shigeru"),
        ("katayama", "tetsu"),
        ("ashida", "hitoshi"),
        ("hatoyama", "ichiro"),
        ("ishibashi", "tanzan"),
        ("kishi", "nobusuke"),
        ("ikeda", "hayato"),
        ("sato", "eisaku"),
        ("tanaka", "kakuei"),
        ("miki", "takeo"),
        ("fukuda", "takeo"),
        ("ohira", "masayoshi"),
        ("suzuki", "zenko"),
        ("nakasone", "yasuhiro"),
        ("takeshita", "noboru"),
        ("uno", "sosuke"),
        ("kaifu", "toshiki"),
        ("miyazawa", "kiichi"),
        ("hosokawa", "morihiro"),
        ("hata", "tsutomu"),
        ("murayama", "tomiichi"),
        ("hashimoto", "ryutaro"),
        ("obuchi", "keizo"),
        ("mori", "yoshiro"),
        ("koizumi", "junichiro"),
        ("abe", "shinzo"),
        ("fukuda", "yasuo"),
        ("aso", "taro"),
        ("hatoyama", "yukio"),
        ("kan", "naoto"),
        ("noda", "yoshihiko"),
        ("suga", "yoshihide"),
        ("kishida", "fumio"),
        ("ishiba", "shigeru"),
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
