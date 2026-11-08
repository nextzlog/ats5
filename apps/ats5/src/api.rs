use std::collections::BTreeMap;
use std::net::SocketAddr;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, Multipart, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::cookie::SignedCookieJar;
use rxsl::ruler::{Program, Section};
use rxsl::{Item, QName, Time};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::{now, user, User};
use crate::mail::Letter;
use crate::store::{Entry, Station, Submission};
use crate::{board, guard, tally, App, Shared};

pub struct Fail(pub StatusCode, pub Value);

impl IntoResponse for Fail {
    fn into_response(self) -> Response {
        (self.0, Json(self.1)).into_response()
    }
}

impl From<String> for Fail {
    fn from(e: String) -> Self {
        eprintln!("error: {e}");
        Fail(StatusCode::INTERNAL_SERVER_ERROR, json!({"code": "server"}))
    }
}

impl From<rusqlite::Error> for Fail {
    fn from(e: rusqlite::Error) -> Self {
        Fail::from(e.to_string())
    }
}

pub type Reply = Result<Response, Fail>;

fn code(status: StatusCode, code: &str) -> Fail {
    Fail(status, json!({ "code": code }))
}

pub fn need_user(app: &App, jar: &SignedCookieJar) -> Result<User, Fail> {
    user(app, jar).ok_or_else(|| code(StatusCode::UNAUTHORIZED, "login"))
}

pub fn need_admin(app: &App, jar: &SignedCookieJar) -> Result<User, Fail> {
    let u = need_user(app, jar)?;
    match u.admin {
        true => Ok(u),
        false => Err(code(StatusCode::FORBIDDEN, "admin")),
    }
}

fn date(d: rxsl::Date) -> String {
    d.to_string()
}

fn markdown(text: &str) -> String {
    let mut out = String::new();
    let parser = pulldown_cmark::Parser::new(text);
    pulldown_cmark::html::push_html(&mut out, parser);
    out
}

pub async fn contest(State(app): State<Shared>, jar: SignedCookieJar) -> Reply {
    let c = app.contest();
    let year = c.year();
    let mut lists: Vec<Vec<Value>> = Vec::new();
    let mut sections = Vec::new();
    for s in c.sections() {
        let cities: Vec<Value> = s
            .city_list()
            .iter()
            .map(|c| json!({"code": c.code(), "name": c.name()}))
            .collect();
        let list = match lists.iter().position(|l| *l == cities) {
            Some(i) => i,
            None => {
                lists.push(cities);
                lists.len() - 1
            }
        };
        sections.push(json!({
            "name": s.name(), "code": s.code(), "absence": s.is_absence(), "cities": list,
        }));
    }
    let codes: Vec<Value> = c
        .section_codes()
        .into_iter()
        .map(|code| json!({"code": code, "limit": c.limit_multiple_entry(code)}))
        .collect();
    let formats: Vec<Value> = rxsl::formats()
        .iter()
        .map(|f| json!({"name": f.name(), "extensions": f.extensions()}))
        .collect();
    let sheets: Vec<Value> = rxsl::sheet::sheets()
        .iter()
        .map(|f| json!({"name": f.name(), "extensions": f.extensions()}))
        .collect();
    let me = user(&app, &jar);
    Ok(Json(json!({
        "name": c.name(), "host": c.host(), "mail": c.mail(), "link": c.link(),
        "help": markdown(c.help()),
        "start": date(app.start()), "final": date(c.final_day(year)),
        "deadline": date(app.deadline()),
        "accept": app.accept(), "finish": app.finish(),
        "offset": rxsl::value::local_offset(),
        "sections": sections, "codes": codes, "cities": lists,
        "formats": formats, "sheets": sheets,
        "login": app.config.google.is_some(),
        "user": me, "mock": app.mailer.mock(),
        "media": std::env::var("ATS5_MEDIA").ok(),
        "contributors": include_str!("../contributors").split_whitespace().collect::<Vec<_>>(),
    }))
    .into_response())
}

