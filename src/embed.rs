use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static/"]
pub struct Assets;

#[cfg(test)]
mod tests {
    use super::Assets;

    #[test]
    fn web_page_contains_message_history_and_debug_controls() {
        let page = Assets::get("index.html").expect("embedded index page");
        let page = std::str::from_utf8(page.data.as_ref()).expect("index page is utf-8");

        for contract in [
            "/message",
            "/history",
            "/debug/bindings",
            "/debug/trigger",
            "start-polling",
            "clear-history",
            "debug-task-list",
        ] {
            assert!(
                page.contains(contract),
                "missing page contract `{contract}`"
            );
        }
    }
}
