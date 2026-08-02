use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub uptime_seconds: u64,
}

#[derive(Deserialize)]
pub struct SmsPayload {
    pub recipient: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct SmsResponse {
    pub message_id: String,
    pub status: String,
    pub recipient: String,
}