pub async fn notes(State(app): State<Shared>) -> Reply {
    let notes: Vec<Value> = app
        .store
        .actives()?
        .into_iter()
        .filter(|s| !s.station.note.trim().is_empty())
        .map(|s| json!({"call": s.station.call, "note": s.station.note}))
        .collect();
    Ok(Json(notes).into_response())
}

pub async fn calls(State(app): State<Shared>) -> Reply {
    let c = app.contest();
    let mut out = Vec::new();
    for sub in app.store.actives()? {
        let call = sub.station.call.clone();
        let warning = if tally::scoreless(&app.store, c, &call)? {
            Some("scoreless")
        } else if tally::absent_only(&app.store, c, &sub)? {
            Some("absent")
        } else {
            None
        };
        out.push(json!({"call": call, "warning": warning}));
    }
    Ok(Json(out).into_response())
}

pub async fn results(State(app): State<Shared>, jar: SignedCookieJar) -> Reply {
    let c = app.contest();
    let admin = user(&app, &jar).is_some_and(|u| u.admin);
    if !(admin || app.finish()) {
        return Ok(Json(json!({"open": false})).into_response());
    }
    let standings = tally::standings(&app.store, c)?;
    let mut checklogs = Vec::new();
    for sub in app.store.actives()? {
        if tally::absent_only(&app.store, c, &sub)? {
            checklogs.push(sub.station.call);
        }
    }
    Ok(Json(json!({"open": true, "standings": standings, "checklogs": checklogs})).into_response())
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Manual {
    pub time: String,
    pub call: String,
    pub band: String,
    pub mode: String,
    pub sent: String,
    pub rcvd: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Form {
    pub station: Station,
    #[serde(default)]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub manual: Vec<Manual>,
    #[serde(default)]
    pub keep: Vec<i64>,
    #[serde(default)]
    pub website: String,
}

pub struct Upload {
    pub form: Form,
    pub files: Vec<(String, Vec<u8>)>,
}

pub async fn read_upload(mut mp: Multipart) -> Result<Upload, Fail> {
    let bad = |_| code(StatusCode::BAD_REQUEST, "request");
    let mut form = None;
    let mut files = Vec::new();
    while let Some(field) = mp.next_field().await.map_err(bad)? {
        let name = field.name().unwrap_or_default().to_string();
        let file = field.file_name().map(String::from);
        let data = field.bytes().await.map_err(bad)?;
        match (name.as_str(), file) {
            ("form", _) => {
                form = Some(
                    serde_json::from_slice(&data)
                        .map_err(|_| code(StatusCode::BAD_REQUEST, "request"))?,
                )
            }
            (_, Some(f)) if !data.is_empty() => files.push((f, data.to_vec())),
            _ => (),
        }
    }
    Ok(Upload {
        form: form.unwrap_or_default(),
        files,
    })
}

fn manual_item(m: &Manual, offset: i32) -> Option<Item> {
    let (d, t) = m.time.trim().split_once(['T', ' '])?;
    let (y, rest) = d.split_once('-')?;
    let (mo, da) = rest.split_once('-')?;
    let (h, mi) = t.split_once(':')?;
    let time = Time::new(
        (y.parse().ok()?, mo.parse().ok()?, da.parse().ok()?),
        (h.parse().ok()?, mi.get(..2)?.parse().ok()?, 0),
        offset,
    );
    let band = rxsl::Band::parse(m.band.trim()).ok()?;
    let call = rxsl::draft::Call::new(&m.call);
    if !call.valid() || m.mode.trim().is_empty() || m.rcvd.trim().is_empty() {
        return None;
    }
    let mut item = Item::default();
    item.attrs.set(QName::qxsl("time"), time.to_iso());
    item.attrs.set(QName::qxsl("band"), band.khz());
    item.attrs
        .set(QName::qxsl("mode"), m.mode.trim().to_uppercase());
    item.rcvd.set(QName::qxsl("call"), call.value());
    let phone = rxsl::conv::is_phone(m.mode.trim());
    let rst = if phone { "59" } else { "599" };
    item.rcvd.set(QName::qxsl("rstq"), rst);
    item.sent.set(QName::qxsl("rstq"), rst);
    item.rcvd.set(QName::qxsl("code"), m.rcvd.trim());
    item.sent.set(QName::qxsl("code"), m.sent.trim());
    Some(item)
}

#[derive(Default, Serialize)]
pub struct Problems {
    pub fields: BTreeMap<String, String>,
    pub conflicts: Vec<String>,
    pub files: Vec<Value>,
}

impl Problems {
    fn add(&mut self, field: impl Into<String>, code: &str) {
        self.fields.insert(field.into(), code.into());
    }

    fn empty(&self) -> bool {
        self.fields.is_empty() && self.conflicts.is_empty()
    }
}

fn mail_ok(mail: &str) -> bool {
    let Some((user, host)) = mail.trim().split_once('@') else {
        return false;
    };
    !user.is_empty()
        && host.contains('.')
        && !host.starts_with('.')
        && !mail.contains(char::is_whitespace)
}

pub struct Checked {
    pub station: Station,
    pub entries: Vec<Entry>,
    pub files: Vec<(String, Vec<u8>)>,
    pub items: usize,
}

pub fn check(
    app: &App,
    up: &Upload,
    kept: &[(String, Vec<u8>)],
    admin: bool,
) -> Result<Checked, Problems> {
    let c = app.contest();
    let mut p = Problems::default();
    let f = &up.form;
    let call = rxsl::draft::Call::new(f.station.call.trim());
    if f.station.call.trim().is_empty() {
        p.add("station.call", "required");
    } else if !call.valid() {
        p.add("station.call", "call");
    }
    for (k, v) in [
        ("station.name", &f.station.name),
        ("station.post", &f.station.post),
        ("station.addr", &f.station.addr),
    ] {
        if v.trim().is_empty() {
            p.add(k, "required");
        }
    }
    if f.station.mail.trim().is_empty() {
        p.add("station.mail", "required");
    } else if !mail_ok(&f.station.mail) {
        p.add("station.mail", "mail");
    }
    let mut chosen: Vec<&dyn Section> = Vec::new();
    if f.entries.is_empty() {
        p.add("entries", "required");
    }
    for (i, e) in f.entries.iter().enumerate() {
        let Some(s) = c.section(&e.sect) else {
            p.add(format!("entries.{i}"), "sect");
            continue;
        };
        let cities = s.city_list();
        if !s.is_absence() && !cities.is_empty() && !cities.iter().any(|c| c.name() == e.city) {
            p.add(format!("entries.{i}"), "city");
        }
        if !s.is_absence() {
            chosen.push(s);
        }
    }
    if c.conflict(&chosen) {
        p.conflicts = c.explain(&chosen);
        p.add("entries", "conflict");
    }
    let offset = rxsl::value::local_offset();
    let mut manual = Vec::new();
    for (i, m) in f.manual.iter().enumerate() {
        match manual_item(m, offset) {
            Some(item) => manual.push(item),
            None => p.add(format!("manual.{i}"), "qso"),
        }
    }
    let mut files: Vec<(String, Vec<u8>)> = kept.to_vec();
    files.extend(up.files.iter().cloned());
    let mut items = 0;
    for (name, data) in &files {
        match tally::decode(data) {
            Ok(list) => {
                items += list.len();
                p.files.push(json!({"file": name, "items": list.len()}));
            }
            Err(e) => {
                p.files.push(json!({"file": name, "error": e}));
                p.add(format!("file.{name}"), "unreadable");
            }
        }
    }
    if !manual.is_empty() {
        items += manual.len();
        let data = rxsl::encode(&manual).unwrap_or_default();
        files.push((String::new(), data));
    }
    if items == 0 && !admin {
        p.add("files", "nolog");
    }
    if !p.empty() {
        return Err(p);
    }
    let mut station = f.station.clone();
    station.call = call.value().to_string();
    station.mail = station.mail.trim().to_string();
    let entries = f
        .entries
        .iter()
        .map(|e| {
            let absent = c.section(&e.sect).is_some_and(|s| s.is_absence());
            Entry {
                sect: e.sect.clone(),
                city: if absent {
                    String::new()
                } else {
                    e.city.clone()
                },
            }
        })
        .collect();
    Ok(Checked {
        station,
        entries,
        files,
        items,
    })
}

fn uuid() -> String {
    let b: [u8; 16] = rand::random();
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-4{}-a{}-{}",
        &h[..8],
        &h[8..12],
        &h[13..16],
        &h[17..20],
        &h[20..]
    )
}

fn deadline_end(app: &App) -> i64 {
    let d = app.deadline();
    let next = d.plus_days(1);
    Time::new(
        (next.year, next.month, next.day),
        (0, 0, 0),
        rxsl::value::local_offset(),
    )
    .unix()
}

pub async fn accept(app: &App, checked: Checked, peer: &str, admin: bool) -> Result<Value, Fail> {
    let c = app.contest();
    let call = checked.station.call.clone();
    let history = app.store.history(&call)?;
    let prev = history.iter().find(|s| s.active).cloned();
    let time = now();
    let mut flags = Vec::new();
    if let Some(p) = &prev {
        if !p.station.mail.eq_ignore_ascii_case(&checked.station.mail) {
            flags.push("mail".to_string());
        }
    }
    let end = deadline_end(app);
    let recent = history.iter().filter(|s| time - s.time < 48 * 3600).count();
    if end - time < 48 * 3600 && end > time && recent >= 2 {
        flags.push("burst".to_string());
    }
    if admin {
        flags.clear();
    }
    let mut station = checked.station;
    station.uuid = prev
        .as_ref()
        .map(|p| p.station.uuid.clone())
        .filter(|u| !u.is_empty())
        .unwrap_or_else(uuid);
    let sub = Submission {
        id: 0,
        station: station.clone(),
        time,
        peer: peer.to_string(),
        active: true,
        flags,
    };
    app.store.submit(&sub, &checked.entries, &checked.files)?;
    tally::refresh(&app.store, c, &call)?;
    app.notify();
    let link = format!("{}/#/proof/{}", app.config.url, call);
    let lines: Vec<String> = checked
        .entries
        .iter()
        .map(|e| match e.city.is_empty() {
            true => e.sect.clone(),
            false => format!("{} (運用地 {})", e.sect, e.city),
        })
        .collect();
    let body = format!(
        "{call} 様\n\nこの度は{name}に参加いただきありがとうございます。\n以下の部門にて書類を受理いたしました。\n\n{list}\n\n提出内容は下記のページで確認できます。\n確認には、提出時のメールアドレスの Google アカウントでのログインが必要です。\n{link}\n\n今後とも当コンテストをよろしくお願いいたします。\n{footer}",
        name = c.name(),
        list = lines.join("\n"),
        footer = footer(c),
    );
    let accept = Letter {
        to: vec![format!("{call} <{}>", station.mail)],
        subject: format!("{}書類提出の完了", c.name()),
        body,
    };
    let mut letters = vec![accept];
    if let Some(p) = prev.filter(|p| !p.station.mail.eq_ignore_ascii_case(&station.mail)) {
        letters.push(Letter {
            to: vec![format!("{call} <{}>", p.station.mail)],
            subject: format!("{}書類の再提出のお知らせ", c.name()),
            body: format!(
                "{call} 様\n\n{call} の書類が、別のメールアドレスから再提出されました。\n心当たりがない場合は、下記の連絡先までお知らせください。\n以前の提出は運営の手元に保管されています。\n{}",
                footer(c)
            ),
        });
    }
    let mut mail_errors = Vec::new();
    for l in letters {
        if let Err(e) = app.mailer.send(l).await {
            mail_errors.push(e);
        }
    }
    Ok(json!({
        "call": call, "uuid": station.uuid, "items": checked.items, "entries": checked.entries,
        "mail": mail_errors.is_empty(),
    }))
}

pub fn footer(c: &dyn Program) -> String {
    format!(
        "---\nこのメールに心当たりがない場合はお手数ですが下記の連絡先までご連絡ください。\n---\n{}\n{}受付担当係\n{}\n{}",
        c.host(),
        c.name(),
        c.link(),
        c.mail()
    )
}

pub async fn entry(
    State(app): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    mp: Multipart,
) -> Reply {
    let up = read_upload(mp).await?;
    if !up.form.website.is_empty() {
        return Err(code(StatusCode::BAD_REQUEST, "request"));
    }
    if !app.accept() {
        return Err(code(StatusCode::FORBIDDEN, "closed"));
    }
    let peer = guard::peer(addr, &headers, app.config.proxy);
    if !app.limiter.allow(&peer, now()) {
        return Err(code(StatusCode::TOO_MANY_REQUESTS, "busy"));
    }
    match check(&app, &up, &[], false) {
        Ok(checked) => Ok(Json(accept(&app, checked, &peer, false).await?).into_response()),
        Err(p) => Err(Fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"code": "invalid", "problems": p}),
        )),
    }
}

