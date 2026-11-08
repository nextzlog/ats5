use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;

use axum::http::HeaderMap;

pub struct Limiter {
    window: i64,
    limit: usize,
    hits: Mutex<HashMap<String, Vec<i64>>>,
}

impl Limiter {
    pub fn new(window: i64, limit: usize) -> Self {
        Limiter {
            window,
            limit,
            hits: Mutex::new(HashMap::new()),
        }
    }

    pub fn allow(&self, key: &str, now: i64) -> bool {
        let mut hits = self.hits.lock().unwrap_or_else(|e| e.into_inner());
        if hits.len() > 10_000 {
            hits.retain(|_, v| v.last().is_some_and(|t| now - t < self.window));
        }
        let list = hits.entry(key.to_string()).or_default();
        list.retain(|t| now - t < self.window);
        if list.len() >= self.limit {
            return false;
        }
        list.push(now);
        true
    }
}

pub fn peer(addr: SocketAddr, headers: &HeaderMap, proxy: bool) -> String {
    if proxy {
        let forwarded = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .and_then(|v| v.trim().parse::<IpAddr>().ok());
        if let Some(ip) = forwarded {
            return ip.to_string();
        }
    }
    addr.ip().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_bursts() {
        let l = Limiter::new(60, 2);
        assert!(l.allow("a", 0));
        assert!(l.allow("a", 1));
        assert!(!l.allow("a", 2));
        assert!(l.allow("b", 2));
        assert!(l.allow("a", 61));
    }
}
