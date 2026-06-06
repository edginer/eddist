use std::sync::Arc;

use eddist_core::{
    domain::pubsub_repository::{AuthTokenRevoked, CHANNEL_AUTH_TOKEN_REVOKED},
    proto::encode_auth_token_revoked,
    redis_keys::authed_token_suspended_key,
    utils::is_authed_token_backup_enabled,
};
use redis::AsyncCommands as _;
use uuid::Uuid;

use crate::{
    auth::AdminIdentity,
    models::{AuthedToken, DeleteAuthedTokenInput, ListAuthedTokensQuery, PaginatedAuthedTokens},
    repository::authed_token_repository::{AuthedTokenRepository, ListAuthedTokensParams},
};

const ALLOWED_SORT_COLUMNS: &[&str] = &["created_at", "authed_at", "last_wrote_at"];

#[async_trait::async_trait]
pub trait AuthedTokenService: Send + Sync {
    async fn list_authed_tokens(
        &self,
        query: ListAuthedTokensQuery,
    ) -> anyhow::Result<PaginatedAuthedTokens>;
    async fn get_authed_token(&self, id: Uuid) -> anyhow::Result<AuthedToken>;
    async fn delete_authed_token(
        &self,
        actor: &AdminIdentity,
        id: Uuid,
        options: DeleteAuthedTokenInput,
    ) -> anyhow::Result<()>;
    async fn set_require_reauth(&self, id: Uuid) -> anyhow::Result<()>;
    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()>;
    async fn suspend_authed_token(&self, id: Uuid, ttl_seconds: u64) -> anyhow::Result<()>;
    async fn revoke_authed_token(&self, actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()>;
}

pub struct AuthedTokenServiceImpl {
    repo: Arc<dyn AuthedTokenRepository>,
    redis_conn: redis::aio::ConnectionManager,
}

impl AuthedTokenServiceImpl {
    pub fn new(
        repo: Arc<dyn AuthedTokenRepository>,
        redis_conn: redis::aio::ConnectionManager,
    ) -> Self {
        Self { repo, redis_conn }
    }

    async fn publish_token_revoked(&self, id: Uuid) {
        let payload = encode_auth_token_revoked(&AuthTokenRevoked {
            authed_token_id: id,
        });
        let mut conn = self.redis_conn.clone();
        let _: Result<(), _> = conn.publish(CHANNEL_AUTH_TOKEN_REVOKED, payload).await;
    }
}

#[async_trait::async_trait]
impl AuthedTokenService for AuthedTokenServiceImpl {
    async fn list_authed_tokens(
        &self,
        query: ListAuthedTokensQuery,
    ) -> anyhow::Result<PaginatedAuthedTokens> {
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(50).clamp(1, 100);
        let offset = (page - 1) as u64 * per_page as u64;
        let sort_column = query
            .sort_by
            .as_deref()
            .filter(|s| ALLOWED_SORT_COLUMNS.contains(s))
            .unwrap_or("created_at");
        let sort_asc = query
            .sort_order
            .as_deref()
            .map(|s| s == "asc")
            .unwrap_or(false);

        let (items, total) = self
            .repo
            .list_authed_tokens(ListAuthedTokensParams {
                offset,
                limit: per_page,
                origin_ip: query.origin_ip.as_deref(),
                writing_ua: query.writing_ua.as_deref(),
                authed_ua: query.authed_ua.as_deref(),
                asn_num: query.asn_num,
                validity: query.validity,
                sort_column,
                sort_asc,
            })
            .await?;

        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;
        Ok(PaginatedAuthedTokens {
            items,
            total,
            page,
            per_page,
            total_pages,
        })
    }

    async fn get_authed_token(&self, id: Uuid) -> anyhow::Result<AuthedToken> {
        self.repo.get_authed_token(id).await
    }

    async fn delete_authed_token(
        &self,
        _actor: &AdminIdentity,
        id: Uuid,
        options: DeleteAuthedTokenInput,
    ) -> anyhow::Result<()> {
        let affected_ids = if !options.using_origin_ip {
            self.repo.delete_authed_token(id).await?;
            vec![id]
        } else {
            self.repo.delete_authed_token_by_origin_ip(id).await?
        };

        if is_authed_token_backup_enabled() {
            for affected_id in affected_ids {
                self.publish_token_revoked(affected_id).await;
            }
        }
        Ok(())
    }

    async fn set_require_reauth(&self, id: Uuid) -> anyhow::Result<()> {
        self.repo.set_require_reauth(id).await
    }

    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()> {
        self.repo.clear_require_reauth(id).await
    }

    async fn suspend_authed_token(&self, id: Uuid, ttl_seconds: u64) -> anyhow::Result<()> {
        let token = self.repo.get_authed_token(id).await?;
        if !token.validity {
            return Err(crate::error::ServiceError::BadRequest(
                "this token has already been permanently revoked".into(),
            )
            .into());
        }
        let key = authed_token_suspended_key(&id.to_string());
        let mut conn = self.redis_conn.clone();
        conn.set_ex::<_, _, ()>(&key, "1", ttl_seconds)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn revoke_authed_token(&self, _actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()> {
        self.repo.delete_authed_token(id).await?;
        if is_authed_token_backup_enabled() {
            self.publish_token_revoked(id).await;
        }
        Ok(())
    }
}
