#![allow(clippy::missing_safety_doc, clippy::too_many_arguments)]

use std::cell::RefCell;
use std::sync::Arc;

use ats5::config::Config;
use ats5::store::Store;
use ats5::{router, tally, App};
use axum::body::Body;
use axum::http::Request;
use axum::Router;
use axum_extra::extract::cookie::Key;
use http_body_util::BodyExt;
use tower::ServiceExt;

thread_local! {
    static SITE: RefCell<Option<Router>> = const { RefCell::new(None) };
}

fn text(ptr: *const u8, len: usize) -> String {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(bytes).into_owned()
}

fn reply(status: u16, head: &str, body: &[u8]) -> *mut u8 {
    let mut out = Vec::with_capacity(8 + head.len() + body.len());
    out.extend((status as u32).to_le_bytes());
    out.extend((head.len() as u32).to_le_bytes());
    out.extend(head.as_bytes());
    out.extend(body);
    let mut frame = (out.len() as u32).to_le_bytes().to_vec();
    frame.extend(out);
    let ptr = ats5_alloc(frame.len());
    unsafe { std::ptr::copy_nonoverlapping(frame.as_ptr(), ptr, frame.len()) };
    ptr
}

fn open(spec: &str) -> Result<Router, String> {
    let (contest, dir) = spec.split_once(' ').unwrap_or((spec, spec));
    let data = std::path::PathBuf::from("/data").join(dir);
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let mut config = Config::from_env();
    config.data = data.clone();
    config.contest = Some(contest.to_string());
    let store = Store::open(&data.join("ats5.db")).map_err(|e| e.to_string())?;
    let key = ats5::key(&data).unwrap_or_else(|_| Key::generate());
    let app = Arc::new(App::new(config, store, key)?);
    tally::refresh_all(&app.store, app.contest())?;
    Ok(router(app))
}

#[unsafe(no_mangle)]
pub extern "C" fn ats5_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ats5_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(unsafe { Vec::from_raw_parts(ptr, 0, len.max(1)) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ats5_open(ptr: *const u8, len: usize) -> *mut u8 {
    let contest = text(ptr, len);
    match open(&contest) {
        Ok(site) => {
            SITE.with(|s| *s.borrow_mut() = Some(site));
            reply(200, "content-type: text/plain", b"OK")
        }
        Err(e) => reply(500, "content-type: text/plain", e.as_bytes()),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ats5_request(
    method: *const u8,
    method_len: usize,
    uri: *const u8,
    uri_len: usize,
    kind: *const u8,
    kind_len: usize,
    body: *const u8,
    body_len: usize,
) -> *mut u8 {
    let method = text(method, method_len);
    let uri = text(uri, uri_len);
    let kind = text(kind, kind_len);
    let body = unsafe { std::slice::from_raw_parts(body, body_len) }.to_vec();
    let Some(site) = SITE.with(|s| s.borrow().clone()) else {
        return reply(503, "content-type: text/plain", b"no contest is open");
    };
    let mut req = Request::builder()
        .method(method.as_str())
        .uri(uri)
        .header("x-ats5", "1");
    if !kind.is_empty() {
        req = req.header("content-type", kind);
    }
    let Ok(req) = req.body(Body::from(body)) else {
        return reply(400, "content-type: text/plain", b"bad request");
    };
    futures_executor::block_on(async move {
        let Ok(res) = site.oneshot(req).await;
        let status = res.status().as_u16();
        let head: Vec<String> = res
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| format!("{k}: {v}")))
            .collect();
        let bytes = res
            .into_body()
            .collect()
            .await
            .map(|b| b.to_bytes().to_vec())
            .unwrap_or_default();
        reply(status, &head.join("\n"), &bytes)
    })
}

fn main() {}
