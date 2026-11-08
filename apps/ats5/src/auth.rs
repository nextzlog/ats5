use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, SameSite, SignedCookieJar};
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::{App, Shared};

const SESSION: &str = "ats5";
const LOGIN: &str = "ats5-login";
const HOURS: i64 = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub email: String,
    pub admin: bool,
}

#[derive(Serialize, Deserialize)]
struct Session {
    email: String,
    expiry: i64,
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn cookie(name: &'static str, value: String, secure: bool) -> Cookie<'static> {
    let mut c = Cookie::new(name, value);
    c.set_path("/");
    c.set_http_only(true);
    c.set_same_site(SameSite::Lax);
    c.set_secure(secure);
    c
}

pub fn user(app: &App, jar: &SignedCookieJar) -> Option<User> {
    let raw = jar.get(SESSION)?;
    let session: Session = serde_json::from_str(raw.value()).ok()?;
    if session.expiry < now() {
        return None;
    }
    let admin = app.config.is_admin(&session.email);
    Some(User {
        email: session.email,
        admin,
    })
}

pub fn sign_in(app: &App, jar: SignedCookieJar, email: &str) -> SignedCookieJar {
    let session = Session {
        email: email.to_ascii_lowercase(),
        expiry: now() + HOURS * 3600,
    };
    let value = serde_json::to_string(&session).unwrap_or_default();
    jar.add(cookie(SESSION, value, app.secure()))
}

#[derive(Deserialize)]
pub struct Next {
    next: Option<String>,
}

fn safe(next: Option<String>) -> String {
    next.filter(|n| n.starts_with('/') && !n.starts_with("//"))
        .unwrap_or_else(|| "/".into())
}

fn random() -> String {
    let bytes: [u8; 24] = rand::random();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub async fn login(State(app): State<Shared>, jar: SignedCookieJar, q: Query<Next>) -> Response {
    let Some(google) = &app.config.google else {
        return Redirect::to("/#/login-unavailable").into_response();
    };
    let state = random();
    let pending = format!("{state} {}", safe(q.0.next));
    let jar = jar.add(cookie(LOGIN, pending, app.secure()));
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?response_type=code&scope=openid%20email&prompt=select_account&client_id={}&redirect_uri={}&state={}",
        encode(&google.client),
        encode(&app.redirect()),
        encode(&state)
    );
    (jar, Redirect::to(&url)).into_response()
}

#[derive(Deserialize)]
pub struct Callback {
    code: Option<String>,
    state: Option<String>,
}

#[derive(Deserialize)]
struct Token {
    id_token: String,
}

#[derive(Deserialize)]
struct Claims {
    iss: String,
    aud: String,
    exp: i64,
    email: Option<String>,
    email_verified: Option<bool>,
}

async fn verify(app: &App, code: &str) -> Result<String, String> {
    let google = app
        .config
        .google
        .as_ref()
        .ok_or("login is not configured")?;
    let redirect = app.redirect();
    let form = [
        ("code", code),
        ("client_id", google.client.as_str()),
        ("client_secret", google.secret.as_str()),
        ("redirect_uri", redirect.as_str()),
        ("grant_type", "authorization_code"),
    ];
    let token: Token = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let payload = token.id_token.split('.').nth(1).ok_or("malformed token")?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|e| e.to_string())?;
    let claims: Claims = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let issuer = matches!(
        claims.iss.as_str(),
        "accounts.google.com" | "https://accounts.google.com"
    );
    if !issuer || claims.aud != google.client || claims.exp < now() {
        return Err("invalid token".into());
    }
    match (claims.email, claims.email_verified) {
        (Some(email), Some(true)) => Ok(email),
        _ => Err("unverified email".into()),
    }
}

pub async fn callback(
    State(app): State<Shared>,
    jar: SignedCookieJar,
    q: Query<Callback>,
) -> Response {
    let pending = jar.get(LOGIN).map(|c| c.value().to_string());
    let jar = jar.remove(Cookie::from(LOGIN));
    let Some((state, next)) = pending.as_deref().and_then(|p| p.split_once(' ')) else {
        return (jar, Redirect::to("/#/login-failed")).into_response();
    };
    if q.state.as_deref() != Some(state) {
        return (jar, Redirect::to("/#/login-failed")).into_response();
    }
    let Some(code) = &q.code else {
        return (jar, Redirect::to("/#/login-failed")).into_response();
    };
    match verify(&app, code).await {
        Ok(email) => {
            let jar = sign_in(&app, jar, &email);
            (jar, Redirect::to(&safe(Some(next.to_string())))).into_response()
        }
        Err(e) => {
            eprintln!("login failed: {e}");
            (jar, Redirect::to("/#/login-failed")).into_response()
        }
    }
}

pub async fn logout(jar: SignedCookieJar) -> Response {
    (jar.remove(Cookie::from(SESSION)), Redirect::to("/")).into_response()
}
