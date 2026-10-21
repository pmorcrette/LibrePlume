//! REST API and LanguageTool-compatible endpoints.

use crate::service::{self, parse_language, ApiError, AppState};
use crate::store::{RequestOverrides, SettingsPatch};
use crate::themes::{Theme, THEME_VARIABLES};
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Form, Json, Router};
use libreplume_core::{Category, Language};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/info", get(info))
        .route("/api/check", post(check))
        .route("/api/correct", post(correct))
        .route("/api/conjugate", post(conjugate_post))
        .route("/api/conjugate/{verb}", get(conjugate_get))
        .route("/api/synonyms/{word}", get(synonyms))
        .route("/api/rules", get(rules))
        .route("/api/rules/{id}", put(set_rule))
        .route("/api/settings", get(get_settings).put(put_settings).patch(put_settings))
        .route("/api/dictionary", get(get_dictionary).post(add_words).delete(remove_words))
        .route("/api/dictionary/{word}", axum::routing::delete(remove_word))
        .route("/api/themes", get(list_themes).post(save_theme_post))
        .route("/api/themes/variables", get(theme_variables))
        .route("/api/themes/{id}", get(get_theme).put(save_theme).delete(delete_theme))
        .route("/v2/check", post(lt_check))
        .route("/v2/languages", get(lt_languages))
}

/// Requires the bearer token (when configured) for every request that changes state.
pub async fn auth(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let Some(token) = &state.config.token else {
        return next.run(req).await;
    };
    let path = req.uri().path();
    let writes = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    let read_only_post = matches!(path, "/api/check" | "/api/correct" | "/api/conjugate" | "/v2/check");
    let protected = path.starts_with("/mcp") || (writes && !read_only_post);
    if protected && !token_matches(req.headers(), token) {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            Json(json!({ "error": "a valid bearer token is required" })),
        )
            .into_response();
    }
    next.run(req).await
}

fn token_matches(headers: &HeaderMap, token: &str) -> bool {
    let given = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")))
        .or_else(|| headers.get("x-api-key").and_then(|v| v.to_str().ok()));
    given.is_some_and(|g| constant_time_eq(g.trim().as_bytes(), token.as_bytes()))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "version": libreplume_core::VERSION }))
}

async fn info(State(state): State<AppState>) -> Json<Value> {
    Json(service::info(&state))
}

#[derive(Deserialize)]
struct CheckBody {
    text: String,
    language: Option<String>,
    #[serde(flatten)]
    overrides: RequestOverrides,
}

async fn check(State(state): State<AppState>, Json(body): Json<CheckBody>) -> Result<Response, ApiError> {
    let mut overrides = body.overrides;
    overrides.language = parse_language(body.language.as_deref())?;
    Ok(Json(service::check(&state, body.text, overrides).await?).into_response())
}

#[derive(Deserialize)]
struct CorrectBody {
    text: String,
    language: Option<String>,
    /// Only apply corrections from these categories.
    categories: Option<Vec<Category>>,
}

async fn correct(State(state): State<AppState>, Json(body): Json<CorrectBody>) -> Result<Response, ApiError> {
    let overrides = RequestOverrides { language: parse_language(body.language.as_deref())?, ..Default::default() };
    Ok(Json(service::correct(&state, body.text, overrides, body.categories).await?).into_response())
}

#[derive(Deserialize)]
struct LangQuery {
    language: Option<String>,
    category: Option<String>,
}

#[derive(Deserialize)]
struct ConjugateBody {
    verb: String,
    language: Option<String>,
}

async fn conjugate_get(Path(verb): Path<String>, Query(q): Query<LangQuery>) -> Result<Response, ApiError> {
    let lang = parse_language(q.language.as_deref())?.or(Some(Language::Fr));
    Ok(Json(service::conjugate(verb, lang).await?).into_response())
}

