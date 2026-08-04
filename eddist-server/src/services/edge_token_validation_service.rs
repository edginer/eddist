use crate::{domain::authed_token::AuthedToken, repositories::bbs_repository::BbsRepository};

use super::AppService;

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
