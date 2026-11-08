use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct Mail {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub mock: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Google {
    pub client: String,
    pub secret: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub data: PathBuf,
    pub bind: String,
    pub url: String,
    pub contest: Option<String>,
    pub admins: Vec<String>,
    pub mail: Mail,
    pub google: Option<Google>,
    pub proxy: bool,
    pub delay: u64,
    pub period: Option<(rxsl::Date, rxsl::Date)>,
}

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn date(text: &str) -> Option<rxsl::Date> {
    let mut parts = text.trim().split('-').map(|p| p.parse::<u32>().ok());
    let (y, m, d) = (parts.next()??, parts.next()??, parts.next()??);
    Some(rxsl::Date::new(y as i32, m, d))
}

fn period(text: &str) -> Option<(rxsl::Date, rxsl::Date)> {
    let (a, b) = text.split_once("..")?;
    Some((date(a)?, date(b)?))
}

fn flag(key: &str) -> bool {
    env(key).is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}

impl Config {
    pub fn from_env() -> Self {
        let bind = env("ATS5_BIND").unwrap_or_else(|| "0.0.0.0:9000".into());
        let url = env("ATS5_URL").unwrap_or_else(|| "http://localhost:9000".into());
        let admins = env("ATS5_ADMINS").unwrap_or_default();
        let host = env("ATS5_MAIL_HOST").unwrap_or_default();
        let google = match (env("ATS5_GOOGLE_ID"), env("ATS5_GOOGLE_SECRET")) {
            (Some(client), Some(secret)) => Some(Google { client, secret }),
            _ => None,
        };
        Config {
            data: env("ATS5_DATA").unwrap_or_else(|| "data".into()).into(),
            bind,
            url: url.trim_end_matches('/').into(),
            contest: env("ATS5_CONTEST"),
            admins: admins
                .split(',')
                .map(|a| a.trim().to_ascii_lowercase())
                .filter(|a| !a.is_empty())
                .collect(),
            mail: Mail {
                mock: host.is_empty() || flag("ATS5_MAIL_MOCK"),
                port: env("ATS5_MAIL_PORT")
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(465),
                user: env("ATS5_MAIL_USER").unwrap_or_default(),
                pass: env("ATS5_MAIL_PASS").unwrap_or_default(),
                host,
            },
            google,
            proxy: flag("ATS5_PROXY"),
            delay: env("ATS5_RTC_DELAY")
                .and_then(|d| d.parse().ok())
                .unwrap_or(5),
            period: env("ATS5_PERIOD").and_then(|p| period(&p)),
        }
    }

    pub fn is_admin(&self, email: &str) -> bool {
        self.admins.iter().any(|a| a.eq_ignore_ascii_case(email))
    }
}
