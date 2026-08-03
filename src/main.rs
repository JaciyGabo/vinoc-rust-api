mod models;
mod errors;

use axum::{
    routing::{get, post}, 
    Router, Json
};
use errors::ApiError;
use models::{HealthResponse, SmsPayload, SmsResponse};
use std::time::Instant;
use tracing::{info, instrument, warn};

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
async fn send_sms_handler(Json(payload): Json<SmsPayload>) -> Result<Json<SmsResponse>, ApiError> {
    info!("Procesando envío de SMS para: {}", payload.recipient);

    if payload.recipient.is_empty() || !payload.recipient.starts_with('+') {
        warn!("Validación fallida: formato de número inválido ('{}')", payload.recipient);
 
        return Err(ApiError::ValidationError(
            "Formato inválido. El número debe incluir el código de país (ej. +52).".to_string()
        ));
    }

    if payload.message.trim().is_empty() {
        warn!("Validación fallida: cuerpo del mensaje vacío para el número '{}'", payload.recipient);
        
        return Err(ApiError::ValidationError(
            "El cuerpo del mensaje SMS no puede estar vacío.".to_string()
        ));
    }

    let mock_id = format!("msg_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());

    Ok(Json(SmsResponse {
        message_id: mock_id,
        status: "queued".to_string(),
        recipient: payload.recipient,
    }))
}