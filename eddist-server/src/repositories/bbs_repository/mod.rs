mod authed_token;
mod board;
#[cfg(feature = "backend-postgres")]
mod legacy;
mod response;
mod thread;

pub use authed_token::{AuthedTokenRepository, CreatingAuthedToken};
pub use board::BoardRepository;
pub use eddist_core::domain::pubsub_repository::CreatingThread;
#[cfg(feature = "backend-postgres")]
pub use legacy::BbsRepositoryPgImpl;
pub use response::ResponseRepository;
pub use thread::{ThreadRepository, ThreadStatus};

use sqlx::MySqlPool;

#[async_trait::async_trait]
pub trait BbsRepository:
    BoardRepository + ThreadRepository + ResponseRepository + AuthedTokenRepository
{
}

#[derive(Debug, Clone)]
pub struct BbsRepositoryImpl {
    pub(super) pool: MySqlPool,
}

impl BbsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> BbsRepositoryImpl {
        BbsRepositoryImpl { pool }
    }
}

impl BbsRepository for BbsRepositoryImpl {}

// The PostgreSQL implementation was originally written against the monolithic
// BbsRepository trait. Keep it in a private compatibility module and adapt it to
// the focused traits used by the current main branch.
#[cfg(feature = "backend-postgres")]
use legacy::{
    BbsRepository as LegacyBbsRepository, CreatingAuthedToken as LegacyCreatingAuthedToken,
    CreatingThread as LegacyCreatingThread, ThreadStatus as LegacyThreadStatus,
};

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl BoardRepository for BbsRepositoryPgImpl {
    async fn get_boards(&self) -> anyhow::Result<Vec<eddist_core::domain::board::Board>> {
        LegacyBbsRepository::get_boards(self).await
    }

    async fn get_board(
        &self,
        board_key: &str,
    ) -> anyhow::Result<Option<eddist_core::domain::board::Board>> {
        LegacyBbsRepository::get_board(self, board_key).await
    }

    async fn get_board_info(
        &self,
        board_id: uuid::Uuid,
    ) -> anyhow::Result<Option<eddist_core::domain::board::BoardInfo>> {
        LegacyBbsRepository::get_board_info(self, board_id).await
    }

    async fn get_ng_words_by_board_key(
        &self,
        board_key: &str,
    ) -> anyhow::Result<Vec<crate::domain::ng_word::NgWord>> {
        LegacyBbsRepository::get_ng_words_by_board_key(self, board_key).await
    }

    async fn get_cap_by_board_key(
        &self,
        cap_hash: &str,
        board_key: &str,
    ) -> anyhow::Result<Option<eddist_core::domain::cap::Cap>> {
        LegacyBbsRepository::get_cap_by_board_key(self, cap_hash, board_key).await
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl ThreadRepository for BbsRepositoryPgImpl {
    async fn get_threads(
        &self,
        board_id: uuid::Uuid,
        status: ThreadStatus,
    ) -> anyhow::Result<Vec<crate::domain::thread::Thread>> {
        let status = match status {
            ThreadStatus::Active => LegacyThreadStatus::Active,
            ThreadStatus::Archived => LegacyThreadStatus::Archived,
            ThreadStatus::Inactive => LegacyThreadStatus::Inactive,
            ThreadStatus::Unarchived => LegacyThreadStatus::Unarchived,
        };
        LegacyBbsRepository::get_threads(self, board_id, status).await
    }

    async fn get_threads_with_metadent(
        &self,
        board_id: uuid::Uuid,
    ) -> anyhow::Result<
        Vec<(
            crate::domain::thread::Thread,
            eddist_core::domain::client_info::ClientInfo,
            crate::domain::authed_token::AuthedToken,
        )>,
    > {
        LegacyBbsRepository::get_threads_with_metadent(self, board_id).await
    }

    async fn get_thread_by_board_key_and_thread_number(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Option<crate::domain::thread::Thread>> {
        LegacyBbsRepository::get_thread_by_board_key_and_thread_number(
            self,
            board_key,
            thread_number,
        )
        .await
    }

    async fn create_thread(&self, thread: CreatingThread) -> anyhow::Result<()> {
        let thread = LegacyCreatingThread {
            thread_id: thread.thread_id,
            response_id: thread.response_id,
            title: thread.title,
            unix_time: thread.unix_time,
            body: thread.body,
            name: thread.name,
            mail: thread.mail,
            created_at: thread.created_at,
            author_ch5id: thread.author_ch5id,
            authed_token_id: thread.authed_token_id,
            ip_addr: thread.ip_addr,
            board_id: thread.board_id,
            metadent: thread.metadent,
            client_info: thread.client_info,
        };
        LegacyBbsRepository::create_thread(self, thread).await
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl ResponseRepository for BbsRepositoryPgImpl {
    async fn get_responses(
        &self,
        thread_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<eddist_core::domain::res::ResView>> {
        let mut responses = LegacyBbsRepository::get_responses(self, thread_id).await?;
        for response in &mut responses {
            response.is_abone_keep_id = false;
        }
        Ok(responses)
    }

    async fn create_response(
        &self,
        response: eddist_core::domain::pubsub_repository::CreatingRes,
    ) -> anyhow::Result<()> {
        LegacyBbsRepository::create_response(self, response).await
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl AuthedTokenRepository for BbsRepositoryPgImpl {
    async fn get_authed_token(
        &self,
        token: &str,
    ) -> anyhow::Result<Option<crate::domain::authed_token::AuthedToken>> {
        LegacyBbsRepository::get_authed_token(self, token).await
    }

    async fn get_authed_token_by_id(
        &self,
        id: uuid::Uuid,
    ) -> anyhow::Result<Option<crate::domain::authed_token::AuthedToken>> {
        LegacyBbsRepository::get_authed_token_by_id(self, id).await
    }

    async fn get_authed_token_by_origin_ip_and_auth_code(
        &self,
        ip: &str,
        auth_code: &str,
    ) -> anyhow::Result<Option<crate::domain::authed_token::AuthedToken>> {
        LegacyBbsRepository::get_authed_token_by_origin_ip_and_auth_code(self, ip, auth_code).await
    }

    async fn get_unauthed_authed_token_by_auth_code(
        &self,
        auth_code: &str,
    ) -> anyhow::Result<Vec<crate::domain::authed_token::AuthedToken>> {
        LegacyBbsRepository::get_unauthed_authed_token_by_auth_code(self, auth_code).await
    }

    async fn create_authed_token(&self, token: CreatingAuthedToken) -> anyhow::Result<()> {
        let token = LegacyCreatingAuthedToken {
            id: token.id,
            token: token.token,
            origin_ip: token.origin_ip,
            asn_num: token.asn_num,
            writing_ua: token.writing_ua,
            auth_code: token.auth_code,
            created_at: token.created_at,
            author_id_seed: token.author_id_seed,
            require_user_registration: token.require_user_registration,
        };
        LegacyBbsRepository::create_authed_token(self, token).await
    }

    async fn activate_authed_status(
        &self,
        token: &str,
        authed_ua: &str,
        authed_time: chrono::DateTime<chrono::Utc>,
        additional_info: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        LegacyBbsRepository::activate_authed_status(
            self,
            token,
            authed_ua,
            authed_time,
            additional_info,
        )
        .await
    }

    async fn update_authed_token_last_wrote(
        &self,
        token_id: uuid::Uuid,
        last_wrote: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<()> {
        LegacyBbsRepository::update_authed_token_last_wrote(self, token_id, last_wrote).await
    }

    async fn revoke_authed_token(&self, token: &str) -> anyhow::Result<()> {
        LegacyBbsRepository::revoke_authed_token(self, token).await
    }

    async fn delete_authed_token(&self, token: &str) -> anyhow::Result<()> {
        LegacyBbsRepository::delete_authed_token(self, token).await
    }

    async fn clear_require_reauth(&self, id: uuid::Uuid) -> anyhow::Result<()> {
        LegacyBbsRepository::clear_require_reauth(self, id).await
    }
}

#[cfg(feature = "backend-postgres")]
impl BbsRepository for BbsRepositoryPgImpl {}
