use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

pub async fn handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try the exact path first
    if let Some(content) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let cache = if path.starts_with("assets/") {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        };
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, mime.as_ref().to_string()),
                (header::CACHE_CONTROL, cache.to_string()),
            ],
            content.data.to_vec(),
        )
            .into_response();
    }

    // SPA fallback: serve index.html for non-file paths
    if let Some(content) = Assets::get("index.html") {
        let mime = mime_guess::from_path("index.html").first_or_octet_stream();
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, mime.as_ref().to_string()),
                (header::CACHE_CONTROL, "no-cache".to_string()),
            ],
            content.data.to_vec(),
        )
            .into_response();
    }

    // No index.html embedded (dev mode with empty dist/)
    (StatusCode::NOT_FOUND, "Not found").into_response()
}
