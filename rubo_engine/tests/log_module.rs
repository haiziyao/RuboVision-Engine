use rubo_engine::log::{error_text, hidden_text, hint_text, success_text, text, warn_text};

#[test]
fn log_color_helpers_wrap_messages_with_ansi_sequences() {
    assert!(text("normal").contains("normal"));
    assert!(error_text("error").contains("\u{1b}[31m"));
    assert!(hidden_text("auto").contains("\u{1b}[35m"));
    assert!(warn_text("warning").contains("\u{1b}[33m"));
    assert!(hint_text("hint").contains("\u{1b}[36m"));
    assert!(success_text("important").contains("\u{1b}[32m"));
}
