use axum::Router;
use axum::routing::get;
use super::state::WebState;
use super::handler::*;

pub fn router(state: WebState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/message", get(message))
        .route("/history", get(history))
        .fallback(handle_404)
        .with_state(state)
}
