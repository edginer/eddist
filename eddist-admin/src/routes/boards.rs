use axum::{
    extract::{Path, State},
    routing::{get, patch, post},
    Json, Router,
};
use eddist_core::domain::board::validate_board_key;

use crate::{
    error::ApiError,
    models::{Board, BoardInfo, CreateBoardInput, EditBoardInput},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/boards", get(get_boards))
        .route("/boards", post(create_board))
        .route("/boards/{boardKey}", get(get_board))
        .route("/boards/{boardKey}/info", get(get_board_info))
        .route("/boards/{boardKey}", patch(edit_board))
}

#[utoipa::path(
    get,
    path = "/boards/",
    responses(
        (status = 200, description = "List boards successfully", body = Vec<Board>),
    )
)]
pub async fn get_boards(State(state): State<AppState>) -> Result<Json<Vec<Board>>, ApiError> {
    let boards = state.admin_board_repo.get_boards_by_key(None).await?;
    Ok(Json(boards))
}

#[utoipa::path(
    get,
    path = "/boards/{board_key}/",
    responses(
        (status = 200, description = "Get board successfully", body = Board),
        (status = 404, description = "Board not found"),
    ),
    params(
        ("board_key" = String, Path, description = "Board ID"),
    )
)]
pub async fn get_board(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
) -> Result<Json<Board>, ApiError> {
    let board = state
        .admin_board_repo
        .get_boards_by_key(Some(vec![board_key]))
        .await?;
    let board = board
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::not_found("Board not found"))?;
    Ok(Json(board))
}

#[utoipa::path(
    get,
    path = "/boards/{board_key}/info/",
    responses(
        (status = 200, description = "Get board info successfully", body = BoardInfo),
        (status = 404, description = "Board not found"),
    ),
    params(
        ("board_key" = String, Path, description = "Board ID"),
    ))
]
pub async fn get_board_info(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
) -> Result<Json<BoardInfo>, ApiError> {
    let board = state
        .admin_board_repo
        .get_boards_by_key(Some(vec![board_key]))
        .await?;
    let board = board
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::not_found("Board not found"))?;

    let board_info = state.admin_board_repo.get_board_info(board.id).await?;
    Ok(Json(board_info))
}

#[utoipa::path(
    post,
    path = "/boards/",
    responses(
        (status = 200, description = "Create board successfully", body = CreateBoardInput),
    ),
    request_body = CreateBoardInput
)]
pub async fn create_board(
    State(state): State<AppState>,
    Json(body): Json<CreateBoardInput>,
) -> Result<Json<Board>, ApiError> {
    if validate_board_key(&body.board_key).is_err() {
        return Err(ApiError::bad_request(
            "board_key must be ascii lower alphabetic or numeric",
        ));
    }

    let board = state.admin_board_repo.create_board(body).await?;
    Ok(Json(board))
}

#[utoipa::path(
    patch,
    path = "/boards/{board_key}/",
    responses(
        (status = 200, description = "Edit board successfully", body = Board),
    ),
    params(
        ("board_key" = Uuid, Path, description = "Board Key"),
    ),
    request_body = EditBoardInput
)]
pub async fn edit_board(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
    Json(body): Json<EditBoardInput>,
) -> Result<Json<Board>, ApiError> {
    let board = state.admin_board_repo.edit_board(&board_key, body).await?;
    Ok(Json(board))
}
