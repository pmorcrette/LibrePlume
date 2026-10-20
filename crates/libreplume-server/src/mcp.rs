//! Model Context Protocol server.
//!
//! The same JSON-RPC handler serves the Streamable HTTP transport (`POST /mcp`) and the
//! stdio transport (`libreplume mcp`). Tools let an AI agent check and correct text,
//! conjugate verbs and administer the instance (rules, dictionary, settings, themes).

use crate::service::{self, parse_language, ApiError, AppState};
use crate::store::{RequestOverrides, SettingsPatch};
use crate::themes::{Theme, THEME_VARIABLES};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use libreplume_core::{Category, Language};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "LibrePlume is a grammar, spelling, conjugation, typography and style checker for French \
and English. Use `check_text` to list problems in a text (offsets are UTF-16 code units), `correct_text` to get an \
automatically corrected version, and `conjugate_verb` for conjugation tables. Administration tools manage which \
rules are active, the shared custom dictionary, default settings and the web interface themes. Style findings are \
suggestions: review them rather than applying them blindly.";

fn tool(name: &str, title: &str, description: &str, schema: Value, read_only: bool, destructive: bool) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": schema,
        "annotations": {
            "title": title,
            "readOnlyHint": read_only,
            "destructiveHint": destructive,
            "idempotentHint": true,
            "openWorldHint": false,
        },
    })
}

fn language_schema() -> Value {
    json!({ "type": "string", "enum": ["fr", "en", "auto"], "description": "Language of the text (default: automatic detection or the server default)." })
}

fn categories_schema(description: &str) -> Value {
    json!({
        "type": "array",
        "items": { "type": "string", "enum": Category::ALL.iter().map(|c| c.code()).collect::<Vec<_>>() },
        "description": description,
    })
}

pub fn tools() -> Vec<Value> {
    let words =
        json!({ "type": "array", "items": { "type": "string" }, "minItems": 1, "description": "Words, one per item." });
    vec![
        tool("check_text", "Check a text",
            "Find spelling, grammar, conjugation, typography, punctuation and style issues. Returns each issue with its rule, category, severity, explanation, position (UTF-16 offset/length), the flagged text and suggested replacements.",
            json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to check." },
                    "language": language_schema(),
                    "categories": categories_schema("Only report these categories (default: all enabled)."),
                },
                "required": ["text"],
            }), true, false),
        tool("correct_text", "Correct a text",
            "Apply the best suggestion of every error and warning and return the corrected text, the list of applied changes and the issues left for a human (style hints are never applied unless their category is requested).",
            json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to correct." },
                    "language": language_schema(),
                    "categories": categories_schema("Only apply corrections from these categories."),
                },
                "required": ["text"],
            }), true, false),
        tool("conjugate_verb", "Conjugate a verb",
            "Full conjugation table of a French or English verb (all moods and tenses, compound tenses, participles). French pronominal verbs are accepted (« se lever »).",
            json!({
                "type": "object",
                "properties": {
                    "verb": { "type": "string", "description": "Infinitive, e.g. « prendre », « s'asseoir », « to go »." },
                    "language": { "type": "string", "enum": ["fr", "en"], "description": "Default: fr." },
                    "tenses": { "type": "array", "items": { "type": "string" }, "description": "Optional filter on tense ids such as « present », « imparfait », « passe_compose »." },
                },
                "required": ["verb"],
            }), true, false),
        tool("list_rules", "List rules",
            "List checking rules with their id, language, category, description and whether they are enabled on this server.",
            json!({
                "type": "object",
                "properties": {
                    "language": { "type": "string", "enum": ["fr", "en"] },
                    "category": { "type": "string", "enum": Category::ALL.iter().map(|c| c.code()).collect::<Vec<_>>() },
                },
            }), true, false),
        tool("set_rule_enabled", "Enable or disable a rule",
            "Turn a rule on or off for every user of this server.",
            json!({
                "type": "object",
                "properties": { "rule_id": { "type": "string" }, "enabled": { "type": "boolean" } },
                "required": ["rule_id", "enabled"],
            }), false, false),
        tool("get_settings", "Get settings", "Current server-wide settings.", json!({ "type": "object", "properties": {} }), true, false),
        tool("update_settings", "Update settings",
            "Change server-wide settings. Only the given fields change.",
            json!({
                "type": "object",
                "properties": {
                    "default_language": { "type": ["string", "null"], "enum": ["fr", "en", "auto", null], "description": "« auto » or null to detect automatically." },
                    "max_sentence_words": { "type": "integer", "minimum": 0, "maximum": 1000, "description": "Report sentences longer than this (0 disables)." },
                    "disabled_categories": categories_schema("Categories that are never reported."),
                    "default_theme": { "type": "string", "description": "Theme id applied to new visitors of the web interface." },
                },
            }), false, false),
        tool("list_dictionary", "List custom dictionary", "Words added to the shared custom dictionary.", json!({ "type": "object", "properties": {} }), true, false),
        tool("add_dictionary_words", "Add words to the dictionary",
            "Accept words (names, jargon…) so the spell checker stops flagging them.",
            json!({ "type": "object", "properties": { "words": words }, "required": ["words"] }), false, false),
        tool("remove_dictionary_words", "Remove words from the dictionary",
            "Remove words from the shared custom dictionary.",
            json!({ "type": "object", "properties": { "words": words }, "required": ["words"] }), false, true),
        tool("list_themes", "List themes", "Built-in and custom themes of the web interface, and the default one.", json!({ "type": "object", "properties": {} }), true, false),
        tool("save_theme", "Create or update a theme",
            "Create or update a custom theme. Variables are CSS values keyed by the names returned in `variables` of list_themes (colours such as « #4f46e5 », font stacks, sizes).",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "pattern": "^[a-z0-9_-]{1,48}$" },
                    "name": { "type": "string" },
                    "dark": { "type": "boolean" },
                    "base": { "type": "string", "description": "Optional theme id to copy missing variables from (default: plume, or encre when dark)." },
                    "variables": { "type": "object", "additionalProperties": { "type": "string" } },
                },
                "required": ["id", "name"],
            }), false, false),
        tool("delete_theme", "Delete a custom theme", "Delete a custom theme (built-in themes cannot be deleted).",
            json!({ "type": "object", "properties": { "id": { "type": "string" } }, "required": ["id"] }), false, true),
        tool("server_info", "Server information", "Version, languages, categories and endpoints of this LibrePlume instance.", json!({ "type": "object", "properties": {} }), true, false),
    ]
}

