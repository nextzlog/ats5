use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;

use crate::Shared;

const WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/ats5.wasm"));

include!(concat!(env!("OUT_DIR"), "/web.rs"));

fn kind(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn serve(path: &str) -> Response {
    if path == "ats5.wasm" && !WASM.is_empty() {
        return (
            [
                (header::CONTENT_TYPE, "application/wasm"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            WASM,
        )
            .into_response();
    }
    let page = !path.rsplit('/').next().unwrap_or_default().contains('.');
    let name = if page { "index.html" } else { path };
    match FILES.iter().find(|(p, _)| *p == name) {
        Some((p, body)) => (
            [
                (header::CONTENT_TYPE, kind(p)),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            *body,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/", get(|| async { serve("index.html") }))
        .route(
            "/{*path}",
            get(|Path(path): Path<String>| async move { serve(&path) }),
        )
}
