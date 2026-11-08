use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Multipart, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::SignedCookieJar;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::{accept, check, need_admin, proof_of, read_upload, Fail, Reply};
use crate::mail::Letter;
use crate::{guard, tally, Shared};

pub async fn stations(State(app): State<Shared>, jar: SignedCookieJar) -> Reply {
    need_admin(&app, &jar)?;
    let c = app.contest();
    let mut out = Vec::new();
    for sub in app.store.actives()? {
        let call = sub.station.call.clone();
        let history = app.store.history(&call)?;
        let entries = app.store.entries(sub.id)?;
        out.push(json!({
            "id": sub.id, "call": call, "name": sub.station.name, "mail": sub.station.mail,
            "time": sub.time, "flags": sub.flags, "history": history.len(),
            "entries": entries,
            "scoreless": tally::scoreless(&app.store, c, &call)?,
            "absent": tally::absent_only(&app.store, c, &sub)?,
        }));
    }
    Ok(Json(out).into_response())
}

pub async fn station(
    State(app): State<Shared>,
    jar: SignedCookieJar,
    Path(call): Path<String>,
) -> Reply {
    need_admin(&app, &jar)?;
    let call = call.to_uppercase();
    let history = app.store.history(&call)?;
    let current = match history.iter().find(|s| s.active) {
        Some(sub) => Some(proof_of(&app, sub, true)?),
        None => None,
    };
    let mut past = Vec::new();
    for sub in &history {
        let files: Vec<Value> = app
            .store
            .archives(sub.id)?
            .iter()
            .map(|a| json!({"id": a.id, "file": a.file}))
            .collect();
        let entries = app.store.entries(sub.id)?;
        past.push(json!({"submission": sub, "files": files, "entries": entries}));
    }
    Ok(Json(json!({"current": current, "history": past})).into_response())
}

pub async fn revert(State(app): State<Shared>, jar: SignedCookieJar, Path(id): Path<i64>) -> Reply {
    need_admin(&app, &jar)?;
    let Some(call) = app.store.activate(id)? else {
        return Err(Fail(StatusCode::NOT_FOUND, json!({"code": "missing"})));
    };
    tally::refresh(&app.store, app.contest(), &call)?;
    app.notify();
    Ok(Json(json!({"call": call})).into_response())
}

#[derive(Deserialize)]
pub struct Flags {
    flags: Vec<String>,
}

pub async fn flags(
    State(app): State<Shared>,
    jar: SignedCookieJar,
    Path(id): Path<i64>,
    Json(f): Json<Flags>,
) -> Reply {
    need_admin(&app, &jar)?;
    app.store.set_flags(id, &f.flags)?;
    Ok(Json(json!({"ok": true})).into_response())
}

pub async fn delete(
    State(app): State<Shared>,
    jar: SignedCookieJar,
    Path(call): Path<String>,
) -> Reply {
    need_admin(&app, &jar)?;
    let call = call.to_uppercase();
    app.store.drop_call(&call)?;
    app.notify();
    Ok(Json(json!({"call": call})).into_response())
}

pub async fn recalc(State(app): State<Shared>, jar: SignedCookieJar) -> Reply {
    need_admin(&app, &jar)?;
    tally::refresh_all(&app.store, app.contest())?;
    app.notify();
    Ok(Json(json!({"ok": true})).into_response())
}

#[derive(Deserialize)]
pub struct Confirm {
    confirm: String,
}

pub async fn clear(
    State(app): State<Shared>,
    jar: SignedCookieJar,
    Json(c): Json<Confirm>,
) -> Reply {
    need_admin(&app, &jar)?;
    if c.confirm != app.contest().name() {
        return Err(Fail(StatusCode::BAD_REQUEST, json!({"code": "confirm"})));
    }
    app.store.clear()?;
    app.notify();
    Ok(Json(json!({"ok": true})).into_response())
}

fn escape(v: &str) -> String {
    let v = v.replace('"', "\"\"").replace(['\n', '\r'], "");
    format!("\"{v}\"")
}

