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

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn value(text: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            code: 200,
            text: text.into(),
            value: Some(value.into()),
            image: None,
        }
    }

    pub fn value_with_image(
        text: impl Into<String>,
        value: impl Into<String>,
        image: impl Into<String>,
    ) -> Self {
        Self {
            code: 200,
            text: text.into(),
            value: Some(value.into()),
            image: Some(image.into()),
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

#[cfg(test)]
mod tests {
    use super::TaskOutput;

    #[test]
    fn value_with_image_sets_value_and_web_image_payload() {
        let output = TaskOutput::value_with_image(
            "color_detect finished: red",
            "red",
            "data:image/jpeg;base64,abc",
        );

        assert_eq!(output.code, 200);
        assert_eq!(output.text, "color_detect finished: red");
        assert_eq!(output.value.as_deref(), Some("red"));
        assert_eq!(output.image.as_deref(), Some("data:image/jpeg;base64,abc"));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpioOutput {
    TaskStarted(String),
    TaskFinished(String),
    Reset,
}
