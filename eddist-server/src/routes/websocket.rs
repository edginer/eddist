use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::{IntoResponse, Response},
};
use futures::{SinkExt, StreamExt};
use log::debug;
use serde::Deserialize;
use tracing::{error, info};

use crate::AppState;
use eddist_core::domain::board::validate_board_key;

#[derive(Deserialize)]
pub struct WsQuery {
    board_key: String,
    thread_number: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> Response {
    println!(
        "WebSocket connection request for board_key: {}, thread_number: {}",
        query.board_key, query.thread_number
    );

    if validate_board_key(&query.board_key).is_err() {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }

    let thread_number: u64 = match query.thread_number.parse() {
        Ok(num) => num,
        Err(_) => return axum::http::StatusCode::BAD_REQUEST.into_response(),
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, query.board_key, thread_number))
}

async fn handle_socket(socket: WebSocket, state: AppState, board_key: String, thread_number: u64) {
    let (mut sender, mut receiver) = socket.split();

    info!(
        "WebSocket client connected to thread {} on board {}",
        thread_number, board_key
    );

    // Subscribe via manager instead of direct Redis connection
    let mut broadcast_rx = match state
        .ws_manager
        .subscribe(board_key.clone(), thread_number)
        .await
    {
        Ok(rx) => rx,
        Err(e) => {
            error!(
                "Failed to subscribe to thread {} on board {}: {}",
                thread_number, board_key, e
            );
            return;
        }
    };

    loop {
        tokio::select! {
            // Receive from broadcast channel (forwarded from Redis)
            result = broadcast_rx.recv() => {
                match result {
                    Ok(payload) => {
                        // Send update notification (empty string means "thread updated, please refetch")
                        if let Err(e) = sender.send(Message::Text(payload.into())).await {
                            error!("Failed to send WebSocket message: {}", e);
                            break;
                        }
                    }
                    Err(_) => {
                        // Broadcast channel closed (Redis listener ended)
                        info!("Broadcast channel closed for thread {} on board {}, closing WebSocket", thread_number, board_key);
                        break;
                    }
                }
            }
            Some(Ok(msg)) = receiver.next() => {
                match msg {
                    Message::Close(_) => {
                        debug!("WebSocket client closed connection for thread {} on board {}", thread_number, board_key);
                        break;
                    }
                    Message::Ping(data) => {
                        if let Err(e) = sender.send(Message::Pong(data)).await {
                            error!("Failed to send pong: {}", e);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            else => break,
        }
    }

    // Cleanup: notify manager that this client disconnected
    state
        .ws_manager
        .unsubscribe(&board_key, thread_number)
        .await;

    info!(
        "WebSocket handler finished for thread {} on board {}",
        thread_number, board_key
    );
}