pub async fn trial(State(app): State<Shared>, mp: Multipart) -> Reply {
    let up = read_upload(mp).await?;
    let c = app.contest();
    let mut files = Vec::new();
    let mut items = Vec::new();
    for (name, data) in &up.files {
        match tally::decode(data) {
            Ok(list) => {
                files.push(json!({"file": name, "items": list.len()}));
                items.extend(list);
            }
            Err(e) => files.push(json!({"file": name, "error": e})),
        }
    }
    let sections: Vec<Value> = up
        .form
        .entries
        .iter()
        .filter_map(|e| c.section(&e.sect))
        .filter(|s| !s.is_absence())
        .map(|s| {
            let sum = s.summarize(&items);
            json!({"sect": s.name(), "valid": sum.accepted().len(), "invalid": sum.rejected().len(),
                   "score": sum.score(), "total": sum.total()})
        })
        .collect();
    Ok(Json(json!({"files": files, "sections": sections})).into_response())
}

pub async fn unbox(State(app): State<Shared>, body: Bytes) -> Reply {
    let c = app.contest();
    let sheet = rxsl::sheet::sheets()
        .into_iter()
        .find_map(|f| f.decode(&body).ok())
        .ok_or_else(|| code(StatusCode::UNPROCESSABLE_ENTITY, "sheet"))?;
    let get = |k: &str| sheet.get(k).unwrap_or_default().trim().to_string();
    let addr = get("ADDRESS");
    let (post, rest) = split_post(&addr);
    let category = Some(get("CATEGORYNAME"))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| get("CATEGORYCODE"));
    let section = c.similar(&category).filter(|_| !category.is_empty());
    let city = section
        .and_then(|s| s.city_base().recommend(&get("OPPLACE")).map(|c| c.name()))
        .filter(|_| !get("OPPLACE").is_empty());
    Ok(Json(json!({
        "call": get("CALLSIGN"), "name": get("NAME"), "post": post, "addr": rest,
        "mail": get("EMAIL"), "note": get("COMMENTS"),
        "sect": section.map(|s| s.name()), "code": section.map(|s| s.code()), "city": city,
    }))
    .into_response())
}