fn prompts() -> Vec<Value> {
    vec![
        json!({
            "name": "proofread",
            "title": "Proofread a text",
            "description": "Proofread a text with LibrePlume and explain the corrections.",
            "arguments": [
                { "name": "text", "description": "Text to proofread", "required": true },
                { "name": "language", "description": "fr or en (optional)", "required": false },
            ],
        }),
        json!({
            "name": "improve_style",
            "title": "Improve the style of a text",
            "description": "Rewrite a text using LibrePlume's style suggestions while keeping its meaning.",
            "arguments": [{ "name": "text", "description": "Text to improve", "required": true }],
        }),
    ]
}

fn resources() -> Vec<Value> {
    vec![
        json!({ "uri": "libreplume://settings", "name": "settings", "title": "Server settings", "mimeType": "application/json" }),
        json!({ "uri": "libreplume://dictionary", "name": "dictionary", "title": "Custom dictionary", "mimeType": "application/json" }),
        json!({ "uri": "libreplume://rules", "name": "rules", "title": "Checking rules", "mimeType": "application/json" }),
        json!({ "uri": "libreplume://themes", "name": "themes", "title": "Interface themes", "mimeType": "application/json" }),
    ]
}

/// JSON-RPC error codes.
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// Handles one JSON-RPC message. Returns `None` for notifications and responses.
pub async fn handle_message(state: &AppState, msg: Value) -> Option<Value> {
    let Some(obj) = msg.as_object() else {
        return Some(rpc_error(Value::Null, INVALID_REQUEST, "expected a JSON-RPC object"));
    };
    let id = obj.get("id").cloned();
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        // A response from the client (we never send requests): nothing to do.
        return id.is_none().then(|| rpc_error(Value::Null, INVALID_REQUEST, "missing method"));
    };
    let id = id?; // notifications get no answer
    let params = obj.get("params").cloned().unwrap_or_else(|| json!({}));
    let result = match method {
        "initialize" => {
            let requested = params["protocolVersion"].as_str().unwrap_or(PROTOCOL_VERSIONS[0]);
            let version = PROTOCOL_VERSIONS.iter().find(|v| **v == requested).copied().unwrap_or(PROTOCOL_VERSIONS[0]);
            Ok(json!({
                "protocolVersion": version,
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "listChanged": false, "subscribe": false },
                    "prompts": { "listChanged": false },
                },
                "serverInfo": { "name": "libreplume", "title": "LibrePlume", "version": libreplume_core::VERSION },
                "instructions": INSTRUCTIONS,
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or_default().to_string();
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            if !tools().iter().any(|t| t["name"] == name) {
                return Some(rpc_error(id, INVALID_PARAMS, format!("unknown tool: {name}")));
            }
            Ok(match call_tool(state, &name, &args).await {
                Ok(value) => json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_default() }],
                    "structuredContent": if value.is_object() { value.clone() } else { json!({ "result": value }) },
                    "isError": false,
                }),
                Err(e) => json!({ "content": [{ "type": "text", "text": e.to_string() }], "isError": true }),
            })
        }
        "resources/list" => Ok(json!({ "resources": resources() })),
        "resources/templates/list" => Ok(json!({ "resourceTemplates": [] })),
        "resources/read" => {
            let uri = params["uri"].as_str().unwrap_or_default();
            let content = match uri {
                "libreplume://settings" => Some(json!(state.store.settings())),
                "libreplume://dictionary" => Some(json!(state.store.dictionary())),
                "libreplume://rules" => Some(json!(service::rules(&state.store, None, None))),
                "libreplume://themes" => Some(json!(state.store.themes())),
                _ => None,
            };
            match content {
                Some(c) => Ok(
                    json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": serde_json::to_string_pretty(&c).unwrap_or_default() }] }),
                ),
                None => Err((INVALID_PARAMS, format!("unknown resource: {uri}"))),
            }
        }
        "prompts/list" => Ok(json!({ "prompts": prompts() })),
        "prompts/get" => get_prompt(&params).map_err(|m| (INVALID_PARAMS, m)),
        "logging/setLevel" => Ok(json!({})),
        "completion/complete" => Ok(json!({ "completion": { "values": [], "hasMore": false } })),
        _ => Err((METHOD_NOT_FOUND, format!("method not found: {method}"))),
    };
    Some(match result {
        Ok(r) => rpc_result(id, r),
        Err((code, msg)) => rpc_error(id, code, msg),
    })
}

