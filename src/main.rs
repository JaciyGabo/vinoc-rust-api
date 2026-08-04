mod models;
mod error;
mod ai;

use axum::{
    routing::{get, post}, 
    Router, Json
};
use error::ApiError;
use models::{HealthResponse, SmsPayload, SmsResponse};
use std::time::Instant;
use tracing::{info, instrument, warn, error};
use regex::Regex;

// Estado global estático para calcular el tiempo de actividad (uptime) del servidor
static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

const MAX_MESSAGE_LENGTH: usize = 500;

static PHONE_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

fn phone_regex() -> &'static Regex {
    PHONE_REGEX.get_or_init(|| {
        Regex::new(r"^\+[1-9]\d{6,14}$").expect("Regex de teléfono inválida")
    })
}

#[tokio::main]
async fn main() {
    // Inicialización de variables de entorno y sistema de logs estructurados
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    START_TIME.set(Instant::now()).ok();

    // Configuración del enrutador principal de Axum
    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/sms/send", post(send_sms_handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    info!("Server running on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}

/// Endpoint de monitoreo para balanceadores de carga y orquestadores.
/// Devuelve el estado actual y el tiempo de actividad del servicio.
async fn health_handler() -> Json<HealthResponse> {
    let elapsed = START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0);
    
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "VINOC Telecom Gateway".to_string(),
        uptime_seconds: elapsed,
    })
}


/// Procesa las solicitudes entrantes de envío de SMS.
/// Implementa validación de reglas de negocio y evaluación de contenido mediante IA.
#[instrument(skip(payload))] // Inyecta el contexto de la función en los logs omitiendo datos sensibles
async fn send_sms_handler(Json(payload): Json<SmsPayload>) -> Result<Json<SmsResponse>, ApiError> {
    info!("Procesando envío de SMS para: {}", payload.recipient);

    // 1. Capa de Validación de Estructura
    if !phone_regex().is_match(&payload.recipient) {
        warn!("Validación fallida: formato de número inválido ('{}')", payload.recipient);
        return Err(ApiError::ValidationError(
            "Formato inválido. Usa formato E.164 con código de país (ej. +524421234567).".to_string()
        ));
    }

    if payload.message.chars().count() > MAX_MESSAGE_LENGTH {
        warn!("Validación fallida: mensaje demasiado largo ({} caracteres)", payload.message.chars().count());
        return Err(ApiError::ValidationError(
            format!("El mensaje excede el límite de {} caracteres.", MAX_MESSAGE_LENGTH)
        ));
    }

    if payload.message.trim().is_empty() {
        warn!("Validación fallida: cuerpo del mensaje vacío para el número '{}'", payload.recipient);
        
        return Err(ApiError::ValidationError(
            "El cuerpo del mensaje SMS no puede estar vacío.".to_string()
        ));
    }

    // 2. Capa de Seguridad (Agente IA)
    info!("Evaluando contenido del mensaje con Gemini IA...");
    let classification = match ai::evaluate_sms_spam(&payload.message).await {
        Ok(result) => result,
        Err(e) => {
            error!("Fallo en el Agente de IA: {}", e);
            "ERROR_IA".to_string()
        }
    };

    // Bloqueo preventivo en caso de detectar amenazas
    if classification == "SPAM" || classification == "PHISHING" {
        warn!("Seguridad: Mensaje bloqueado por Agente IA. Razón: {}", classification);
        return Err(ApiError::ValidationError(
            format!("Mensaje rechazado por políticas de seguridad de red (Clasificación: {})", classification)
        ));
    }

    if classification == "ERROR_IA" {
    return Err(ApiError::ValidationError(
        "No se pudo validar el mensaje en este momento. Intenta de nuevo.".to_string()
    ));
}

    // 3. Capa de Respuesta y Encolado
    let mock_id = format!("msg_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());

    Ok(Json(SmsResponse {
        message_id: mock_id,
        status: "queued".to_string(),
        recipient: payload.recipient,
        classification,
    }))
}