pub mod admin;
#[cfg(not(target_os = "emscripten"))]
pub mod agent;
pub mod api;
pub mod auth;
pub mod board;
pub mod config;
pub mod guard;
pub mod mail;
pub mod store;
pub mod tally;
pub mod web;

use std::convert::Infallible;
use std::path::Path;
use std::sync::Arc;
#[cfg(not(target_os = "emscripten"))]
use std::time::Duration;

use ats5_core::{Builder, Registry};
use axum::extract::{DefaultBodyLimit, FromRef, Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::{self, Next};
#[cfg(not(target_os = "emscripten"))]
use axum::response::sse::KeepAlive;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use axum_extra::extract::cookie::Key;
use rxsl::ruler::Program;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use config::Config;
use guard::Limiter;
use mail::Mailer;
use store::Store;

pub const DEFAULT: &str = "ja1";

pub fn register(b: &mut Builder, rules: Vec<(String, String)>, select: Option<String>) {
    let select = select.unwrap_or_else(|| DEFAULT.into());
    b.extension(0, || ats5_core::Formats);
    b.extension(10, move || ats5_lua::Rules::builtin().only(Some(&select)));
    b.extension(20, move || ats5_lua::Rules { files: rules });
}

pub struct App {
    pub config: Config,
    pub store: Store,
    pub registry: Registry,
    pub contest: usize,
    pub mailer: Mailer,
    pub key: Key,
    pub limiter: Limiter,
    pub reports: Limiter,
    pub board: broadcast::Sender<()>,
    pub agent: broadcast::Sender<Vec<u8>>,
    pub visitor: std::sync::Mutex<Option<auth::User>>,
}

#[derive(Clone)]
pub struct Shared(pub Arc<App>);

impl std::ops::Deref for Shared {
    type Target = App;

    fn deref(&self) -> &App {
        &self.0
    }
}

impl FromRef<Shared> for Key {
    fn from_ref(app: &Shared) -> Key {
        app.key.clone()
    }
}

impl App {
    pub fn new(config: Config, store: Store, key: Key) -> Result<Self, String> {
        let mut b = Builder::default();
        register(&mut b, config.rules.clone(), config.contest.clone());
        let registry = b.build();
        let contest = match &config.contest {
            Some(name) => registry
                .contests()
                .position(|c| c.name().eq_ignore_ascii_case(name))
                .unwrap_or(0),
            None => 0,
        };
        let c = registry
            .contests()
            .nth(contest)
            .ok_or("no contest is registered")?;
        let mailer = Mailer::new(
            config.mail.clone(),
            config.data.clone(),
            config.google.clone(),
            c.host(),
            c.mail(),
        );
        Ok(App {
            config,
            store,
            registry,
            contest,
            mailer,
            key,
            limiter: Limiter::new(600, 10),
            reports: Limiter::new(60, 30),
            board: broadcast::channel(64).0,
            agent: broadcast::channel(256).0,
            visitor: std::sync::Mutex::new(None),
        })
    }

    pub fn contest(&self) -> &dyn Program {
        self.registry.contests().nth(self.contest).expect("contest")
    }

    pub fn start(&self) -> rxsl::Date {
        let c = self.contest();
        self.config
            .period
            .map_or_else(|| c.start_day(c.year()), |p| p.0)
    }

    pub fn deadline(&self) -> rxsl::Date {
        let c = self.contest();
        self.config
            .period
            .map_or_else(|| c.dead_line(c.year()), |p| p.1)
    }

    pub fn accept(&self) -> bool {
        match self.config.period {
            Some((start, end)) => {
                let today = rxsl::Date::today();
                start <= today && today <= end
            }
            None => self.contest().accept(),
        }
    }

    pub fn finish(&self) -> bool {
        match self.config.period {
            Some((_, end)) => end < rxsl::Date::today(),
            None => self.contest().finish(),
        }
    }

    pub fn notify(&self) {
        let _ = self.board.send(());
    }

    pub fn secure(&self) -> bool {
        self.config.url.starts_with("https://")
    }

    pub fn redirect(&self) -> String {
        format!("{}/auth/callback", self.config.url)
    }
}

pub fn key(data: &Path) -> std::io::Result<Key> {
    let path = data.join("secret.key");
    match std::fs::read(&path) {
        Ok(bytes) if bytes.len() >= 64 => Ok(Key::from(&bytes)),
        _ => {
            let key = Key::generate();
            std::fs::write(&path, key.master())?;
            Ok(key)
        }
    }
}

async fn events(State(app): State<Shared>) -> impl IntoResponse {
    let stream = BroadcastStream::new(app.board.subscribe())
        .map(|_| Ok::<_, Infallible>(Event::default().event("update").data("1")));
    #[cfg(not(target_os = "emscripten"))]
    let sse = Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(20)));
    #[cfg(target_os = "emscripten")]
    let sse = Sse::new(stream);
    sse
}

