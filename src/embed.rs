use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

/// Convert `Cow<'static, [u8]>` to `Bytes` without copying in release builds
/// (where rust-embed returns `Cow::Borrowed` pointing into the binary's data segment).
fn cow_to_bytes(data: std::borrow::Cow<'static, [u8]>) -> Bytes {
    match data {
        std::borrow::Cow::Borrowed(b) => Bytes::from_static(b),
        std::borrow::Cow::Owned(v) => Bytes::from(v),
    }
}

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
            cow_to_bytes(content.data),
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
            cow_to_bytes(content.data),
        )
            .into_response();
    }

    // No index.html embedded (dev mode with empty dist/)
    (StatusCode::NOT_FOUND, "Not found").into_response()
}
