use owo_colors::OwoColorize;

pub fn text(message: impl AsRef<str>) -> String {
    message.as_ref().to_string()
}

pub fn error_text(message: impl AsRef<str>) -> String {
    message.as_ref().red().to_string()
}

pub fn hidden_text(message: impl AsRef<str>) -> String {
    message.as_ref().purple().to_string()
}

pub fn warn_text(message: impl AsRef<str>) -> String {
    message.as_ref().yellow().to_string()
}

pub fn hint_text(message: impl AsRef<str>) -> String {
    message.as_ref().cyan().to_string()
}

pub fn success_text(message: impl AsRef<str>) -> String {
    message.as_ref().green().to_string()
}