fn get_prompt(params: &Value) -> Result<Value, String> {
    let name = params["name"].as_str().unwrap_or_default();
    let args = &params["arguments"];
    let text = args["text"].as_str().ok_or("argument « text » is required")?;
    let instruction = match name {
        "proofread" => {
            let lang = args["language"].as_str().map(|l| format!(" (language: {l})")).unwrap_or_default();
            format!(
                "Proofread the following text{lang}. Call the `check_text` tool, then give the corrected text and a short \
                 list of the corrections with a one-line explanation each. Keep the author's voice; treat style hints as optional.\n\n{text}"
            )
        }
        "improve_style" => format!(
            "Improve the style of the following text. Call `check_text` with categories [\"style\"] and use its hints \
             (repetitions, long sentences, pleonasms, weak intensifiers) to rewrite the text without changing its meaning. \
             Then run `check_text` again on your version to make sure no new errors were introduced.\n\n{text}"
        ),
        _ => return Err(format!("unknown prompt: {name}")),
    };
    Ok(json!({
        "description": name,
        "messages": [{ "role": "user", "content": { "type": "text", "text": instruction } }],
    }))
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, ApiError> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request(format!("argument « {key} » is required")))
}

fn arg_words(args: &Value) -> Result<Vec<String>, ApiError> {
    let words: Vec<String> = args["words"]
        .as_array()
        .ok_or_else(|| ApiError::bad_request("argument « words » must be an array of strings"))?
        .iter()
        .filter_map(|w| w.as_str().map(str::to_string))
        .collect();
    if words.is_empty() {
        return Err(ApiError::bad_request("argument « words » is empty"));
    }
    Ok(words)
}

