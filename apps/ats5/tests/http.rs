use std::net::SocketAddr;
use std::sync::Arc;

use ats5::config::Config;
use ats5::store::Store;
use ats5::{auth, router, App, Shared};
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{header, Request, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use axum_extra::extract::cookie::{Key, SignedCookieJar};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const ADMIN: &str = "admin@example.com";

fn sample() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/ats5-lua/tests/data/allja1.qxml"
    );
    std::fs::read(path).unwrap()
}

struct Test {
    app: Arc<App>,
    router: Router,
    dir: std::path::PathBuf,
}

impl Test {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("ats5-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut config = Config::from_env();
        config.data = dir.clone();
        config.admins = vec![ADMIN.into()];
        config.delay = 0;
        let app = Arc::new(App::new(config, Store::memory().unwrap(), Key::generate()).unwrap());
        let router =
            router(app.clone()).layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))));
        Test { app, router, dir }
    }

    fn cookie(&self, email: &str) -> String {
        let jar = auth::sign_in(&self.app, SignedCookieJar::new(self.app.key.clone()), email);
        let res = jar.into_response();
        let set = res
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        set.split(';').next().unwrap().to_string()
    }

    async fn send(&self, req: Request<Body>) -> (StatusCode, Value, String) {
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8_lossy(&bytes).to_string();
        (
            status,
            serde_json::from_str(&text).unwrap_or(Value::Null),
            text,
        )
    }

    async fn get(&self, path: &str, who: Option<&str>) -> (StatusCode, Value, String) {
        let mut b = Request::get(path);
        if let Some(w) = who {
            b = b.header(header::COOKIE, self.cookie(w));
        }
        self.send(b.body(Body::empty()).unwrap()).await
    }

    async fn upload(
        &self,
        path: &str,
        who: Option<&str>,
        form: Value,
        files: &[(&str, Vec<u8>)],
    ) -> (StatusCode, Value, String) {
        let boundary = "ats5boundary";
        let mut body = Vec::new();
        body.extend(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"form\"\r\n\r\n{form}\r\n"
            )
            .as_bytes(),
        );
        for (name, data) in files {
            body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
            body.extend(data);
            body.extend(b"\r\n");
        }
        body.extend(format!("--{boundary}--\r\n").as_bytes());
        let mut b = Request::post(path).header("x-ats5", "1").header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        );
        if let Some(w) = who {
            b = b.header(header::COOKIE, self.cookie(w));
        }
        self.send(b.body(Body::from(body)).unwrap()).await
    }
}

