use std::net::SocketAddr;

use axum::{
    Router,
    routing::{get, post, put},
};

use crate::{
    WebError, WebState,
    web::api::{
        config, config_bindings, config_devices, config_functions, config_save, config_sinks,
        config_sources, config_validate, debug_bindings, debug_trigger, health, index, interface,
        output_detail, outputs, outputs_latest, profile_status, remove_source_api, runtime_chain,
        runtime_control_status, runtime_events, runtime_outputs_latest, runtime_restart,
        runtime_start, runtime_stop, runtime_summary, update_binding_api, update_device_api,
        update_function_api, update_profile, update_sink_api, update_source_api,
    },
};

pub fn build_router(state: WebState) -> Router {
    let routes = state.config().routes().clone();
    Router::new()
        .route("/", get(index))
        .route(routes.interface(), get(interface))
        .route(routes.health(), get(health))
        .route(routes.runtime_summary(), get(runtime_summary))
        .route(routes.runtime_chain(), get(runtime_chain))
        .route(routes.runtime_outputs_latest(), get(runtime_outputs_latest))
        .route(routes.runtime_events(), get(runtime_events))
        .route(routes.runtime_start(), post(runtime_start))
        .route(routes.runtime_stop(), post(runtime_stop))
        .route(routes.runtime_restart(), post(runtime_restart))
        .route(routes.runtime_control_status(), get(runtime_control_status))
        .route(routes.outputs(), get(outputs))
        .route(routes.outputs_latest(), get(outputs_latest))
        .route(routes.output_detail(), get(output_detail))
        .route(routes.config(), get(config))
        .route(routes.config_sources(), get(config_sources))
        .route(routes.config_devices(), get(config_devices))
        .route(routes.config_functions(), get(config_functions))
        .route(routes.config_sinks(), get(config_sinks))
        .route(routes.config_bindings(), get(config_bindings))
        .route(
            routes.config_profile(),
            get(profile_status).put(update_profile),
        )
        .route(
            &format!("{}/{{id}}", routes.config_sources()),
            put(update_source_api).delete(remove_source_api),
        )
        .route(
            &format!("{}/{{id}}", routes.config_devices()),
            put(update_device_api),
        )
        .route(
            &format!("{}/{{id}}", routes.config_functions()),
            put(update_function_api),
        )
        .route(
            &format!("{}/{{id}}", routes.config_sinks()),
            put(update_sink_api),
        )
        .route(
            &format!("{}/{{id}}", routes.config_bindings()),
            put(update_binding_api),
        )
        .route(routes.config_validate(), post(config_validate))
        .route(routes.config_save(), post(config_save))
        .route(routes.debug_bindings(), get(debug_bindings))
        .route(routes.debug_trigger(), post(debug_trigger))
        .with_state(state)
}

pub async fn serve(state: WebState) -> Result<(), WebError> {
    let address: SocketAddr = state
        .config()
        .address()
        .parse()
        .map_err(|error| WebError::invalid_request(format!("invalid web address: {error}")))?;
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|error| WebError::runtime(format!("failed to bind web server: {error}")))?;
    axum::serve(listener, build_router(state))
        .await
        .map_err(|error| WebError::internal(format!("web server failed: {error}")))
}
