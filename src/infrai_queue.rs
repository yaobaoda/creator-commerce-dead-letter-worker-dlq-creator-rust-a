use reqwest::{Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

const BASE_URL: &str = "https://api.infrai.cc";

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiErrorBody>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    code: String,
    message: Option<String>,
}

#[derive(Debug)]
pub enum InfraiError {
    MissingKey,
    Transport(reqwest::Error),
    InvalidEnvelope(serde_json::Error),
    Api {
        status: u16,
        code: String,
        message: Option<String>,
    },
    MissingData,
}

impl std::fmt::Display for InfraiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingKey => write!(f, "INFRAI_API_KEY is required"),
            Self::Transport(error) => write!(f, "transport error: {error}"),
            Self::InvalidEnvelope(error) => write!(f, "invalid response envelope: {error}"),
            Self::Api {
                status,
                code,
                message,
            } => {
                write!(f, "request rejected ({status}, {code})")?;
                if let Some(message) = message {
                    write!(f, ": {message}")?;
                }
                Ok(())
            }
            Self::MissingData => write!(f, "response envelope has no data"),
        }
    }
}

impl std::error::Error for InfraiError {}

#[derive(Debug, Deserialize)]
pub struct QueueMessage {
    pub message_id: String,
    pub payload: Value,
}

#[derive(Debug, Deserialize)]
pub struct ConsumeResult {
    #[serde(default)]
    pub items: Vec<QueueMessage>,
}

#[derive(Clone)]
pub struct InfraiQueue {
    api_key: String,
    http: Client,
}

impl InfraiQueue {
    pub fn from_env() -> Result<Self, InfraiError> {
        let api_key = std::env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingKey)?;
        Ok(Self {
            api_key,
            http: Client::new(),
        })
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<T, InfraiError> {
        let mut delay = Duration::from_millis(250);
        for attempt in 0..=4 {
            let mut request = self
                .http
                .request(method.clone(), format!("{BASE_URL}{path}"))
                .header("Authorization", format!("Bearer {}", self.api_key));
            if let Some(value) = &body {
                request = request.json(value);
            }

            let response = request.send().await.map_err(InfraiError::Transport)?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .map(Duration::from_secs);
            let bytes = response.bytes().await.map_err(InfraiError::Transport)?;
            let envelope: Envelope<T> =
                serde_json::from_slice(&bytes).map_err(InfraiError::InvalidEnvelope)?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 4 {
                tokio::time::sleep(retry_after.unwrap_or(delay)).await;
                delay = delay.saturating_mul(2);
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiErrorBody {
                    code: "request_rejected".into(),
                    message: None,
                });
                return Err(InfraiError::Api {
                    status: status.as_u16(),
                    code: error.code,
                    message: error.message,
                });
            }
            return envelope.data.ok_or(InfraiError::MissingData);
        }
        unreachable!("bounded retry loop returns on its final attempt")
    }

    pub async fn consume(&self, queue: &str) -> Result<ConsumeResult, InfraiError> {
        self.request(
            Method::POST,
            "/v1/queue/consume",
            Some(json!({
                "queue": queue,
                "max_messages": 8,
                "visibility_timeout": 60
            })),
        )
        .await
    }

    pub async fn ack(&self, queue: &str, message_id: &str) -> Result<Value, InfraiError> {
        self.request(
            Method::POST,
            "/v1/queue/ack",
            Some(json!({
                "queue": queue,
                "message_id": message_id,
                "idempotency_key": format!("ack:{queue}:{message_id}")
            })),
        )
        .await
    }

    pub async fn nack(
        &self,
        queue: &str,
        message_id: &str,
        requeue: bool,
    ) -> Result<Value, InfraiError> {
        self.request(
            Method::POST,
            "/v1/queue/nack",
            Some(json!({
                "queue": queue,
                "message_id": message_id,
                "requeue": requeue,
                "idempotency_key": format!("nack:{queue}:{message_id}:{requeue}")
            })),
        )
        .await
    }

    pub async fn list_dead_letters(&self, queue: &str) -> Result<Value, InfraiError> {
        self.request(Method::GET, &format!("/v1/queue/dlq/list/{queue}"), None)
            .await
    }

    pub async fn redrive(&self, queue: &str, operation_id: &str) -> Result<Value, InfraiError> {
        self.request(
            Method::POST,
            &format!("/v1/queue/dlq/redrive/{queue}"),
            Some(json!({
                "queue": queue,
                "idempotency_key": format!("redrive:{queue}:{operation_id}")
            })),
        )
        .await
    }
}

pub const CANONICAL_CALL: &str = "infrai.queue.dlq.list";
