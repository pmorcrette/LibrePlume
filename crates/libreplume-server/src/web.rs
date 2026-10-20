//! Web interface, embedded in the binary.

use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web"]
struct Assets;

pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(file) => respond(path, file),
        // Unknown paths without an extension are client-side routes.
        None if !path.contains('.') => match Assets::get("index.html") {
            Some(file) => respond("index.html", file),
            None => StatusCode::NOT_FOUND.into_response(),
        },
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn respond(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let cache =
        if path == "index.html" || path.ends_with(".webmanifest") { "no-cache" } else { "public, max-age=3600" };
    (
        [(header::CONTENT_TYPE, file.metadata.mimetype().to_string()), (header::CACHE_CONTROL, cache.to_string())],
        file.data,
    )
        .into_response()
}
