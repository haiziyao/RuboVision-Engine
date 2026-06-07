use super::model::{DebugBindingView, DebugTriggerRequest, DebugTriggerResponse, WebMessage};
use super::state::WebState;
use crate::embed::Assets;
use crate::source::{DebugSourceError, Event};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use log::{debug, info};

pub async fn index() -> impl IntoResponse {
    info!("Index getting started");
    match Assets::get("index.html") {
        Some(file) => {
            Html(String::from_utf8_lossy(file.data.as_ref()).into_owned()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn message(State(state): State<WebState>) -> impl IntoResponse {
    // info!("sending cached message");

    match state.latest().await {
        Some(msg) => Json(msg).into_response(),
        None => Json(WebMessage::empty()).into_response(),
    }
}

pub async fn push_message(
    State(state): State<WebState>,
    Json(msg): Json<WebMessage>,
) -> impl IntoResponse {
    info!("accepting pushed web message");
    state.push_message(msg).await;
    StatusCode::ACCEPTED.into_response()
}

pub async fn history(State(state): State<WebState>) -> impl IntoResponse {
    // info!("sending cached history");

    Json(state.history().await).into_response()
}

pub async fn debug_bindings(State(state): State<WebState>) -> impl IntoResponse {
    let Some(source) = state.debug_source() else {
        return Json(Vec::<DebugBindingView>::new()).into_response();
    };
    let bindings = source
        .bindings()
        .into_iter()
        .map(|binding| DebugBindingView {
            source_key: binding.source_key,
            task_id: binding.task_id,
            device_id: binding.device_id,
            function_id: binding.function_id,
        })
        .collect::<Vec<_>>();

    Json(bindings).into_response()
}

pub async fn debug_trigger(
    State(state): State<WebState>,
    Json(request): Json<DebugTriggerRequest>,
) -> impl IntoResponse {
    let Some(source) = state.debug_source() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "debug source is unavailable",
        )
            .into_response();
    };

    match source.trigger(request.source_key.trim()).await {
        Ok(Event::UsualEvent(task_id, _, _)) => (
            StatusCode::ACCEPTED,
            Json(DebugTriggerResponse {
                accepted: true,
                task_id,
            }),
        )
            .into_response(),
        Ok(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "debug source produced an invalid event",
        )
            .into_response(),
        Err(DebugSourceError::UnknownSourceKey(_)) => {
            (StatusCode::NOT_FOUND, "unknown debug source key").into_response()
        }
        Err(DebugSourceError::EventChannelClosed) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "task event channel is closed",
        )
            .into_response(),
    }
}

pub async fn handle_404() -> impl IntoResponse {
    debug!("not found! 404 problem");
    (StatusCode::NOT_FOUND, "Not found")
}

#[cfg(test)]
mod tests {
    use axum::Json;
    use axum::body::to_bytes;
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use tokio::sync::mpsc;

    use crate::config::binding::DebugBinding;
    use crate::source::{DebugSource, Event};
    use crate::web::handler::{debug_bindings, debug_trigger};
    use crate::web::model::{DebugBindingView, DebugTriggerRequest, DebugTriggerResponse};
    use crate::web::state::WebState;

    fn binding(source_key: &str) -> DebugBinding {
        DebugBinding {
            task_id: "debug_color_detect".to_string(),
            source_key: source_key.to_string(),
            device_id: "color_camera".to_string(),
            function_id: "color_detect".to_string(),
        }
    }

    #[tokio::test]
    async fn debug_bindings_returns_public_metadata() {
        let (tx, _rx) = mpsc::channel(1);
        let state =
            WebState::in_memory(20).with_debug_source(DebugSource::new(vec![binding("color")], tx));

        let response = debug_bindings(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let bindings: Vec<DebugBindingView> =
            serde_json::from_slice(&body).expect("bindings response");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].source_key, "color");
        assert_eq!(bindings[0].task_id, "debug_color_detect");
        assert_eq!(bindings[0].device_id, "color_camera");
        assert_eq!(bindings[0].function_id, "color_detect");
    }

    #[tokio::test]
    async fn debug_trigger_maps_accept_not_found_and_unavailable_statuses() {
        let (tx, mut rx) = mpsc::channel(1);
        let state =
            WebState::in_memory(20).with_debug_source(DebugSource::new(vec![binding("color")], tx));

        let accepted = debug_trigger(
            State(state.clone()),
            Json(DebugTriggerRequest {
                source_key: "color".to_string(),
            }),
        )
        .await
        .into_response();
        assert_eq!(accepted.status(), StatusCode::ACCEPTED);
        let body = to_bytes(accepted.into_body(), usize::MAX)
            .await
            .expect("accepted body");
        let accepted: DebugTriggerResponse =
            serde_json::from_slice(&body).expect("accepted response");
        assert!(accepted.accepted);
        assert_eq!(accepted.task_id, "debug_color_detect");
        assert!(matches!(rx.recv().await, Some(Event::UsualEvent(..))));

        let missing = debug_trigger(
            State(state),
            Json(DebugTriggerRequest {
                source_key: "missing".to_string(),
            }),
        )
        .await
        .into_response();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);

        let (closed_tx, closed_rx) = mpsc::channel(1);
        drop(closed_rx);
        let closed_state = WebState::in_memory(20)
            .with_debug_source(DebugSource::new(vec![binding("color")], closed_tx));
        let unavailable = debug_trigger(
            State(closed_state),
            Json(DebugTriggerRequest {
                source_key: "color".to_string(),
            }),
        )
        .await
        .into_response();
        assert_eq!(unavailable.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
