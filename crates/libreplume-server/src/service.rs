//! Operations shared by the REST API and the MCP server.

use crate::store::{RequestOverrides, Store, StoreError};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use libreplume_core::{Category, CheckResult, Conjugation, Correction, Language, RuleInfo};
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub config: Arc<Config>,
}

#[derive(Debug, Clone)]
pub struct Config {
    /// When set, write operations and the MCP endpoint require this bearer token.
    pub token: Option<String>,
    /// Maximum number of characters accepted for a check.
    pub max_text_chars: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config { token: None, max_text_chars: 200_000 }
    }
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> ApiError {
        ApiError { status: StatusCode::BAD_REQUEST, message: msg.into() }
    }

    pub fn not_found(msg: impl Into<String>) -> ApiError {
        ApiError { status: StatusCode::NOT_FOUND, message: msg.into() }
    }
}

impl From<StoreError> for ApiError {
    fn from(e: StoreError) -> Self {
        let status = match e {
            StoreError::NotFound(_) => StatusCode::NOT_FOUND,
            StoreError::Invalid(_) => StatusCode::BAD_REQUEST,
            StoreError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        ApiError { status, message: e.to_string() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

pub fn parse_language(s: Option<&str>) -> Result<Option<Language>, ApiError> {
    match s.map(str::trim) {
        None | Some("") | Some("auto") => Ok(None),
        Some(l) => l.parse().map(Some).map_err(ApiError::bad_request),
    }
}

fn validate_text(state: &AppState, text: &str) -> Result<(), ApiError> {
    if text.chars().count() > state.config.max_text_chars {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            message: format!("text is longer than {} characters", state.config.max_text_chars),
        });
    }
    Ok(())
}

pub async fn check(state: &AppState, text: String, overrides: RequestOverrides) -> Result<CheckResult, ApiError> {
    validate_text(state, &text)?;
    let options = state.store.check_options(&overrides);
    tokio::task::spawn_blocking(move || libreplume_core::check(&text, &options))
        .await
        .map_err(|e| ApiError { status: StatusCode::INTERNAL_SERVER_ERROR, message: e.to_string() })
}

pub async fn correct(
    state: &AppState,
    text: String,
    overrides: RequestOverrides,
    categories: Option<Vec<Category>>,
) -> Result<Correction, ApiError> {
    validate_text(state, &text)?;
    let options = state.store.check_options(&RequestOverrides { categories: None, ..overrides });
    let cats: Option<HashSet<Category>> = categories.map(|c| c.into_iter().collect());
    tokio::task::spawn_blocking(move || libreplume_core::correct(&text, &options, cats.as_ref()))
        .await
        .map_err(|e| ApiError { status: StatusCode::INTERNAL_SERVER_ERROR, message: e.to_string() })
}

pub async fn conjugate(verb: String, language: Option<Language>) -> Result<Conjugation, ApiError> {
    if verb.chars().count() > 60 {
        return Err(ApiError::bad_request("verb is too long"));
    }
    let lang = language.unwrap_or_else(|| libreplume_core::detect_language(&verb));
    tokio::task::spawn_blocking(move || libreplume_core::conjugate(&verb, lang))
        .await
        .map_err(|e| ApiError { status: StatusCode::INTERNAL_SERVER_ERROR, message: e.to_string() })?
        .map_err(|e| ApiError::not_found(e.to_string()))
}

#[derive(Debug, Serialize)]
pub struct RuleView {
    #[serde(flatten)]
    pub info: RuleInfo,
    pub enabled: bool,
}

pub fn rules(store: &Store, language: Option<Language>, category: Option<Category>) -> Vec<RuleView> {
    let settings = store.settings();
    libreplume_core::list_rules(language)
        .into_iter()
        .filter(|r| category.is_none_or(|c| r.category == c))
        .map(|info| {
            let enabled = !settings.disabled_categories.contains(&info.category)
                && Store::rule_enabled(&settings, info.id, info.enabled_by_default);
            RuleView { info, enabled }
        })
        .collect()
}

pub fn info(state: &AppState) -> serde_json::Value {
    json!({
        "name": "LibrePlume",
        "version": libreplume_core::VERSION,
        "languages": Language::ALL.iter().map(|l| json!({ "code": l.code(), "name": l.name() })).collect::<Vec<_>>(),
        "categories": Category::ALL.iter().map(|c| c.code()).collect::<Vec<_>>(),
        "rules": libreplume_core::list_rules(None).len(),
        "dictionary_words": state.store.dictionary().len(),
        "auth_required_for_changes": state.config.token.is_some(),
        "max_text_chars": state.config.max_text_chars,
        "endpoints": {
            "rest": "/api",
            "mcp": "/mcp",
            "languagetool": "/v2/check",
        },
    })
}