async fn synonyms(
    State(state): State<AppState>,
    Path(word): Path<String>,
    Query(q): Query<LangQuery>,
) -> Result<Response, ApiError> {
    let lang = parse_language(q.language.as_deref())?;
    Ok(Json(service::synonyms(&state.store, word, lang).await?).into_response())
}

async fn conjugate_post(Json(body): Json<ConjugateBody>) -> Result<Response, ApiError> {
    let lang = parse_language(body.language.as_deref())?.or(Some(Language::Fr));
    Ok(Json(service::conjugate(body.verb, lang).await?).into_response())
}

async fn rules(State(state): State<AppState>, Query(q): Query<LangQuery>) -> Result<Response, ApiError> {
    let lang = parse_language(q.language.as_deref())?;
    let category = q.category.as_deref().map(str::parse::<Category>).transpose().map_err(ApiError::bad_request)?;
    Ok(Json(service::rules(&state.store, lang, category)).into_response())
}

#[derive(Deserialize)]
struct RuleToggle {
    enabled: bool,
}

async fn set_rule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RuleToggle>,
) -> Result<Response, ApiError> {
    Ok(Json(state.store.set_rule(&id, body.enabled)?).into_response())
}

async fn get_settings(State(state): State<AppState>) -> Json<Value> {
    Json(json!(state.store.settings()))
}

async fn put_settings(State(state): State<AppState>, Json(patch): Json<SettingsPatch>) -> Result<Response, ApiError> {
    Ok(Json(state.store.update_settings(patch)?).into_response())
}

async fn get_dictionary(State(state): State<AppState>) -> Json<Value> {
    Json(json!({ "words": state.store.dictionary() }))
}

#[derive(Deserialize)]
struct WordsBody {
    #[serde(default)]
    word: Option<String>,
    #[serde(default)]
    words: Vec<String>,
}

impl WordsBody {
    fn all(self) -> Vec<String> {
        self.words.into_iter().chain(self.word).collect()
    }
}

async fn add_words(State(state): State<AppState>, Json(body): Json<WordsBody>) -> Result<Response, ApiError> {
    let words = body.all();
    if words.is_empty() {
        return Err(ApiError::bad_request("give « word » or « words »"));
    }
    Ok(Json(json!({ "words": state.store.add_words(&words)? })).into_response())
}

async fn remove_words(State(state): State<AppState>, Json(body): Json<WordsBody>) -> Result<Response, ApiError> {
    Ok(Json(json!({ "words": state.store.remove_words(&body.all())? })).into_response())
}

async fn remove_word(State(state): State<AppState>, Path(word): Path<String>) -> Result<Response, ApiError> {
    Ok(Json(json!({ "words": state.store.remove_words(&[word])? })).into_response())
}

async fn list_themes(State(state): State<AppState>) -> Json<Value> {
    Json(json!({ "default": state.store.settings().default_theme, "themes": state.store.themes() }))
}

async fn theme_variables() -> Json<Value> {
    Json(json!(THEME_VARIABLES.iter().map(|(n, d)| json!({ "name": n, "description": d })).collect::<Vec<_>>()))
}

async fn get_theme(State(state): State<AppState>, Path(id): Path<String>) -> Result<Response, ApiError> {
    let theme = state.store.theme(&id).ok_or_else(|| ApiError::not_found(format!("theme {id}")))?;
    Ok(Json(theme).into_response())
}

async fn save_theme(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(mut theme): Json<Theme>,
) -> Result<Response, ApiError> {
    theme.id = id;
    Ok(Json(state.store.save_theme(theme)?).into_response())
}

async fn save_theme_post(State(state): State<AppState>, Json(theme): Json<Theme>) -> Result<Response, ApiError> {
    Ok((StatusCode::CREATED, Json(state.store.save_theme(theme)?)).into_response())
}

