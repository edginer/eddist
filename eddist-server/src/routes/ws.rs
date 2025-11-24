use axum::{
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade, close_code},
    },
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{
    app::AppState,
    repositories::bbs_repository::BbsRepository,
    services::{AppService, user_page_service::UserPageServiceInput},
};

/// Maximum duration for a WebSocket connection (1 hour)
const MAX_SESSION_DURATION: Duration = Duration::from_secs(3600);
/// Interval for sending ping messages to keep connection alive
const PING_INTERVAL: Duration = Duration::from_secs(30);

/// WebSocket handler for real-time thread updates
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    jar: CookieJar,
    Path((board_key, thread_number)): Path<(String, u64)>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let user_sid = jar
        .get("user-sid")
        .map(|c| c.value().to_string())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let _user = state
        .services
        .user_page()
        .execute(UserPageServiceInput { user_sid })
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let thread_uuid = match state
        .bbs_repo
        .get_thread_by_board_key_and_thread_number(&board_key, thread_number)
        .await
        .map(|opt| opt.map(|t| t.id))
    {
        Ok(Some(uuid)) => uuid,
        Ok(None) => {
            return Ok(ws.on_upgrade(|mut socket| async move {
                let _ = socket
                    .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code: close_code::POLICY,
                        reason: "Thread not found".into(),
                    })))
                    .await;
            }));
        }
        Err(_) => {
            return Ok(ws.on_upgrade(|mut socket| async move {
                let _ = socket
                    .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code: close_code::ERROR,
                        reason: "Internal error".into(),
                    })))
                    .await;
            }));
        }
    };

    debug!("WebSocket connection request for board: {board_key}, thread: {thread_number}");

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, thread_uuid)))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, thread_id: Uuid) {
    let mut rx = state.stream_manager.subscribe(thread_id);
    let mut ping_interval = tokio::time::interval(PING_INTERVAL);

    debug!("WebSocket connected for thread: {thread_id}");

    let _ = tokio::time::timeout(MAX_SESSION_DURATION, async move {
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            if socket.send(Message::Text(msg.into())).await.is_err() {
                                // Client disconnected
                                break;
                            }
                        }
                        Err(RecvError::Lagged(count)) => {
                            warn!("WebSocket client lagged, skipped {count} messages");
                        }
                        Err(RecvError::Closed) => {
                            break;
                        }
                    }
                }

                // Send periodic pings to keep connection alive
                _ = ping_interval.tick() => {
                    if socket.send(Message::Ping(Vec::new().into())).await.is_err() {
                        break;
                    }
                }

                // Handle incoming messages from client (pings, close)
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(Message::Ping(data))) => {
                            if socket.send(Message::Pong(data)).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // Pong received, connection is alive
                        }
                        Some(Ok(Message::Close(_))) => {
                            break;
                        }
                        Some(Ok(_)) => {
                            // Ignore other message types (Text, Binary)
                        }
                        Some(Err(_)) => {
                            // Error receiving
                            break;
                        }
                        None => {
                            // Connection closed
                            break;
                        }
                    }
                }
            }
        }
    })
    .await;

    debug!("WebSocket disconnected for thread: {thread_id}");

    state.stream_manager.cleanup_unused();
}
