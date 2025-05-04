use rand::{Rng, distributions::Uniform, rngs::OsRng};
use redis::{AsyncCommands, aio::ConnectionManager};

use crate::{domain::authed_token::AuthedToken, utils::redis::user_reg_temp_url_register_key};

pub const USER_REG_TEMP_URL_LEN: usize = 5;

#[derive(Clone)]
pub struct UserRegTempUrlService {
    redis_conn: ConnectionManager,
}

impl UserRegTempUrlService {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self { redis_conn }
    }

    pub async fn create_userreg_temp_url(
        &self,
        authed_token: &AuthedToken,
    ) -> anyhow::Result<UserRegUrlKind> {
        let mut redis_conn = self.redis_conn.clone();

        if authed_token.registered_user_id.is_some() {
            return Ok(UserRegUrlKind::Registered);
        }

        let temp_url_query = generate_random_string(USER_REG_TEMP_URL_LEN);
        // first, duplicate check
        let temp_url_path = if redis_conn
            .exists::<_, bool>(user_reg_temp_url_register_key(&temp_url_query))
            .await?
        {
            // NOTE: retry only once (we does not consider collision between `exists` and `set`, it's very rare case)
            let temp_url_query = generate_random_string(USER_REG_TEMP_URL_LEN);
            if redis_conn
                .exists::<_, bool>(user_reg_temp_url_register_key(&temp_url_query))
                .await?
            {
                return Err(anyhow::anyhow!("Failed to generate temp_url_query"));
            }
            temp_url_query
        } else {
            temp_url_query
        };

        redis_conn
            .set_ex::<_, _, ()>(
                user_reg_temp_url_register_key(&temp_url_path),
                authed_token.id.to_string().clone(),
                60 * 3,
            )
            .await?;

        Ok(UserRegUrlKind::NotRegistered(format!(
            "{}/user/register/{temp_url_path}",
            std::env::var("BASE_URL").unwrap(),
        )))
    }
}

#[derive(Debug)]
pub enum UserRegUrlKind {
    Registered,
    NotRegistered(String),
}

fn generate_random_string(len: usize) -> String {
    // NOTE: We have removed 'I', 'i', 'L', 'l', 'O', 'o', '0', '1' from the usual alphanumeric set.
    let charset: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz";

    let mut rng = OsRng;

    let index_dist = Uniform::from(0..charset.len());

    (0..len)
        .map(|_| {
            let idx = rng.sample(index_dist);
            charset[idx] as char
        })
        .collect()
}
