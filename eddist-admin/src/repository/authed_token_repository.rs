use crate::entity::authed_token;
use crate::models::AuthedToken;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

pub struct ListAuthedTokensParams<'a> {
    pub offset: u64,
    pub limit: u32,
    pub origin_ip: Option<&'a str>,
    pub writing_ua: Option<&'a str>,
    pub authed_ua: Option<&'a str>,
    pub asn_num: Option<i32>,
    pub validity: Option<bool>,
    pub sort_column: &'a str,
    pub sort_asc: bool,
}

#[async_trait::async_trait]
pub trait AuthedTokenRepository: Send + Sync {
    async fn get_authed_token(&self, id: Uuid) -> anyhow::Result<AuthedToken>;
    async fn delete_authed_token(&self, id: Uuid) -> anyhow::Result<()>;
    async fn delete_authed_token_by_origin_ip(&self, id: Uuid) -> anyhow::Result<Vec<Uuid>>;
    async fn list_authed_tokens(
        &self,
        params: ListAuthedTokensParams<'_>,
    ) -> anyhow::Result<(Vec<AuthedToken>, u64)>;
    async fn set_require_reauth(&self, id: Uuid) -> anyhow::Result<()>;
    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct AuthedTokenRepositoryImpl(DatabaseConnection);

impl AuthedTokenRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

fn into_domain(model: authed_token::Model) -> AuthedToken {
    AuthedToken {
        id: model.id,
        token: model.token,
        origin_ip: model.origin_ip,
        reduced_origin_ip: model.reduced_origin_ip,
        asn_num: model.asn_num,
        writing_ua: model.writing_ua,
        authed_ua: model.authed_ua,
        created_at: model.created_at,
        authed_at: model.authed_at,
        validity: model.validity,
        last_wrote_at: model.last_wrote_at,
        additional_info: model.additional_info,
        require_reauth: model.require_reauth,
        // Suspension lives in Redis, not this table; callers fill this in from there.
        is_suspended: None,
    }
}

#[async_trait::async_trait]
impl AuthedTokenRepository for AuthedTokenRepositoryImpl {
    async fn get_authed_token(&self, id: Uuid) -> anyhow::Result<AuthedToken> {
        authed_token::Entity::find_by_id(id)
            .one(&self.0)
            .await?
            .map(into_domain)
            .ok_or_else(|| {
                crate::error::ServiceError::NotFound("Authed token not found".into()).into()
            })
    }

    async fn delete_authed_token(&self, id: Uuid) -> anyhow::Result<()> {
        authed_token::ActiveModel {
            id: Set(id),
            validity: Set(false),
            ..Default::default()
        }
        .update(&self.0)
        .await?;
        Ok(())
    }

    async fn delete_authed_token_by_origin_ip(&self, id: Uuid) -> anyhow::Result<Vec<Uuid>> {
        let Some(target) = authed_token::Entity::find_by_id(id).one(&self.0).await? else {
            return Ok(Vec::new());
        };

        let affected_ids = authed_token::Entity::find()
            .filter(authed_token::Column::Validity.eq(true))
            .filter(authed_token::Column::OriginIp.eq(target.origin_ip.clone()))
            .all(&self.0)
            .await?
            .into_iter()
            .map(|model| model.id)
            .collect::<Vec<_>>();

        authed_token::Entity::update_many()
            .col_expr(authed_token::Column::Validity, Expr::value(false))
            .filter(authed_token::Column::OriginIp.eq(target.origin_ip))
            .exec(&self.0)
            .await?;

        Ok(affected_ids)
    }

    async fn list_authed_tokens(
        &self,
        params: ListAuthedTokensParams<'_>,
    ) -> anyhow::Result<(Vec<AuthedToken>, u64)> {
        let ListAuthedTokensParams {
            offset,
            limit,
            origin_ip,
            writing_ua,
            authed_ua,
            asn_num,
            validity,
            sort_column,
            sort_asc,
        } = params;

        let mut query = authed_token::Entity::find();
        if let Some(origin_ip) = origin_ip {
            query = query.filter(authed_token::Column::OriginIp.eq(origin_ip));
        }
        if let Some(writing_ua) = writing_ua {
            query = query.filter(authed_token::Column::WritingUa.contains(writing_ua));
        }
        if let Some(authed_ua) = authed_ua {
            query = query.filter(authed_token::Column::AuthedUa.contains(authed_ua));
        }
        if let Some(asn_num) = asn_num {
            query = query.filter(authed_token::Column::AsnNum.eq(asn_num));
        }
        if let Some(validity) = validity {
            query = query.filter(authed_token::Column::Validity.eq(validity));
        }

        let total = query.clone().count(&self.0).await?;
        let query = match sort_column {
            "created_at" => {
                if sort_asc {
                    query.order_by_asc(authed_token::Column::CreatedAt)
                } else {
                    query.order_by_desc(authed_token::Column::CreatedAt)
                }
            }
            "authed_at" => {
                if sort_asc {
                    query.order_by_asc(authed_token::Column::AuthedAt)
                } else {
                    query.order_by_desc(authed_token::Column::AuthedAt)
                }
            }
            "last_wrote_at" => {
                if sort_asc {
                    query.order_by_asc(authed_token::Column::LastWroteAt)
                } else {
                    query.order_by_desc(authed_token::Column::LastWroteAt)
                }
            }
            _ => anyhow::bail!("invalid sort column: {sort_column}"),
        };

        let models = query
            .limit(u64::from(limit))
            .offset(offset)
            .all(&self.0)
            .await?;
        Ok((models.into_iter().map(into_domain).collect(), total))
    }

    async fn set_require_reauth(&self, id: Uuid) -> anyhow::Result<()> {
        authed_token::ActiveModel {
            id: Set(id),
            require_reauth: Set(true),
            ..Default::default()
        }
        .update(&self.0)
        .await?;
        Ok(())
    }

    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()> {
        authed_token::ActiveModel {
            id: Set(id),
            require_reauth: Set(false),
            ..Default::default()
        }
        .update(&self.0)
        .await?;
        Ok(())
    }
}
