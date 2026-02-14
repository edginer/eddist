use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use eddist_core::domain::user_restriction::{
    CreateUserRestrictionRuleInput, UpdateUserRestrictionRuleInput, UserRestrictionRule,
};
use uuid::Uuid;

use crate::{
    auth::AdminEmail,
    error::ApiError,
    models::{
        Cap, CreateRestrictionRuleRequest, CreationCapInput, CreationNgWordInput, NgWord,
        UpdateCapInput, UpdateNgWordInput, UpdateRestrictionRuleRequest,
    },
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // NgWords
        .route("/ng_words", get(get_ng_words))
        .route("/ng_words", post(create_ng_word))
        .route("/ng_words/{ngWordId}", delete(delete_ng_word))
        .route("/ng_words/{ngWordId}", patch(update_ng_word))
        // Caps
        .route("/caps", get(get_caps))
        .route("/caps", post(create_cap))
        .route("/caps/{capId}", delete(delete_cap))
        .route("/caps/{capId}", patch(update_cap))
        // User Restrictions
        .route("/restriction_rules", get(get_restriction_rules))
        .route("/restriction_rules", post(create_restriction_rule))
        .route("/restriction_rules/{rule_id}", get(get_restriction_rule))
        .route(
            "/restriction_rules/{rule_id}",
            patch(update_restriction_rule),
        )
        .route(
            "/restriction_rules/{rule_id}",
            delete(delete_restriction_rule),
        )
}

// NgWord handlers
#[utoipa::path(
    get,
    path = "/ng_words/",
    responses(
        (status = 200, description = "List ng words successfully", body = Vec<NgWord>),
    )
)]
pub async fn get_ng_words(State(state): State<AppState>) -> Result<Json<Vec<NgWord>>, ApiError> {
    let ng_words = state.ng_word_repo.get_ng_words().await?;
    Ok(Json(ng_words))
}

#[utoipa::path(
    post,
    path = "/ng_words/",
    responses(
        (status = 200, description = "Create ng word successfully", body = NgWord),
    ),
    request_body = CreationNgWordInput
)]
pub async fn create_ng_word(
    State(state): State<AppState>,
    Json(body): Json<CreationNgWordInput>,
) -> Result<Json<NgWord>, ApiError> {
    let ng_word = state
        .ng_word_repo
        .create_ng_word(&body.name, &body.word)
        .await?;
    Ok(Json(ng_word))
}

#[utoipa::path(
    patch,
    path = "/ng_words/{ng_word_id}/",
    responses(
        (status = 200, description = "Update ng word successfully", body = NgWord),
    ),
    params(
        ("ng_word_id" = Uuid, Path, description = "NG word ID"),
    ),
    request_body = UpdateNgWordInput
)]
pub async fn update_ng_word(
    State(state): State<AppState>,
    Path(ng_word_id): Path<Uuid>,
    Json(body): Json<UpdateNgWordInput>,
) -> Result<Json<NgWord>, ApiError> {
    let ng_word = state
        .ng_word_repo
        .update_ng_word(
            ng_word_id,
            body.name.as_deref(),
            body.word.as_deref(),
            body.board_ids,
        )
        .await?;
    Ok(Json(ng_word))
}

#[utoipa::path(
    delete,
    path = "/ng_words/{ng_word_id}/",
    responses(
        (status = 200, description = "Delete ng word successfully"),
    ),
    params(
        ("ng_word_id" = Uuid, Path, description = "NG word ID"),
    ),
)]
pub async fn delete_ng_word(
    State(state): State<AppState>,
    Path(ng_word_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.ng_word_repo.delete_ng_word(ng_word_id).await?;
    Ok(StatusCode::OK)
}

// Cap handlers
#[utoipa::path(
    get,
    path = "/caps/",
    responses(
        (status = 200, description = "List cap words successfully", body = Vec<Cap>),
    )
)]
pub async fn get_caps(State(state): State<AppState>) -> Result<Json<Vec<Cap>>, ApiError> {
    let caps = state.cap_repo.get_caps().await?;
    Ok(Json(caps))
}

#[utoipa::path(
    post,
    path = "/caps/",
    responses(
        (status = 200, description = "Create cap successfully", body = Cap),
    ),
    request_body = CreationCapInput
)]
pub async fn create_cap(
    State(state): State<AppState>,
    Json(body): Json<CreationCapInput>,
) -> Result<Json<Cap>, ApiError> {
    let tinker_secret = std::env::var("TINKER_SECRET")
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("TINKER_SECRET not configured")))?;
    let cap = state
        .cap_repo
        .create_cap(
            &body.name,
            &body.description,
            &eddist_core::domain::cap::calculate_cap_hash(&body.password, &tinker_secret),
        )
        .await?;
    Ok(Json(cap))
}