fn split_post(addr: &str) -> (String, String) {
    let chars: Vec<char> = addr.chars().collect();
    for i in 0..chars.len() {
        if chars[i] != '〒' && chars[i] != '〠' {
            continue;
        }
        let mut j = i + 1;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        let digits: String = chars[j..]
            .iter()
            .take_while(|c| c.is_ascii_digit() || **c == '-')
            .collect();
        let pure: String = digits.chars().filter(char::is_ascii_digit).collect();
        if pure.len() == 7 {
            let end = j + digits.chars().count();
            let rest: String = chars[..i].iter().chain(&chars[end..]).collect();
            let post = format!("{}-{}", &pure[..3], &pure[3..]);
            return (post, rest.trim().to_string());
        }
    }
    (String::new(), addr.trim().to_string())
}

fn by_band(rows: &[tally::Row]) -> BTreeMap<String, usize> {
    let mut map = BTreeMap::new();
    for r in rows {
        *map.entry(r.band.clone()).or_insert(0) += 1;
    }
    map
}

pub fn proof_of(app: &App, sub: &Submission, admin: bool) -> Result<Value, Fail> {
    let c = app.contest();
    let files: Vec<Value> = app
        .store
        .archives(sub.id)?
        .iter()
        .map(|a| match tally::decode(&a.data) {
            Ok(items) => json!({"id": a.id, "file": a.file, "items": items.len()}),
            Err(e) => {
                json!({"id": a.id, "file": a.file, "error": if admin { e } else { String::new() }})
            }
        })
        .collect();
    let sheets: Vec<Value> = tally::sheets(&app.store, c, sub)?
        .into_iter()
        .map(|mut s| {
            let valid = by_band(&s.accepted);
            let invalid = by_band(&s.rejected);
            if !admin {
                for r in s.accepted.iter_mut().chain(s.rejected.iter_mut()) {
                    r.text.clear();
                    r.score = 0;
                }
            }
            let mut v = json!({
                "sect": s.sect, "city": s.city, "absence": s.absence,
                "valid": valid, "invalid": invalid,
                "accepted": s.accepted, "rejected": s.rejected,
            });
            if admin {
                v["score"] = json!(s.score);
                v["total"] = json!(s.total);
            }
            v
        })
        .collect();
    Ok(json!({"submission": sub, "files": files, "sheets": sheets}))
}