fn arg_categories(args: &Value) -> Result<Option<Vec<Category>>, ApiError> {
    match args.get("categories") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) => a
            .iter()
            .map(|c| c.as_str().unwrap_or_default().parse::<Category>().map_err(ApiError::bad_request))
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        _ => Err(ApiError::bad_request("argument « categories » must be an array")),
    }
}

async fn call_tool(state: &AppState, name: &str, args: &Value) -> Result<Value, ApiError> {
    let language = parse_language(args.get("language").and_then(Value::as_str))?;
    Ok(match name {
        "check_text" => {
            let text = arg_str(args, "text")?.to_string();
            let overrides = RequestOverrides { language, categories: arg_categories(args)?, ..Default::default() };
            let r = service::check(state, text, overrides).await?;
            json!({
                "language": r.language,
                "issue_count": r.matches.len(),
                "issues": r.matches.iter().map(|m| json!({
                    "rule_id": m.rule_id,
                    "category": m.category,
                    "severity": m.severity,
                    "message": m.message,
                    "offset": m.offset,
                    "length": m.length,
                    "text": m.text,
                    "replacements": m.replacements,
                })).collect::<Vec<_>>(),
                "stats": r.stats,
            })
        }
        "correct_text" => {
            let text = arg_str(args, "text")?.to_string();
            let overrides = RequestOverrides { language, ..Default::default() };
            let c = service::correct(state, text, overrides, arg_categories(args)?).await?;
            let summarize = |m: &libreplume_core::Match| json!({ "rule_id": m.rule_id, "category": m.category, "message": m.message, "text": m.text, "replacements": m.replacements });
            json!({
                "language": c.language,
                "corrected": c.corrected,
                "changed": c.corrected != c.original,
                "applied": c.applied.iter().map(|m| {
                    let mut v = summarize(m);
                    v["replacement"] = json!(m.replacements.first());
                    v
                }).collect::<Vec<_>>(),
                "remaining": c.remaining.iter().map(summarize).collect::<Vec<_>>(),
            })
        }
        "conjugate_verb" => {
            let verb = arg_str(args, "verb")?.to_string();
            let mut conj = json!(service::conjugate(verb, language.or(Some(Language::Fr))).await?);
            if let Some(filter) = args["tenses"].as_array().filter(|a| !a.is_empty()) {
                let wanted: Vec<&str> = filter.iter().filter_map(Value::as_str).collect();
                if let Some(moods) = conj["moods"].as_array_mut() {
                    for mood in moods.iter_mut() {
                        if let Some(tenses) = mood["tenses"].as_array_mut() {
                            tenses.retain(|t| wanted.contains(&t["id"].as_str().unwrap_or_default()));
                        }
                    }
                    moods.retain(|m| m["tenses"].as_array().is_some_and(|t| !t.is_empty()));
                }
            }
            conj
        }
        "list_rules" => {
            let category = args
                .get("category")
                .and_then(Value::as_str)
                .map(str::parse::<Category>)
                .transpose()
                .map_err(ApiError::bad_request)?;
            json!({ "rules": service::rules(&state.store, language, category) })
        }
        "set_rule_enabled" => {
            let id = arg_str(args, "rule_id")?;
            let enabled = args["enabled"]
                .as_bool()
                .ok_or_else(|| ApiError::bad_request("argument « enabled » must be a boolean"))?;
            state.store.set_rule(id, enabled)?;
            json!({ "rule_id": id, "enabled": enabled })
        }
        "get_settings" => json!(state.store.settings()),
        "update_settings" => {
            let mut patch = SettingsPatch::default();
            if let Some(l) = args.get("default_language") {
                patch.default_language = Some(parse_language(l.as_str())?);
            }
            if let Some(m) = args.get("max_sentence_words") {
                patch.max_sentence_words =
                    Some(m.as_u64().ok_or_else(|| ApiError::bad_request("max_sentence_words must be an integer"))?
                        as usize);
            }
            if args.get("disabled_categories").is_some() {
                patch.disabled_categories = Some(
                    arg_categories(&json!({ "categories": args["disabled_categories"] }))?
                        .unwrap_or_default()
                        .into_iter()
                        .collect(),
                );
            }
            if let Some(t) = args.get("default_theme").and_then(Value::as_str) {
                patch.default_theme = Some(t.to_string());
            }
            json!(state.store.update_settings(patch)?)
        }
        "list_dictionary" => json!({ "words": state.store.dictionary() }),
        "add_dictionary_words" => json!({ "words": state.store.add_words(&arg_words(args)?)? }),
        "remove_dictionary_words" => json!({ "words": state.store.remove_words(&arg_words(args)?)? }),
        "list_themes" => json!({
            "default": state.store.settings().default_theme,
            "themes": state.store.themes(),
            "variables": THEME_VARIABLES.iter().map(|(n, d)| json!({ "name": n, "description": d })).collect::<Vec<_>>(),
        }),
        "save_theme" => {
            let id = arg_str(args, "id")?.to_string();
            let dark = args["dark"].as_bool().unwrap_or(false);
            let base_id = args["base"].as_str().unwrap_or(if dark { "encre" } else { "plume" });
            let base = state.store.theme(base_id).ok_or_else(|| ApiError::not_found(format!("theme {base_id}")))?;
            let mut variables: BTreeMap<String, String> =
                state.store.theme(&id).map(|t| t.variables).unwrap_or(base.variables);
            if let Some(Value::Object(vars)) = args.get("variables") {
                for (k, v) in vars {
                    let k = k.trim_start_matches("--").to_string();
                    let v = v
                        .as_str()
                        .ok_or_else(|| ApiError::bad_request(format!("variable « {k} » must be a string")))?;
                    variables.insert(k, v.to_string());
                }
            }
            let theme = Theme { id, name: arg_str(args, "name")?.to_string(), dark, builtin: false, variables };
            json!(state.store.save_theme(theme)?)
        }
        "delete_theme" => {
            let id = arg_str(args, "id")?;
            state.store.delete_theme(id)?;
            json!({ "deleted": id })
        }
        "server_info" => service::info(state),
        _ => return Err(ApiError::not_found(format!("unknown tool {name}"))),
    })
}

