use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use libreplume_server::service::{AppState, Config};
use libreplume_server::store::Store;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn app(token: Option<&str>) -> axum::Router {
    let state = AppState {
        store: Arc::new(Store::in_memory()),
        config: Arc::new(Config { token: token.map(str::to_string), ..Config::default() }),
    };
    libreplume_server::app(state)
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req.header(header::CONTENT_TYPE, "application/json").body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn check_conjugate_and_settings() {
    let app = app(None);
    let (s, v) = call(&app, "GET", "/api/health", None, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["status"], "ok");

    let (s, v) = call(&app, "POST", "/api/check", Some(json!({ "text": "Il à mangé.", "language": "fr" })), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["matches"][0]["rule_id"], "FR_A_ACCENT");
    assert_eq!(v["matches"][0]["offset"], 3);

    let (s, v) = call(&app, "GET", "/api/conjugate/venir?language=fr", None, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["auxiliary"], "être");
    let (s, _) = call(&app, "GET", "/api/conjugate/table", None, None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    let (s, _) = call(&app, "PUT", "/api/rules/FR_A_ACCENT", Some(json!({ "enabled": false })), None).await;
    assert_eq!(s, StatusCode::OK);
    let (_, v) = call(&app, "POST", "/api/check", Some(json!({ "text": "Il à mangé.", "language": "fr" })), None).await;
    assert_eq!(v["matches"].as_array().unwrap().len(), 0);

    let (_, v) = call(&app, "POST", "/api/dictionary", Some(json!({ "word": "Zorglub" })), None).await;
    assert_eq!(v["words"], json!(["Zorglub"]));
    let (_, v) =
        call(&app, "POST", "/api/check", Some(json!({ "text": "Zorglub arrive.", "language": "fr" })), None).await;
    assert_eq!(v["matches"].as_array().unwrap().len(), 0);

    let (s, v) =
        call(&app, "PATCH", "/api/settings", Some(json!({ "default_language": "en", "max_sentence_words": 25 })), None)
            .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["default_language"], "en");
    let (_, v) = call(&app, "POST", "/api/correct", Some(json!({ "text": "He go home." })), None).await;
    assert_eq!(v["corrected"], "He goes home.");
}

#[tokio::test]
async fn themes_crud() {
    let app = app(None);
    let (_, v) = call(&app, "GET", "/api/themes", None, None).await;
    assert!(v["themes"].as_array().unwrap().len() >= 6);
    let theme = json!({ "id": "ocean", "name": "Océan", "dark": true, "variables": { "accent": "#0ea5e9" } });
    let (s, v) = call(&app, "PUT", "/api/themes/ocean", Some(theme), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (s, _) = call(&app, "PATCH", "/api/settings", Some(json!({ "default_theme": "ocean" })), None).await;
    assert_eq!(s, StatusCode::OK);
    let bad = json!({ "id": "evil", "name": "Evil", "variables": { "bg": "red;}" } });
    let (s, _) = call(&app, "PUT", "/api/themes/evil", Some(bad), None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = call(&app, "DELETE", "/api/themes/ocean", None, None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (_, v) = call(&app, "GET", "/api/settings", None, None).await;
    assert_eq!(v["default_theme"], "plume");
}

#[tokio::test]
async fn token_protects_changes_and_mcp() {
    let app = app(Some("secret"));
    let (s, _) = call(&app, "POST", "/api/check", Some(json!({ "text": "Bonjour." })), None).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = call(&app, "POST", "/api/dictionary", Some(json!({ "word": "x" })), None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(&app, "POST", "/api/dictionary", Some(json!({ "word": "x" })), Some("wrong")).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(&app, "POST", "/api/dictionary", Some(json!({ "word": "x" })), Some("secret")).await;
    assert_eq!(s, StatusCode::OK);
    let ping = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
    let (s, _) = call(&app, "POST", "/mcp", Some(ping.clone()), None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(&app, "POST", "/mcp", Some(ping), Some("secret")).await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn mcp_protocol() {
    let app = app(None);
    let init = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } } });
    let (s, v) = call(&app, "POST", "/mcp", Some(init), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(v["result"]["serverInfo"]["name"], "libreplume");

    let (s, _) =
        call(&app, "POST", "/mcp", Some(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })), None)
            .await;
    assert_eq!(s, StatusCode::ACCEPTED);

    let (_, v) =
        call(&app, "POST", "/mcp", Some(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })), None).await;
    let names: Vec<&str> =
        v["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"check_text") && names.contains(&"save_theme"));

    let call_tool = |id: i64, name: &str, args: Value| json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": name, "arguments": args } });
    let (_, v) = call(
        &app,
        "POST",
        "/mcp",
        Some(call_tool(3, "correct_text", json!({ "text": "Tu mange une pomme.", "language": "fr" }))),
        None,
    )
    .await;
    assert_eq!(v["result"]["structuredContent"]["corrected"], "Tu manges une pomme.");

    let (_, v) =
        call(&app, "POST", "/mcp", Some(call_tool(4, "add_dictionary_words", json!({ "words": ["Plumette"] }))), None)
            .await;
    assert_eq!(v["result"]["isError"], false);
    let (_, v) = call(
        &app,
        "POST",
        "/mcp",
        Some(call_tool(5, "check_text", json!({ "text": "Plumette arrive.", "language": "fr" }))),
        None,
    )
    .await;
    assert_eq!(v["result"]["structuredContent"]["issue_count"], 0);

    let (_, v) = call(
        &app,
        "POST",
        "/mcp",
        Some(call_tool(
            6,
            "save_theme",
            json!({ "id": "lagon", "name": "Lagon", "variables": { "accent": "#0ea5e9" } }),
        )),
        None,
    )
    .await;
    assert_eq!(v["result"]["structuredContent"]["variables"]["accent"], "#0ea5e9");
    assert_eq!(v["result"]["structuredContent"]["variables"]["bg"], "#f6f3ee");

    let (_, v) = call(
        &app,
        "POST",
        "/mcp",
        Some(call_tool(7, "conjugate_verb", json!({ "verb": "prendre", "tenses": ["present"] }))),
        None,
    )
    .await;
    assert_eq!(v["result"]["structuredContent"]["moods"][0]["tenses"][0]["forms"][0]["text"], "je prends");

    let (_, v) = call(
        &app,
        "POST",
        "/mcp",
        Some(call_tool(8, "set_rule_enabled", json!({ "rule_id": "NOPE", "enabled": true }))),
        None,
    )
    .await;
    assert_eq!(v["result"]["isError"], true);

    let (_, v) = call(&app, "POST", "/mcp", Some(json!({ "jsonrpc": "2.0", "id": 9, "method": "nope" })), None).await;
    assert_eq!(v["error"]["code"], -32601);
}

#[tokio::test]
async fn languagetool_compat() {
    let app = app(None);
    let req = Request::builder()
        .method("POST")
        .uri("/v2/check")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from("language=fr&text=Il%20%C3%A0%20mang%C3%A9."))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v: Value = serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(v["matches"][0]["rule"]["id"], "FR_A_ACCENT");
    assert_eq!(v["matches"][0]["replacements"][0]["value"], "a");
}

#[tokio::test]
async fn serves_web_interface() {
    let app = app(None);
    let res = app.clone().oneshot(Request::get("/").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/html"));
    let res = app.clone().oneshot(Request::get("/conjugueur").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
