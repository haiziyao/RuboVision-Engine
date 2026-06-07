use super::handler::*;
use super::state::WebState;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

pub fn router(state: WebState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/message", get(message).post(push_message))
        .route("/history", get(history))
        .route("/debug/bindings", get(debug_bindings))
        .route("/debug/trigger", post(debug_trigger))
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        .fallback(handle_404)
        .with_state(state)
}
