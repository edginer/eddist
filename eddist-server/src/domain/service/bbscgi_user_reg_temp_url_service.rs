use rand::{RngExt, distr::Uniform};

use crate::{
    domain::{
        authed_token::AuthedToken,
        user::user_reg_state::{RegistrationSource, TempUrlRegistrationRecord},
    },
    stores::auth_state_store::AuthStateStore,
};

pub const USER_REG_TEMP_URL_LEN: usize = 5;

pub async fn create_userreg_temp_url(
    auth_state: &dyn AuthStateStore,
    base_url: &str,
    authed_token: &AuthedToken,
) -> anyhow::Result<UserRegUrlKind> {
    if authed_token.registered_user_id.is_some() {
        return Ok(UserRegUrlKind::Registered);
    }

    let temp_url_path = auth_state
        .issue_user_reg_temp_url(&TempUrlRegistrationRecord {
            authed_token_id: authed_token.id.to_string(),
            source: RegistrationSource::BbsCgi,
        })
        .await?;

    Ok(UserRegUrlKind::NotRegistered(format!(
        "{base_url}/user/register/{temp_url_path}"
    )))
}

#[derive(Debug)]
pub enum UserRegUrlKind {
    Registered,
    NotRegistered(String),
}

pub(crate) fn generate_random_string(len: usize) -> String {
    // NOTE: We have removed 'I', 'i', 'L', 'l', 'O', 'o', '0', '1' from the usual alphanumeric set.
    let charset: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz";

    let index_dist = Uniform::try_from(0..charset.len()).unwrap();

    (0..len)
        .map(|_| {
            let idx = rand::rng().sample(index_dist);
            charset[idx] as char
        })
        .collect()
}
