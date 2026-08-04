# Secure SMS Gateway API - Rust PoC

Prueba de Concepto (PoC) de un gateway de telecomunicaciones construido en **Rust**. La API simula el encolado de mensajes SMS e integra una capa de seguridad impulsada por IA (Google Gemini) que intercepta y bloquea en tiempo real mensajes clasificados como SPAM o PHISHING antes de encolarlos.

## Arquitectura y tecnologías

- **Lenguaje:** Rust — seguridad en memoria y concurrencia sin data races.
- **Framework web:** [`axum`](https://github.com/tokio-rs/axum) — routing tipado sobre `tokio`.
- **Runtime asíncrono:** `tokio` (`#[tokio::main]`).
- **Cliente HTTP:** `reqwest` — cliente reutilizado (`OnceLock`) con timeout de 10s, para invocar la API de Google Gemini.
- **Agente de IA:** Google AI Studio (Gemini API vía `generateContent`), con **structured output** (`responseSchema`) para forzar la respuesta a un enum fijo.
- **Serialización:** `serde` / `serde_json`.
- **Configuración:** `dotenvy` — carga de variables desde `.env`.
- **Validación:** `regex` (formato E.164 del destinatario).
- **Identificadores:** `uuid` (v4) para `message_id`.
- **Observabilidad:** `tracing` + `tracing-subscriber` — logs estructurados, con `#[instrument]` en el handler principal.
- **Manejo de errores:** enum `ApiError` centralizado que implementa `IntoResponse`, devolviendo `400 Bad Request` con un cuerpo JSON consistente.

## Estructura del proyecto

```
src/
├── main.rs     # Router de Axum, arranque del servidor y handlers
├── models.rs   # Structs de request/response (Serialize/Deserialize) + tests
├── error.rs    # Enum ApiError y su conversión a respuesta HTTP
└── ai.rs       # Cliente del agente de IA (Gemini) para clasificación de spam
```

## Prerrequisitos

1. [Rust y Cargo](https://rustup.rs/) (edición 2021 o superior).
2. Una API key de Google Gemini (Google AI Studio) con acceso al modelo configurado en `ai.rs`.

## Instalación y configuración

1. **Clonar el repositorio:**
   ```bash
   git clone https://github.com/JaciyGabo/vinoc-rust-api.git
   cd vinoc-rust-api
   ```

2. **Configurar variables de entorno:** crea un archivo `.env` en la raíz del proyecto:
   ```
   GEMINI_API_KEY=tu_api_key_aqui
   ```

3. **Compilar y ejecutar:**
   ```bash
   cargo run
   ```
   El servidor queda escuchando en `http://127.0.0.1:3000`.

## Endpoints de la API

### 1. Health Check

Verifica el estado del servicio y su tiempo de actividad.

- **GET** `/api/health`

**Respuesta (200 OK):**
```json
{
  "status": "ok",
  "service": "VINOC Telecom Gateway",
  "uptime_seconds": 124
}
```

### 2. Envío de SMS (protegido por IA)

Valida el payload, lo evalúa con el agente de IA y lo encola si es seguro.

- **POST** `/api/sms/send`
- **Headers:** `Content-Type: application/json`

**Payload esperado:**
```json
{
  "recipient": "+524421234567",
  "message": "Hola, confirma tu cita médica para mañana."
}
```

**Reglas de validación (antes de invocar a la IA):**
- `recipient` debe cumplir formato **E.164** (ej. `+524421234567`) — validado con `regex`.
- `message` no puede estar vacío, ni exceder el límite de longitud configurado (`MAX_MESSAGE_LENGTH`).

#### A) Escenario de éxito

```bash
curl -i -X POST http://127.0.0.1:3000/api/sms/send \
  -H "Content-Type: application/json" \
  -d '{"recipient": "+524421234567", "message": "Hola, tu código de verificación es 9942"}'
```

**Respuesta (200 OK):**
```json
{
  "message_id": "msg_a1b2c3d4-e5f6-4789-a0b1-c2d3e4f5a6b7",
  "status": "queued",
  "recipient": "+524421234567",
  "classification": "LEGITIMO"
}
```

#### B) Escenario de bloqueo (phishing detectado por la IA)

```bash
curl -i -X POST http://127.0.0.1:3000/api/sms/send \
  -H "Content-Type: application/json" \
  -d '{"recipient": "+524421234567", "message": "Felicidades, ganaste un iPhone 15, haz clic en este enlace para reclamarlo."}'
```

**Respuesta (400 Bad Request):**
```json
{
  "status": "error",
  "message": "Mensaje rechazado por políticas de seguridad de red (Clasificación: PHISHING)"
}
```

#### C) Validación fallida (formato de número inválido)

```bash
curl -i -X POST http://127.0.0.1:3000/api/sms/send \
  -H "Content-Type: application/json" \
  -d '{"recipient": "5215500000000", "message": "Hola"}'
```

**Respuesta (400 Bad Request):**
```json
{
  "status": "error",
  "message": "Formato inválido. Usa formato E.164 con código de país (ej. +524421234567)."
}
```

## Flujo del agente de IA

1. La API recibe el payload JSON y valida las reglas de negocio base (formato E.164 del destinatario, longitud y contenido del mensaje).
2. El texto del mensaje se sanitiza (`sanitize_message`) y se inyecta en un prompt que delimita el mensaje como datos, indicándole al modelo que ignore cualquier instrucción contenida dentro de él.
3. `ai::evaluate_sms_spam` llama al endpoint `generateContent` de Gemini (API key vía header `x-goog-api-key`, no en la URL) usando un cliente HTTP reutilizado con timeout de 10s.
4. La API de Gemini está configurada con `responseSchema` para devolver únicamente JSON estructurado con un campo `classification` restringido al enum `LEGITIMO | SPAM | PHISHING` — el modelo no puede responder texto libre.
5. Si la clasificación es `SPAM`, `PHISHING`, o si el agente de IA falla por cualquier motivo (timeout, error HTTP, respuesta no parseable → `ERROR_IA`), la solicitud se **rechaza** (fail-closed) con `400 Bad Request`, y el motivo queda registrado en los logs.
6. Solo si la clasificación es `LEGITIMO`, el mensaje se encola con un `message_id` único generado con UUID v4.

## Hardening de seguridad aplicado

Este proyecto empezó como una PoC funcional y pasó por una segunda iteración enfocada en cerrar riesgos típicos de un gateway que depende de un LLM como capa de decisión:

| # | Mejora | Problema que resuelve |
|---|--------|------------------------|
| 1 | **Fail-closed en `ERROR_IA`** | Antes, si el agente de IA fallaba, el mensaje pasaba igual — un atacante podía saturar la API de Gemini para hacer *bypass* del filtro. |
| 2 | **Cliente HTTP reutilizado + timeout** | Evita reconstruir la conexión TLS en cada request y evita que un Gemini colgado deje el handler esperando indefinidamente. |
| 3 | **API key vía header (`x-goog-api-key`)** | La key ya no viaja en la URL, donde podía quedar expuesta en logs de proxies o del servidor. |
| 4 | **Validación E.164 + límite de longitud del mensaje** | El validador anterior solo exigía que el número empezara con `+`; ahora se valida formato real y se evita mandar payloads arbitrariamente grandes a la IA. |
| 5 | **`message_id` con UUID v4** | El ID anterior se basaba en timestamp por segundo — dos requests en el mismo segundo generaban colisiones. |
| 6 | **Structured output (`responseSchema`)** | El parsing anterior confiaba en que el modelo respondiera *exactamente* una palabra; ahora la respuesta está forzada a un enum fijo vía la API de Gemini, eliminando ambigüedad de parsing. |
| 7 | **Delimitadores + sanitización contra prompt injection** | El mensaje del usuario se trata explícitamente como datos (no instrucciones) y se sanitiza para no romper los delimitadores del prompt. |

## Tests

El proyecto incluye una prueba unitaria de deserialización en `models.rs`:

```bash
cargo test
```

## Notas y limitaciones (PoC)

- El envío de SMS es **simulado**: no hay integración real con un proveedor (Twilio, etc.), solo se genera un `message_id` con UUID y se responde `queued`.
- No hay persistencia (base de datos, cola real como SQS/RabbitMQ); todo vive en memoria durante la vida del proceso.
- El `responseSchema` reduce significativamente el riesgo de prompt injection, pero ningún prompt es 100% infalible — para un entorno productivo se recomienda además rate limiting y logging de auditoría de los mensajes bloqueados.
- Verifica que el nombre del modelo de Gemini configurado en `ai.rs` esté disponible para tu API key en Google AI Studio.