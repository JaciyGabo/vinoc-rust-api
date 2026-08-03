mod models;

use axum::{
    routing::{get, post}, 
    Router, Json
};
use models::{HealthResponse, SmsPayload, SmsResponse};
use std::time::Instant;
use tracing::{info, instrument};

static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    START_TIME.set(Instant::now()).ok();

    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/sms/send", post(send_sms_handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    info!("Server running on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler() -> Json<HealthResponse> {
    let elapsed = START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0);
    
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "VINOC Telecom Gateway".to_string(),
        uptime_seconds: elapsed,
    })
}

#[instrument(skip(payload))]
async fn send_sms_handler(Json(payload): Json<SmsPayload>) -> Json<SmsResponse> {
    info!("Procesando envío de SMS para: {}", payload.recipient);

    Json(SmsResponse {
        message_id: "msg_9876543210".to_string(),
        status: "queued".to_string(),
        recipient: payload.recipient,
    })
}