/// Streamable HTTP transport: one JSON-RPC message (or a batch) per POST, answered with JSON.
pub async fn http_post(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let msg: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, axum::Json(rpc_error(Value::Null, -32700, format!("parse error: {e}"))))
                .into_response()
        }
    };
    let is_initialize = msg.get("method").and_then(Value::as_str) == Some("initialize");
    let response = match msg {
        Value::Array(batch) => {
            let mut out = Vec::new();
            for m in batch {
                if let Some(r) = handle_message(&state, m).await {
                    out.push(r);
                }
            }
            (!out.is_empty()).then_some(Value::Array(out))
        }
        m => handle_message(&state, m).await,
    };
    let Some(response) = response else {
        return StatusCode::ACCEPTED.into_response();
    };
    let mut res = axum::Json(response).into_response();
    let session = headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .or_else(|| is_initialize.then(new_session_id));
    if let Some(sid) = session.and_then(|s| HeaderValue::from_str(&s).ok()) {
        res.headers_mut().insert("mcp-session-id", sid);
    }
    res
}

/// No server-initiated stream is offered.
pub async fn http_get() -> Response {
    (StatusCode::METHOD_NOT_ALLOWED, [(header::ALLOW, "POST, DELETE")]).into_response()
}

/// Sessions hold no state, so ending one always succeeds.
pub async fn http_delete() -> StatusCode {
    StatusCode::NO_CONTENT
}

fn new_session_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
    let a = h.finish();
    let mut h2 = std::collections::hash_map::RandomState::new().build_hasher();
    h2.write_u64(a);
    format!("{a:016x}{:016x}", h2.finish())
}

/// stdio transport: newline-delimited JSON-RPC on stdin/stdout.
pub async fn serve_stdio(state: AppState) -> std::io::Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(Value::Array(batch)) => {
                let mut out = Vec::new();
                for m in batch {
                    if let Some(r) = handle_message(&state, m).await {
                        out.push(r);
                    }
                }
                (!out.is_empty()).then_some(Value::Array(out))
            }
            Ok(msg) => handle_message(&state, msg).await,
            Err(e) => Some(rpc_error(Value::Null, -32700, format!("parse error: {e}"))),
        };
        if let Some(r) = response {
            let mut bytes = serde_json::to_vec(&r).unwrap_or_default();
            bytes.push(b'\n');
            stdout.write_all(&bytes).await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}
