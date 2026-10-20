//! LibrePlume server: REST API, MCP endpoint and embedded web interface.

pub mod api;
pub mod mcp;
pub mod service;
pub mod store;
pub mod themes;
pub mod web;

use axum::http::{header, HeaderValue, Method};
use axum::routing::post;
use axum::Router;
use service::AppState;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

/// Builds the complete application router.
pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            "x-api-key".parse().unwrap(),
            "mcp-session-id".parse().unwrap(),
            "mcp-protocol-version".parse().unwrap(),
        ])
        .expose_headers(["mcp-session-id".parse().unwrap()]);
    Router::new()
        .merge(api::router())
        .route("/mcp", post(mcp::http_post).get(mcp::http_get).delete(mcp::http_delete))
        .fallback(web::serve)
        .layer(axum::middleware::from_fn_with_state(state.clone(), api::auth))
        .layer(RequestBodyLimitLayer::new(4 * 1024 * 1024))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer")))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
