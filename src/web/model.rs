use tracing::{debug, info};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebMessage {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub created_at_ms: u64,
    pub code: u16,
    pub text: String,
    pub image: Option<String>,
}

impl WebMessage {
    #[allow(dead_code)]
    pub fn ok(text: impl Into<String>) -> Self {
        info!("this is a simple web message");
        Self {
            id: 0,
            created_at_ms: 0,
            code: 200,
            text: text.into(),
            image: None,
        }
    }

    #[allow(dead_code)]
    pub fn error(text: impl Into<String>) -> Self {
        info!("this is a error web message");
        Self {
            id: 0,
            created_at_ms: 0,
            code: 500,
            text: text.into(),
            image: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_image(text: impl Into<String>, image: impl Into<String>) -> Self {
        info!("this is a imaged web message");
        Self {
            id: 0,
            created_at_ms: 0,
            code: 200,
            text: text.into(),
            image: Some(image.into()),
        }
    }

    #[allow(dead_code)]
    pub fn closed() -> Self {
        info!("this is a closed web message");
        Self {
            id: 0,
            created_at_ms: 0,
            code: 503,
            text: "message channel closed".to_string(),
            image: None,
        }
    }

    pub fn empty() -> Self {
        debug!("this is a empty web message");
        Self {
            id: 0,
            created_at_ms: 0,
            code: 204,
            text: "no new message".to_string(),
            image: None,
        }
    }

    pub fn with_runtime_meta(mut self, id: u64, created_at_ms: u64) -> Self {
        self.id = id;
        self.created_at_ms = created_at_ms;
        self
    }
}
