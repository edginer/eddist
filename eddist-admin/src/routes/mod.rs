use axum::Router;

use crate::AppState;

pub mod archives;
pub mod auth_tokens;
pub mod boards;
pub mod captcha;
pub mod idps;
pub mod internal;
pub mod moderation;
pub mod notices;
pub mod server_settings;
pub mod terms;
pub mod threads;
pub mod users;

pub fn create_internal_routes() -> Router<AppState> {
    // Keep the session and internal API route sets in sync.
    create_api_routes().merge(internal::create_internal_routes())
}

pub fn create_api_routes() -> Router<AppState> {
    Router::new()
        .merge(boards::routes())
        .merge(threads::routes())
        .merge(archives::routes())
        .merge(auth_tokens::routes())
        .merge(captcha::routes())
        .merge(idps::routes())
        .merge(moderation::routes())
        .merge(notices::routes())
        .merge(server_settings::routes())
        .merge(terms::routes())
        .merge(users::routes())
}

#[cfg(test)]
mod tests {
    #[test]
    fn internal_routes_merge_without_conflicts() {
        let _ = super::create_internal_routes();
    }
}
