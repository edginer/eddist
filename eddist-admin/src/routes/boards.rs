use axum::{
    extract::{Path, State},
    response::Response,
    routing::{get, patch, post},
    Json, Router,
};
use eddist_core::domain::board::validate_board_key;

use crate::{
    models::{Board, BoardInfo, CreateBoardInput, EditBoardInput},
    repository::admin_bbs_repository::AdminBbsRepository,
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
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
pub async fn get_boards(State(state): State<DefaultAppState>) -> Json<Vec<Board>> {
    let boards = state.admin_bbs_repo.get_boards_by_key(None).await.unwrap();
    boards.into()
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
    State(state): State<DefaultAppState>,
    Path(board_key): Path<String>,
) -> Response {
    let board = state
        .admin_bbs_repo
        .get_boards_by_key(Some(vec![board_key]))
        .await
        .unwrap();
    let Some(board) = board.first() else {
        return Response::builder()
            .status(404)
            .body(axum::body::Body::empty())
            .unwrap();
    };

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&board).unwrap().into())
        .unwrap()
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
    State(state): State<DefaultAppState>,
    Path(board_key): Path<String>,
) -> Response {
    let board = state
        .admin_bbs_repo
        .get_boards_by_key(Some(vec![board_key]))
        .await
        .unwrap();
    let Some(board) = board.first() else {
        return Response::builder()
            .status(404)
            .body(axum::body::Body::empty())
            .unwrap();
    };

    let board_info = state.admin_bbs_repo.get_board_info(board.id).await.unwrap();

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&board_info).unwrap().into())
        .unwrap()
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
    State(state): State<DefaultAppState>,
    Json(body): Json<CreateBoardInput>,
) -> Response {
    if validate_board_key(&body.board_key).is_err() {
        return Response::builder()
            .status(400)
            .body("board_key must be ascii lower alphabetic or numeric".into())
            .unwrap();
    }

    let board = state.admin_bbs_repo.create_board(body).await.unwrap();

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&board).unwrap().into())
        .unwrap()
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
    State(state): State<DefaultAppState>,
    Path(board_key): Path<String>,
    Json(body): Json<EditBoardInput>,
) -> Response {
    let board = state
        .admin_bbs_repo
        .edit_board(&board_key, body)
        .await
        .unwrap();

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&board).unwrap().into())
        .unwrap()
}
