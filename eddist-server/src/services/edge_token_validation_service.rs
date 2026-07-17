use crate::{domain::authed_token::AuthedToken, repositories::bbs_repository::BbsRepository};

use super::AppService;

/// Resolves an `edge-token` cookie to its authed token, rejecting values that do
/// not exist or are no longer valid.
///
/// Read-only endpoints use this rather than `BbsCgiAuthService::check_validity`,
/// which is posting-specific: it mints a new token when the cookie is absent and
/// reports its errors as auth-code challenges.
#[derive(Clone)]
pub struct EdgeTokenValidationService<T: BbsRepository>(T);

impl<T: BbsRepository> EdgeTokenValidationService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }
}

pub struct EdgeTokenValidationServiceInput {
    pub edge_token: String,
}

#[async_trait::async_trait]
impl<T: BbsRepository> AppService<EdgeTokenValidationServiceInput, Option<AuthedToken>>
    for EdgeTokenValidationService<T>
{
    async fn execute(
        &self,
        input: EdgeTokenValidationServiceInput,
    ) -> anyhow::Result<Option<AuthedToken>> {
        Ok(self
            .0
            .get_authed_token(&input.edge_token)
            .await?
            .filter(|token| token.validity))
    }
}
