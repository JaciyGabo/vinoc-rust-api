use axum::{routing::get, Router, Json};
use serde::Serialize;

#[derive(Serialize)]
struct HealthStatus {
    status: String,
    service: String,
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(health_check));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<HealthStatus> {
    Json(HealthStatus {
        status: "ok".to_string(),
        service: "VINOC Telecom API".to_string(),
    })
}