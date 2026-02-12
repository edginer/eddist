use axum::Router;

use crate::DefaultAppState;

pub mod archives;
pub mod auth_tokens;
pub mod boards;
pub mod captcha;
pub mod moderation;
pub mod notices;
pub mod terms;
pub mod threads;
pub mod users;

pub fn create_api_routes() -> Router<DefaultAppState> {
    Router::new()
        .merge(boards::routes())
        .merge(threads::routes())
        .merge(archives::routes())
        .merge(auth_tokens::routes())
        .merge(captcha::routes())
        .merge(moderation::routes())
        .merge(notices::routes())
        .merge(terms::routes())
        .merge(users::routes())
}
