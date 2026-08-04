use reqwest::Client;
use serde_json::json;
use std::env;
use std::sync::OnceLock;
use std::time::Duration;

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn get_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("No se pudo construir el cliente HTTP")
    })
}

/// Actúa como un interceptor de seguridad evaluando el contexto del mensaje.
/// Se comunica con la API de Google Gemini para clasificar el texto en tiempo real.
pub async fn evaluate_sms_spam(message: &str) -> Result<String, String> {
    let api_key = env::var("GEMINI_API_KEY")
        .map_err(|_| "Error crítico: Falta configurar GEMINI_API_KEY".to_string())?;
    
    // Utilizamos el modelo Flash por su baja latencia, ideal para flujos de red rápidos
    let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.6-flash:generateContent";
    
    let client = get_client();
    
    let payload = json!({
        "contents": [{
            "parts": [{"text": format!("Actúa como un filtro de seguridad de telecomunicaciones. Analiza el siguiente SMS y clasifícalo estrictamente con una sola palabra de esta lista: LEGITIMO, SPAM, o PHISHING. No agregues saludos ni explicaciones. Mensaje: '{}'", message)}]
        }]
    });

    let res = client.post(url)
        .header("x-goog-api-key", &api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !res.status().is_success() {
        let status = res.status();
        let error_body = res.text().await.unwrap_or_default();
        return Err(format!("Error HTTP {}: {}", status, error_body));
    }

    let json_body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    
    let evaluation = json_body["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("DESCONOCIDO")
        .trim()
        .to_uppercase();

    Ok(evaluation)
}