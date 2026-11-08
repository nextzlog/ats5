use std::collections::BTreeMap;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};

use crate::{tally, App, Shared};

pub async fn agent(
    State(app): State<Shared>,
    Path(uuid): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    if !app.accept() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(Some(sub)) = app.store.by_uuid(&uuid) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let call = sub.station.call;
    ws.on_upgrade(move |socket| run(app, call, socket))
}

pub fn push(app: &App, call: &str, data: &[u8]) -> Result<String, String> {
    let Some((&head, body)) = data.split_first() else {
        return Ok(String::new());
    };
    let items = rxsl::sheet::unpack(body).map_err(|e| e.to_string())?;
    let n = (head as usize).min(items.len());
    let db = |r: rusqlite::Result<()>| r.map_err(|e| e.to_string());
    db(app.store.drop_messages(call, &items[..n]))?;
    db(app.store.add_messages(call, &items[n..]))?;
    tally::update(&app.store, app.contest(), call)?;
    app.notify();
    if !app.finish() {
        return Ok(String::new());
    }
    Ok(rankings(app)?.to_string())
}

pub fn rankings(app: &App) -> Result<Value, String> {
    let rows = app.store.rankings().map_err(|e| e.to_string())?;
    let mut map: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for s in app.contest().sections() {
        map.insert(s.name().to_string(), Vec::new());
    }
    for r in rows {
        let list = map.entry(r.sect.clone()).or_default();
        list.push(json!({"call": r.call, "score": r.score, "total": r.total}));
    }
    Ok(json!(map))
}

async fn run(app: Shared, call: String, socket: WebSocket) {
    let (mut tx, mut rx) = socket.split();
    let mut bus = app.agent.subscribe();
    loop {
        tokio::select! {
            msg = rx.next() => match msg {
                Some(Ok(Message::Binary(data))) => {
                    let reply = push(&app, &call, &data).unwrap_or_else(|e| {
                        eprintln!("agent {call}: {e}");
                        String::new()
                    });
                    let app = app.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(app.config.delay)).await;
                        let _ = app.agent.send(reply.into_bytes());
                    });
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => (),
            },
            out = bus.recv() => match out {
                Ok(bytes) => {
                    if tx.send(Message::Binary(bytes.into())).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => (),
                Err(_) => break,
            },
        }
    }
}
