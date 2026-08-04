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
    pub classification: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sms_payload_deserialization() {
        let incoming_data = json!({
            "recipient": "+524421234567",
            "message": "Prueba de integración técnica"
        });

        let payload: SmsPayload = serde_json::from_value(incoming_data).unwrap();
        
        assert_eq!(payload.recipient, "+524421234567");
        assert_eq!(payload.message, "Prueba de integración técnica");
    }
}