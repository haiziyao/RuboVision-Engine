use tracing::{info,debug};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebMessage {
    pub code: u16,
    pub text: String,
    pub image: Option<String>,
}



impl WebMessage {
    pub fn ok(text: impl Into<String>) -> Self {
        info!("this is a simple web message");
        Self {
            code: 200,
            text: text.into(),
            image: None,
        }
    }

    pub fn with_image(text: impl Into<String>, image: impl Into<String>) -> Self {
        info!("this is a imaged web message");
        Self {
            code: 200,
            text: text.into(),
            image: Some(image.into()),
        }
    }

    pub fn closed() -> Self {
        info!("this is a closed web message");
        Self {
            code: 503,
            text: "message channel closed".to_string(),
            image: None,
        }
    }

    pub fn empty() -> Self {
        debug!("this is a empty web message");
        Self {
            code: 204,
            text: "no new message".to_string(),
            image: None,
        }
    }
}