impl Drop for Test {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn form(call: &str, mail: &str, sect: &str) -> Value {
    json!({
        "station": {"call": call, "name": "Test", "post": "113-0033", "addr": "Tokyo", "mail": mail, "note": "73"},
        "entries": [{"sect": sect, "city": "東京都 文京区"}],
    })
}

const TOTAL: &str = "1エリア内 個人 総合 部門";

#[tokio::test]
async fn contest_is_described() {
    let t = Test::new("contest");
    let (status, v, _) = t.get("/api/contest", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["name"], "ALLJA1");
    assert_eq!(v["contributors"][0], "JG1VPP");
    assert!(v["sections"].as_array().unwrap().len() > 50);
    assert!(v["formats"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["name"] == "zLog (ZLOX)"));
    for path in [
        "/",
        "/entry",
        "/lists",
        "/calls",
        "/forum",
        "/guide",
        "/admin",
        "/admin/proof/JA1ZLO",
    ] {
        let (status, _, html) = t.get(path, None).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(html.contains("app.js"), "{path}");
    }
    let (status, _, js) = t.get("/views/entry.js", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(js.contains("entryPage"));
    let (status, _, _) = t.get("/missing.js", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn writes_need_the_header() {
    let t = Test::new("csrf");
    let req = Request::post("/api/entry").body(Body::empty()).unwrap();
    let (status, _, _) = t.send(req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn public_entry_follows_the_period() {
    let t = Test::new("period");
    let (status, v, _) = t
        .upload(
            "/api/entry",
            None,
            form("JA1ZLO", "a@example.com", TOTAL),
            &[("a.qxml", sample())],
        )
        .await;
    if t.app.contest().accept() {
        assert_eq!(status, StatusCode::OK, "{v}");
    } else {
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(v["code"], "closed");
    }
}

#[tokio::test]
async fn honeypot_is_rejected() {
    let t = Test::new("honeypot");
    let mut f = form("JA1ZLO", "a@example.com", TOTAL);
    f["website"] = json!("spam");
    let (status, _, _) = t
        .upload("/api/entry", None, f, &[("a.qxml", sample())])
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn problems_point_at_fields() {
    let t = Test::new("problems");
    let f = json!({
        "station": {"call": "JA1 ZLO", "name": "", "post": "1", "addr": "x", "mail": "bad"},
        "entries": [{"sect": "1エリア内 個人 電信限定 7MHz部門", "city": "東京都 文京区"}],
        "manual": [{"time": "", "call": "", "band": "7MHz", "mode": "CW", "sent": "", "rcvd": ""}],
    });
    let (status, v, _) = t
        .upload(
            "/api/admin/entry",
            Some(ADMIN),
            f,
            &[("bad.txt", b"garbage".to_vec())],
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let fields = &v["problems"]["fields"];
    assert_eq!(fields["station.call"], "call");
    assert_eq!(fields["station.name"], "required");
    assert_eq!(fields["station.mail"], "mail");
    assert_eq!(fields["entries"], "conflict");
    assert_eq!(fields["manual.0"], "qso");
    assert!(fields.get("file.bad.txt").is_none());
    assert!(v["problems"]["conflicts"][0]
        .as_str()
        .unwrap()
        .contains("総合部門"));
}

#[tokio::test]
async fn submission_is_tabulated_and_protected() {
    let t = Test::new("flow");
    let (status, v, _) = t
        .upload(
            "/api/admin/entry",
            Some(ADMIN),
            form("ja1zlo", "owner@example.com", TOTAL),
            &[("allja1.qxml", sample())],
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert_eq!(v["call"], "JA1ZLO");
    assert_eq!(v["items"], 1000);
    assert_eq!(v["uuid"].as_str().unwrap().len(), 36);

    let (_, results, _) = t.get("/api/results", Some(ADMIN)).await;
    let total = &results["standings"][0]["places"][0];
    assert_eq!(total["call"], "JA1ZLO");
    assert_eq!(total["score"], 429);
    assert_eq!(total["total"], 139425);

    let (status, _, _) = t.get("/api/proof/JA1ZLO", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = t.get("/api/proof/JA1ZLO", Some("other@example.com")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, p, _) = t.get("/api/proof/JA1ZLO", Some("Owner@Example.com")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(p["sheets"][0].get("total").is_none());
    assert!(p["sheets"][0]["rejected"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["text"] == ""));
    let (_, p, _) = t.get("/api/proof/JA1ZLO", Some(ADMIN)).await;
    assert_eq!(p["sheets"][0]["total"], 139425);

    let (status, _, _) = t
        .get("/api/admin/stations", Some("owner@example.com"))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, csv, text) = t.get("/admin/excel", Some(ADMIN)).await;
    assert!(csv.is_null());
    assert!(text.contains("\"JA1ZLO\""));
    assert!(text.contains("139425"));

    let (status, _, log) = t.get("/admin/table/JA1ZLO/allja1.qxml", Some(ADMIN)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(log.contains("qxsl"));
    let (status, _, _) = t
        .get("/admin/table/JA1ZLO/allja1.qxml", Some("owner@example.com"))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, mine, _) = t.get("/api/mine", Some("owner@example.com")).await;
    assert_eq!(mine["calls"][0], "JA1ZLO");
    let log = std::fs::read_to_string(t.dir.join("mail.log")).unwrap();
    assert!(log.contains("owner@example.com"));
}

#[tokio::test]
async fn unreadable_logs_are_accepted_and_flagged() {
    let t = Test::new("unreadable");
    let files = [("log.xlsx", b"PK\x03\x04 not a log".to_vec())];
    let (status, v, _) = t
        .upload(
            "/api/admin/entry",
            Some(ADMIN),
            form("JA1ZLO", "a@example.com", TOTAL),
            &files,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert_eq!(v["unreadable"][0], "log.xlsx");
    let (_, stations, _) = t.get("/api/admin/stations", Some(ADMIN)).await;
    assert_eq!(stations[0]["flags"][0], "unreadable");
    let log = std::fs::read_to_string(t.dir.join("mail.log")).unwrap();
    assert!(log.contains("log.xlsx"));
}

#[tokio::test]
async fn resubmission_keeps_history_and_reverts() {
    let t = Test::new("history");
    t.upload(
        "/api/admin/entry",
        Some(ADMIN),
        form("JA1ZLO", "a@example.com", TOTAL),
        &[("a.qxml", sample())],
    )
    .await;
    let (status, _, _) = t
        .upload(
            "/api/admin/entry",
            Some(ADMIN),
            form("JA1ZLO", "b@example.com", TOTAL),
            &[],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, station, _) = t.get("/api/admin/station/JA1ZLO", Some(ADMIN)).await;
    let history = station["history"].as_array().unwrap();
    assert_eq!(history.len(), 2);
    let log = std::fs::read_to_string(t.dir.join("mail.log")).unwrap();
    assert!(log.contains("再提出"));
    let (_, results, _) = t.get("/api/results", Some(ADMIN)).await;
    assert_eq!(results["standings"][0]["places"][0]["score"], 0);
    let old = history[1]["submission"]["id"].as_i64().unwrap();
    let req = Request::post(format!("/api/admin/revert/{old}"))
        .header("x-ats5", "1")
        .header(header::COOKIE, t.cookie(ADMIN))
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = t.send(req).await;
    assert_eq!(status, StatusCode::OK);
    let (_, results, _) = t.get("/api/results", Some(ADMIN)).await;
    assert_eq!(results["standings"][0]["places"][0]["score"], 429);
}

#[tokio::test]
async fn gmail_link_is_for_organizers() {
    let t = Test::new("gmail");
    let (status, v, _) = t.get("/api/admin/mailer", Some(ADMIN)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["method"], "none");
    let (status, _, _) = t.get("/api/admin/mailer", Some("owner@example.com")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let res = t
        .router
        .clone()
        .oneshot(Request::get("/auth/gmail").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let to = res
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(to.starts_with("/login"), "{to}");
}

#[tokio::test]
async fn online_scores_reach_the_board() {
    let t = Test::new("board");
    let xml = "<dynamicresults><contest>ALLJA1</contest><call>JA1ZLO</call><class ops=\"SINGLE-OP\"/><breakdown><qso band=\"total\" mode=\"ALL\">12</qso><qso band=\"7\" mode=\"CW\">12</qso></breakdown><score>144</score></dynamicresults>";
    let req = Request::post("/board").body(Body::from(xml)).unwrap();
    let (status, _, _) = t.send(req).await;
    assert_eq!(status, StatusCode::OK);
    let (_, board, _) = t.get("/api/board", None).await;
    assert_eq!(board["reports"][0]["call"], "JA1ZLO");
    assert_eq!(board["reports"][0]["score"], 144);
    assert_eq!(board["reports"][0]["bands"]["7"], 12);
}

#[tokio::test]
async fn summary_sheets_fill_the_form() {
    let t = Test::new("sheet");
    let sheet = "<SUMMARYSHEET VERSION=R2.1>\r\n<CALLSIGN>JA1ZLO</CALLSIGN>\r\n<NAME>東大無線部</NAME>\r\n<ADDRESS>〒113-8656 東京都文京区本郷</ADDRESS>\r\n<EMAIL>a@example.com</EMAIL>\r\n<CATEGORYNAME>1エリア内 個人 総合 部門</CATEGORYNAME>\r\n<OPPLACE>東京都文京区</OPPLACE>\r\n</SUMMARYSHEET>\r\n";
    let req = Request::post("/api/unbox")
        .header("x-ats5", "1")
        .body(Body::from(sheet))
        .unwrap();
    let (status, v, text) = t.send(req).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert_eq!(v["call"], "JA1ZLO");
    assert_eq!(v["post"], "113-8656", "{v}");
    assert_eq!(v["addr"], "東京都文京区本郷");
    assert_eq!(v["sect"], TOTAL);
    assert_eq!(v["city"], "東京都 文京区");
}

#[test]
fn shared_state_derefs() {
    fn takes(_: &App) {}
    let config = Config::from_env();
    let app = Arc::new(App::new(config, Store::memory().unwrap(), Key::generate()).unwrap());
    takes(&Shared(app));
}