#[utoipa::path(
    patch,
    path = "/caps/{cap_id}/",
    responses(
        (status = 200, description = "Update cap word successfully", body = Cap),
    ),
    params(
        ("cap_id" = Uuid, Path, description = "Cap ID"),
    ),
    request_body = UpdateCapInput
)]
pub async fn update_cap(
    State(state): State<AppState>,
    Path(cap_id): Path<Uuid>,
    Json(body): Json<UpdateCapInput>,
) -> Result<Json<Cap>, ApiError> {
    let cap = state
        .cap_repo
        .update_cap(
            cap_id,
            body.name.as_deref(),
            body.description.as_deref(),
            body.password
                .map(|x| {
                    let secret = std::env::var("TINKER_SECRET").unwrap_or_default();
                    eddist_core::domain::cap::calculate_cap_hash(&x, &secret)
                })
                .as_deref(),
            body.board_ids,
        )
        .await?;
    Ok(Json(cap))
}

#[utoipa::path(
    delete,
    path = "/caps/{cap_id}/",
    responses(
        (status = 200, description = "Delete Cap successfully"),
    ),
    params(
        ("cap_id" = Uuid, Path, description = "Cap ID"),
    ),
)]
pub async fn delete_cap(
    State(state): State<AppState>,
    Path(cap_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.cap_repo.delete_cap(cap_id).await?;
    Ok(StatusCode::OK)
}

// User Restriction handlers
#[utoipa::path(
    get,
    path = "/restriction_rules",
    responses(
        (status = 200, description = "List all restriction rules", body = Vec<crate::models::UserRestrictionRuleSchema>)
    )
)]
pub async fn get_restriction_rules(
    State(app_state): State<AppState>,
) -> Result<Json<Vec<UserRestrictionRule>>, ApiError> {
    let rules = app_state
        .user_restriction_repo
        .get_all_rules()
        .await
        .unwrap_or_default();
    Ok(Json(rules))
}

#[utoipa::path(
    post,
    path = "/restriction_rules",
    request_body = CreateRestrictionRuleRequest,
    responses(
        (status = 201, description = "Create restriction rule", body = crate::models::UserRestrictionRuleSchema)
    )
)]
pub async fn create_restriction_rule(
    State(app_state): State<AppState>,
    AdminEmail(admin_email): AdminEmail,
    Json(req): Json<CreateRestrictionRuleRequest>,
) -> Result<(StatusCode, Json<UserRestrictionRule>), ApiError> {
    let input = CreateUserRestrictionRuleInput {
        name: req.name,
        rule_type: req.rule_type.into(),
        rule_value: req.rule_value,
        expires_at: req.expires_at,
        created_by_email: admin_email,
    };

    let rule = app_state.user_restriction_repo.create_rule(input).await?;
    Ok((StatusCode::CREATED, Json(rule)))
}

#[utoipa::path(
    patch,
    path = "/restriction_rules/{rule_id}",
    request_body = UpdateRestrictionRuleRequest,
    responses(
        (status = 200, description = "Update restriction rule")
    ),
    params(
        ("rule_id" = Uuid, Path, description = "Rule ID")
    )
)]
pub async fn update_restriction_rule(
    Path(rule_id): Path<Uuid>,
    State(app_state): State<AppState>,
    Json(req): Json<UpdateRestrictionRuleRequest>,
) -> Result<Json<()>, ApiError> {
    let input = UpdateUserRestrictionRuleInput {
        id: rule_id,
        name: req.name,
        rule_type: req.rule_type.map(|rt| rt.into()),
        rule_value: req.rule_value,
        expires_at: req.expires_at,
    };

    app_state.user_restriction_repo.update_rule(input).await?;
    Ok(Json(()))
}

#[utoipa::path(
    delete,
    path = "/restriction_rules/{rule_id}",
    responses(
        (status = 200, description = "Delete restriction rule")
    ),
    params(
        ("rule_id" = Uuid, Path, description = "Rule ID")
    )
)]
pub async fn delete_restriction_rule(
    Path(rule_id): Path<Uuid>,
    State(app_state): State<AppState>,
) -> Result<Json<()>, ApiError> {
    app_state.user_restriction_repo.delete_rule(rule_id).await?;
    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/restriction_rules/{rule_id}",
    responses(
        (status = 200, description = "Get restriction rule by ID", body = crate::models::UserRestrictionRuleSchema)
    ),
    params(
        ("rule_id" = Uuid, Path, description = "Rule ID")
    )
)]
pub async fn get_restriction_rule(
    Path(rule_id): Path<Uuid>,
    State(app_state): State<AppState>,
) -> Result<Json<Option<UserRestrictionRule>>, ApiError> {
    let rule = app_state
        .user_restriction_repo
        .get_rule_by_id(rule_id)
        .await?;
    Ok(Json(rule))
}