pub async fn csv(State(app): State<Shared>, jar: SignedCookieJar) -> Reply {
    need_admin(&app, &jar)?;
    let c = app.contest();
    let mut lines = Vec::new();
    let actives = app.store.actives()?;
    let station = |call: &str| {
        actives
            .iter()
            .find(|s| s.station.call == call)
            .map(|s| s.station.clone())
            .unwrap_or_default()
    };
    for standing in tally::standings(&app.store, c)? {
        for p in &standing.places {
            let s = station(&p.call);
            lines.push(
                [
                    escape(&p.call),
                    escape(&p.city),
                    escape(&standing.name),
                    p.rank.to_string(),
                    p.award.to_string(),
                    p.total.to_string(),
                    escape(&s.name),
                    escape(&s.post),
                    escape(&s.addr),
                    escape(&s.mail),
                    escape(&s.note),
                ]
                .join(","),
            );
        }
    }
    for r in app.store.rankings()? {
        if c.section(&r.sect).is_some_and(|s| s.is_absence()) {
            let s = station(&r.call);
            let row = [
                &r.call, &r.city, &r.sect, &s.name, &s.post, &s.addr, &s.mail, &s.note,
            ];
            lines.push(row.iter().map(|v| escape(v)).collect::<Vec<_>>().join(","));
        }
    }
    let body = format!("\u{FEFF}{}\n", lines.join("\n"));
    let name = format!("attachment; filename=\"{}.csv\"", c.name());
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, name),
        ],
        body,
    )
        .into_response())
}

pub async fn file(State(app): State<Shared>, jar: SignedCookieJar, Path(id): Path<i64>) -> Reply {
    need_admin(&app, &jar)?;
    let Some((_, archive)) = app.store.archive(id)? else {
        return Err(Fail(StatusCode::NOT_FOUND, json!({"code": "missing"})));
    };
    let name = if archive.file.is_empty() {
        "manual.qxml".to_string()
    } else {
        archive.file
    };
    let safe: String = name
        .chars()
        .filter(|c| !c.is_control() && *c != '"')
        .collect();
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{safe}\""),
            ),
        ],
        archive.data,
    )
        .into_response())
}

#[derive(Deserialize)]
pub struct Mail {
    to: String,
    subject: String,
    body: String,
}

pub async fn mail(State(app): State<Shared>, jar: SignedCookieJar, Json(m): Json<Mail>) -> Reply {
    need_admin(&app, &jar)?;
    let to: Vec<String> =
        m.to.split([',', '\n'])
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(String::from)
            .collect();
    if to.is_empty() || m.subject.trim().is_empty() {
        return Err(Fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"code": "invalid"}),
        ));
    }
    let letter = Letter {
        to,
        subject: m.subject,
        body: m.body,
    };
    match app.mailer.send(letter).await {
        Ok(()) => Ok(Json(json!({"ok": true, "mock": app.mailer.mock()})).into_response()),
        Err(e) => Err(Fail(
            StatusCode::BAD_GATEWAY,
            json!({"code": "mail", "error": e}),
        )),
    }
}

pub async fn form(
    State(app): State<Shared>,
    jar: SignedCookieJar,
    Path(call): Path<String>,
) -> Reply {
    need_admin(&app, &jar)?;
    let Some(sub) = app.store.active(&call.to_uppercase())? else {
        return Err(Fail(StatusCode::NOT_FOUND, json!({"code": "missing"})));
    };
    let files: Vec<Value> = app
        .store
        .archives(sub.id)?
        .iter()
        .map(|a| json!({"id": a.id, "file": a.file}))
        .collect();
    let entries = app.store.entries(sub.id)?;
    Ok(Json(json!({"station": sub.station, "entries": entries, "files": files})).into_response())
}

pub async fn entry(
    State(app): State<Shared>,
    jar: SignedCookieJar,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    mp: Multipart,
) -> Reply {
    need_admin(&app, &jar)?;
    let up = read_upload(mp).await?;
    let call = rxsl::draft::Call::new(&up.form.station.call);
    let mut kept = Vec::new();
    if let Some(prev) = app.store.active(call.value())? {
        for a in app.store.archives(prev.id)? {
            if up.form.keep.contains(&a.id) {
                kept.push((a.file, a.data));
            }
        }
    }
    let peer = guard::peer(addr, &headers, app.config.proxy);
    match check(&app, &up, &kept, true) {
        Ok(checked) => Ok(Json(accept(&app, checked, &peer, true).await?).into_response()),
        Err(p) => Err(Fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"code": "invalid", "problems": p}),
        )),
    }
}