pub async fn proof(
    State(app): State<Shared>,
    jar: SignedCookieJar,
    Path(call): Path<String>,
) -> Reply {
    let me = need_user(&app, &jar)?;
    let call = call.to_uppercase();
    let Some(sub) = app.store.active(&call)? else {
        return Err(code(StatusCode::NOT_FOUND, "missing"));
    };
    if !me.admin && !sub.station.mail.eq_ignore_ascii_case(&me.email) {
        return Err(code(StatusCode::FORBIDDEN, "owner"));
    }
    Ok(Json(proof_of(&app, &sub, me.admin)?).into_response())
}

pub async fn mine(State(app): State<Shared>, jar: SignedCookieJar) -> Reply {
    let me = need_user(&app, &jar)?;
    let calls: Vec<String> = app
        .store
        .actives()?
        .into_iter()
        .filter(|s| s.station.mail.eq_ignore_ascii_case(&me.email))
        .map(|s| s.station.call)
        .collect();
    Ok(Json(json!({"user": me, "calls": calls})).into_response())
}

pub async fn board(State(app): State<Shared>) -> Reply {
    let reports: Vec<Value> = app
        .store
        .reports()?
        .into_iter()
        .filter_map(|(call, data, time)| {
            let r: board::Report = serde_json::from_str(&data).ok()?;
            let mut bands: BTreeMap<String, i64> = BTreeMap::new();
            for c in r
                .breakdown
                .iter()
                .filter(|c| c.kind == "qso" && !c.band.eq_ignore_ascii_case("total"))
            {
                *bands.entry(c.band.clone()).or_insert(0) += c.value;
            }
            Some(json!({
                "call": call, "area": board::area(&call), "contest": r.contest, "class": r.class,
                "club": r.club, "score": r.score, "qso": r.total("qso"), "mult": r.total("mult"),
                "bands": bands, "time": time,
            }))
        })
        .collect();
    let standings = tally::standings(&app.store, app.contest())?;
    Ok(Json(json!({"reports": reports, "standings": standings})).into_response())
}

pub async fn report(
    State(app): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Reply {
    let peer = guard::peer(addr, &headers, app.config.proxy);
    if !app.reports.allow(&peer, now()) {
        return Err(code(StatusCode::TOO_MANY_REQUESTS, "busy"));
    }
    let text = String::from_utf8_lossy(&body);
    let report = board::parse(&text).map_err(|_| code(StatusCode::BAD_REQUEST, "report"))?;
    let data = serde_json::to_string(&report).map_err(|e| Fail::from(e.to_string()))?;
    app.store.put_report(&report.call, &data, now())?;
    app.notify();
    Ok((StatusCode::OK, [(header::CONTENT_TYPE, "text/plain")], "OK").into_response())
}
