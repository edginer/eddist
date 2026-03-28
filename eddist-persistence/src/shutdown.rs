use std::convert::Infallible;

use hyper::{Response, server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::TcpListener;
use tracing::error_span;

pub async fn run_shutdown_server(ctrl_c_tx: tokio::sync::broadcast::Sender<()>) {
    let listener = TcpListener::bind("0.0.0.0:9874").await.unwrap();
    if let Ok((stream, _)) = listener.accept().await {
        let svc = service_fn(|_| async move {
            let response = Response::new("Request received. Shutting down.\n".to_string());
            Ok::<_, Infallible>(response)
        });

        let mut builder = http1::Builder::new();
        let builder = builder.timer(TokioTimer::new());
        builder
            .serve_connection(TokioIo::new(stream), svc)
            .await
            .unwrap();
    }

    error_span!(
        "received shutdown signal",
        message = "shutting down the service"
    );

    ctrl_c_tx.send(()).unwrap();
}