async fn delete_theme(State(state): State<AppState>, Path(id): Path<String>) -> Result<Response, ApiError> {
    state.store.delete_theme(&id)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---------------------------------------------------------------------------
// LanguageTool-compatible API, so existing LanguageTool clients can use LibrePlume.

#[derive(Deserialize)]
struct LtForm {
    text: Option<String>,
    data: Option<String>,
    language: Option<String>,
    #[serde(rename = "disabledRules")]
    disabled_rules: Option<String>,
    #[serde(rename = "enabledRules")]
    enabled_rules: Option<String>,
}

fn lt_language(code: &str) -> &'static str {
    match code {
        "en" => "en-US",
        _ => "fr",
    }
}

async fn lt_check(State(state): State<AppState>, Form(form): Form<LtForm>) -> Result<Response, ApiError> {
    let text = match (form.text, form.data) {
        (Some(t), _) => t,
        (None, Some(data)) => lt_data_to_text(&data)?,
        (None, None) => return Err(ApiError::bad_request("missing « text » or « data »")),
    };
    let language = parse_language(form.language.as_deref())?;
    let split = |s: Option<String>| -> Vec<String> {
        s.map(|s| s.split(',').map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).collect()).unwrap_or_default()
    };
    let overrides = RequestOverrides {
        language,
        disabled_rules: split(form.disabled_rules),
        enabled_rules: split(form.enabled_rules),
        ..Default::default()
    };
    let result = service::check(&state, text, overrides).await?;
    let code = result.language.code();
    let matches: Vec<Value> = result
        .matches
        .iter()
        .map(|m| {
            json!({
                "message": m.message,
                "shortMessage": "",
                "replacements": m.replacements.iter().map(|r| json!({ "value": r })).collect::<Vec<_>>(),
                "offset": m.offset,
                "length": m.length,
                "context": { "text": m.sentence, "offset": m.sentence.find(&m.text).unwrap_or(0), "length": m.length },
                "sentence": m.sentence,
                "type": { "typeName": if m.category == Category::Spelling { "UnknownWord" } else { "Other" } },
                "rule": {
                    "id": m.rule_id,
                    "description": m.message,
                    "issueType": match m.category {
                        Category::Spelling => "misspelling",
                        Category::Style => "style",
                        Category::Typography | Category::Punctuation => "typographical",
                        _ => "grammar",
                    },
                    "category": { "id": m.category.code().to_uppercase(), "name": m.category.code() },
                },
                "ignoreForIncompleteSentence": false,
                "contextForSureMatch": 0,
            })
        })
        .collect();
    Ok(Json(json!({
        "software": {
            "name": "LibrePlume",
            "version": libreplume_core::VERSION,
            "buildDate": "",
            "apiVersion": 1,
            "premium": false,
            "status": "",
        },
        "language": {
            "name": result.language.name(),
            "code": lt_language(code),
            "detectedLanguage": { "name": result.language.name(), "code": lt_language(code), "confidence": 0.9 },
        },
        "matches": matches,
    }))
    .into_response())
}

/// LanguageTool's `data` parameter: annotated text, markup is replaced by its interpretation or blanked out.
fn lt_data_to_text(data: &str) -> Result<String, ApiError> {
    let v: Value = serde_json::from_str(data).map_err(|e| ApiError::bad_request(format!("invalid data: {e}")))?;
    let mut text = String::new();
    for part in v["annotation"].as_array().into_iter().flatten() {
        if let Some(t) = part["text"].as_str() {
            text.push_str(t);
        } else if let Some(m) = part["markup"].as_str() {
            // Offsets must point into the original markup, so the markup is replaced by
            // its interpretation padded with spaces to the same length.
            let len = m.encode_utf16().count();
            let interp = part["interpretAs"].as_str().filter(|i| i.encode_utf16().count() <= len).unwrap_or("");
            text.push_str(interp);
            text.extend(std::iter::repeat_n(' ', len - interp.encode_utf16().count()));
        }
    }
    Ok(text)
}

async fn lt_languages() -> Json<Value> {
    Json(json!([
        { "name": "French", "code": "fr", "longCode": "fr" },
        { "name": "English (US)", "code": "en", "longCode": "en-US" },
    ]))
}
