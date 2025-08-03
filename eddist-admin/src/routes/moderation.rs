use axum::{
    extract::{Path, State},
    response::Response,
    routing::{delete, get, patch, post},
    Json, Router,
};
use eddist_core::domain::user_restriction::{
    CreateUserRestrictionRuleInput, UpdateUserRestrictionRuleInput, UserRestrictionRule,
};
use uuid::Uuid;

use crate::{
    auth::AdminSession,
    models::{
        Cap, CreateRestrictionRuleRequest, CreationCapInput, CreationNgWordInput, NgWord,
        UpdateCapInput, UpdateNgWordInput, UpdateRestrictionRuleRequest,
    },
    repository::{
        cap_repository::CapRepository, ngword_repository::NgWordRepository,
        user_restriction_repository::UserRestrictionRepository,
    },
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
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
pub async fn get_ng_words(State(state): State<DefaultAppState>) -> Json<Vec<NgWord>> {
    let ng_words = state.ng_word_repo.get_ng_words().await.unwrap();
    ng_words.into()
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
    State(state): State<DefaultAppState>,
    Json(body): Json<CreationNgWordInput>,
) -> Response {
    let ng_word = state
        .ng_word_repo
        .create_ng_word(&body.name, &body.word)
        .await
        .unwrap();

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&ng_word).unwrap().into())
        .unwrap()
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
    State(state): State<DefaultAppState>,
    Path(ng_word_id): Path<Uuid>,
    Json(body): Json<UpdateNgWordInput>,
) -> Response {
    let ng_word = state
        .ng_word_repo
        .update_ng_word(
            ng_word_id,
            body.name.as_deref(),
            body.word.as_deref(),
            body.board_ids,
        )
        .await
        .unwrap();

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&ng_word).unwrap().into())
        .unwrap()
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
    State(state): State<DefaultAppState>,
    Path(ng_word_id): Path<Uuid>,
) -> Response {
    state.ng_word_repo.delete_ng_word(ng_word_id).await.unwrap();

    Response::builder()
        .status(200)
        .body(axum::body::Body::empty())
        .unwrap()
}

// Cap handlers
#[utoipa::path(
    get,
    path = "/caps/",
    responses(
        (status = 200, description = "List cap words successfully", body = Vec<Cap>),
    )
)]
pub async fn get_caps(State(state): State<DefaultAppState>) -> Json<Vec<Cap>> {
    let caps = state.cap_repo.get_caps().await.unwrap();
    caps.into()
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
    State(state): State<DefaultAppState>,
    Json(body): Json<CreationCapInput>,
) -> Response {
    let cap = state
        .cap_repo
        .create_cap(
            &body.name,
            &body.description,
            &eddist_core::domain::cap::calculate_cap_hash(
                &body.password,
                &std::env::var("TINKER_SECRET").unwrap(),
            ),
        )
        .await
        .unwrap();

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&cap).unwrap().into())
        .unwrap()
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
    State(state): State<DefaultAppState>,
    Path(cap_id): Path<Uuid>,
    Json(body): Json<UpdateCapInput>,
) -> Response {
    let cap = state
        .cap_repo
        .update_cap(
            cap_id,
            body.name.as_deref(),
            body.description.as_deref(),
            body.password
                .map(|x| {
                    eddist_core::domain::cap::calculate_cap_hash(
                        &x,
                        &std::env::var("TINKER_SECRET").unwrap(),
                    )
                })
                .as_deref(),
            body.board_ids,
        )
        .await
        .unwrap();

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&cap).unwrap().into())
        .unwrap()
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
    State(state): State<DefaultAppState>,
    Path(cap_id): Path<Uuid>,
) -> Response {
    state.cap_repo.delete_cap(cap_id).await.unwrap();

    Response::builder()
        .status(200)
        .body(axum::body::Body::empty())
        .unwrap()
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
    State(app_state): State<DefaultAppState>,
) -> Json<Vec<UserRestrictionRule>> {
    let rules = app_state
        .user_restriction_repo
        .get_all_rules()
        .await
        .unwrap_or_default();
    Json(rules)
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
    State(app_state): State<DefaultAppState>,
    admin_session: AdminSession,
    Json(req): Json<CreateRestrictionRuleRequest>,
) -> Response {
    // Extract Auth0 user ID from session userinfo
    let Some(admin_user_id) = admin_session.get_admin_email() else {
        return Response::builder()
            .status(401)
            .body("Unauthorized: No user information available".into())
            .unwrap();
    };

    let input = CreateUserRestrictionRuleInput {
        name: req.name,
        rule_type: req.rule_type.into(),
        rule_value: req.rule_value,
        expires_at: req.expires_at,
        created_by_email: admin_user_id,
    };

    let rule = app_state
        .user_restriction_repo
        .create_rule(input)
        .await
        .unwrap();

    Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&rule).unwrap().into())
        .unwrap()
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
    State(app_state): State<DefaultAppState>,
    Json(req): Json<UpdateRestrictionRuleRequest>,
) -> Json<()> {
    let input = UpdateUserRestrictionRuleInput {
        id: rule_id,
        name: req.name,
        rule_type: req.rule_type.map(|rt| rt.into()),
        rule_value: req.rule_value,
        expires_at: req.expires_at,
    };

    app_state
        .user_restriction_repo
        .update_rule(input)
        .await
        .unwrap();
    Json(())
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
    State(app_state): State<DefaultAppState>,
) -> Json<()> {
    app_state
        .user_restriction_repo
        .delete_rule(rule_id)
        .await
        .unwrap();
    Json(())
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
    State(app_state): State<DefaultAppState>,
) -> Json<Option<UserRestrictionRule>> {
    let rule = app_state
        .user_restriction_repo
        .get_rule_by_id(rule_id)
        .await
        .unwrap();
    Json(rule)
}
