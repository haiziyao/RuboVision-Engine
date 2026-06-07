#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskOutput {
    pub code: u16,
    pub text: String,
    pub value: Option<String>,
    pub image: Option<String>,
}

impl TaskOutput {
    pub fn ok(text: impl Into<String>) -> Self {
        Self {
            code: 200,
            text: text.into(),
            value: None,
            image: None,
        }
    }

    pub fn value(text: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            code: 200,
            text: text.into(),
            value: Some(value.into()),
            image: None,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            code: 500,
            text: text.into(),
            value: None,
            image: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpioOutput {
    TaskStarted(String),
    TaskFinished(String),
    Reset,
}