async fn csrf(req: Request, next: Next) -> Response {
    let unsafe_method = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    let marked = req.headers().get("x-ats5").is_some();
    if unsafe_method && !marked {
        return (StatusCode::FORBIDDEN, "missing x-ats5 header").into_response();
    }
    next.run(req).await
}

async fn headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    let set = |h: &mut axum::http::HeaderMap, k: &'static str, v: &'static str| {
        h.insert(k, axum::http::HeaderValue::from_static(v));
    };
    set(h, "x-content-type-options", "nosniff");
    set(h, "referrer-policy", "same-origin");
    set(h, "x-frame-options", "DENY");
    if !h.contains_key("cache-control") {
        set(h, "cache-control", "no-cache, no-store, must-revalidate");
    }
    res
}

pub fn router(app: Arc<App>) -> Router {
    let app = Shared(app);
    let base = Router::new();
    #[cfg(not(target_os = "emscripten"))]
    let base = base.route("/agent/{uuid}", get(agent::agent));
    let api = Router::new()
        .route("/contest", get(api::contest))
        .route("/notes", get(api::notes))
        .route("/calls", get(api::calls))
        .route("/results", get(api::results))
        .route("/entry", post(api::entry))
        .route("/trial", post(api::trial))
        .route("/unbox", post(api::unbox))
        .route("/proof/{call}", get(api::proof))
        .route("/mine", get(api::mine))
        .route("/board", get(api::board))
        .route("/board/events", get(events))
        .route("/admin/stations", get(admin::stations))
        .route(
            "/admin/station/{call}",
            get(admin::station).delete(admin::delete),
        )
        .route("/admin/revert/{id}", post(admin::revert))
        .route("/admin/flags/{id}", post(admin::flags))
        .route("/admin/recalc", post(admin::recalc))
        .route("/admin/clear", post(admin::clear))
        .route("/admin/csv", get(admin::csv))
        .route("/admin/file/{id}", get(admin::file))
        .route("/admin/mail", post(admin::mail))
        .route("/admin/mailer", get(admin::mailer).delete(admin::unlink))
        .route("/admin/form/{call}", get(admin::form))
        .route("/admin/entry", post(admin::entry));
    #[cfg(target_os = "emscripten")]
    let api = api
        .route("/import", post(api::import))
        .route("/visitor", post(auth::visitor));
    let api = api.layer(middleware::from_fn(csrf));
    base.nest("/api", api)
        .route("/board", post(api::report))
        .route("/login", get(auth::login))
        .route("/auth/callback", get(auth::callback))
        .route("/auth/gmail", get(auth::gmail))
        .route("/logout", get(auth::logout))
        .route("/admin/excel", get(admin::csv))
        .route("/admin/table/{call}/{file}", get(admin::table))
        .merge(web::routes())
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .layer(middleware::from_fn(headers))
        .with_state(app)
}
