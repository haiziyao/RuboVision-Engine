use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Json;
use log::{debug, info};
use super::model::WebMessage;
use super::state::WebState;
use crate::embed::Assets;


pub async fn index() -> impl IntoResponse {
    info!("Index getting started");
    match Assets::get("index.html") {
        Some(file) => Html(String::from_utf8_lossy(file.data.as_ref()).into_owned()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}


pub async fn message(State(state): State<WebState>) -> impl IntoResponse {
    let mut rx = state.rx.lock().await;
    info!("sending message");
    tokio::select! {
        msg = rx.recv() => {
            match msg {
                Some(msg) => Json(msg).into_response(),
                None => Json(WebMessage::closed()).into_response(),
            }
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
            Json(WebMessage::empty()).into_response()
        }
    }
}

pub async fn handle_404() -> impl IntoResponse {
    debug!("not found! 404 problem");
    (StatusCode::NOT_FOUND, "Not found")
}
