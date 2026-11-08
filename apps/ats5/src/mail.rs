use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

#[cfg(not(target_os = "emscripten"))]
use base64::Engine;
use serde::{Deserialize, Serialize};

#[cfg(not(target_os = "emscripten"))]
use lettre::message::{header::ContentType, Mailbox};
#[cfg(not(target_os = "emscripten"))]
use lettre::transport::smtp::authentication::Credentials;
#[cfg(not(target_os = "emscripten"))]
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::{Google, Mail};

#[derive(Debug, Clone)]
pub struct Letter {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gmail {
    pub email: String,
    pub refresh: String,
}

#[cfg_attr(target_os = "emscripten", allow(dead_code))]
pub struct Mailer {
    config: Mail,
    log: PathBuf,
    token: PathBuf,
    google: Option<Google>,
    gmail: Mutex<Option<Gmail>>,
    from_name: String,
    reply: String,
}

#[cfg(not(target_os = "emscripten"))]
#[derive(Deserialize)]
struct Access {
    access_token: String,
}

impl Mailer {
    pub fn new(
        config: Mail,
        data: PathBuf,
        google: Option<Google>,
        from_name: &str,
        reply: &str,
    ) -> Self {
        let token = data.join("gmail.json");
        let gmail = std::fs::read(&token)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        Mailer {
            config,
            log: data.join("mail.log"),
            token,
            google,
            gmail: Mutex::new(gmail),
            from_name: from_name.into(),
            reply: reply.into(),
        }
    }

    fn gmail(&self) -> Option<Gmail> {
        self.gmail.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn method(&self) -> (&'static str, Option<String>) {
        match self.gmail() {
            Some(g) => ("gmail", Some(g.email)),
            None if self.config.host.is_empty() => ("none", None),
            None => ("smtp", Some(self.config.host.clone())),
        }
    }

    pub fn connect(&self, gmail: Gmail) -> Result<(), String> {
        let json = serde_json::to_vec(&gmail).map_err(|e| e.to_string())?;
        std::fs::write(&self.token, json).map_err(|e| e.to_string())?;
        *self.gmail.lock().unwrap_or_else(|e| e.into_inner()) = Some(gmail);
        Ok(())
    }

    pub fn disconnect(&self) {
        let _ = std::fs::remove_file(&self.token);
        *self.gmail.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    #[cfg(not(target_os = "emscripten"))]
    fn from(&self) -> String {
        let user = &self.config.user;
        let addr = if user.contains('@') {
            user.clone()
        } else {
            format!("{user}@{}", self.config.host)
        };
        format!("{} <{addr}>", self.from_name)
    }

    fn record(&self, letter: &Letter, state: &str) {
        let line = format!(
            "{state}\tto={}\tsubject={}\n{}\n---\n",
            letter.to.join(","),
            letter.subject,
            letter.body
        );
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log)
        {
            let _ = f.write_all(line.as_bytes());
        }
    }

    #[cfg(not(target_os = "emscripten"))]
    fn build(&self, letter: &Letter, from: &str) -> Result<Message, String> {
        let parse = |s: &str| s.parse::<Mailbox>().map_err(|e| format!("{s}: {e}"));
        let mut b = Message::builder()
            .from(parse(from)?)
            .subject(letter.subject.clone())
            .header(ContentType::TEXT_PLAIN);
        for to in &letter.to {
            b = b.to(parse(to)?);
        }
        if let Ok(reply) = parse(&self.reply) {
            b = b.bcc(reply.clone()).reply_to(reply);
        }
        b.body(letter.body.clone()).map_err(|e| e.to_string())
    }

    #[cfg(not(target_os = "emscripten"))]
    async fn gmail_send(&self, gmail: &Gmail, letter: &Letter) -> Result<(), String> {
        let google = self.google.as_ref().ok_or("OAuth is not configured")?;
        let client = reqwest::Client::new();
        let form = [
            ("client_id", google.client.as_str()),
            ("client_secret", google.secret.as_str()),
            ("refresh_token", gmail.refresh.as_str()),
            ("grant_type", "refresh_token"),
        ];
        let access: Access = client
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
        let from = format!("{} <{}>", self.from_name, gmail.email);
        let raw = self.build(letter, &from)?.formatted();
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
        client
            .post("https://gmail.googleapis.com/gmail/v1/users/me/messages/send")
            .bearer_auth(access.access_token)
            .json(&serde_json::json!({ "raw": raw }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[cfg(not(target_os = "emscripten"))]
    pub async fn send(&self, letter: Letter) -> Result<(), String> {
        if let Some(gmail) = self.gmail() {
            return match self.gmail_send(&gmail, &letter).await {
                Ok(()) => {
                    self.record(&letter, "GMAIL");
                    Ok(())
                }
                Err(e) => {
                    self.record(&letter, &format!("FAILED {e}"));
                    Err(e)
                }
            };
        }
        if self.config.host.is_empty() {
            self.record(&letter, "UNSENT");
            return Err("mail is not configured".into());
        }
        let message = self.build(&letter, &self.from())?;
        let creds = Credentials::new(self.config.user.clone(), self.config.pass.clone());
        let relay = match self.config.port {
            465 => AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.host),
            _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.host),
        };
        let transport = relay
            .map_err(|e| e.to_string())?
            .port(self.config.port)
            .credentials(creds)
            .build();
        match transport.send(message).await {
            Ok(_) => {
                self.record(&letter, "SENT");
                Ok(())
            }
            Err(e) => {
                self.record(&letter, &format!("FAILED {e}"));
                Err(e.to_string())
            }
        }
    }
}

#[cfg(target_os = "emscripten")]
impl Mailer {
    pub async fn send(&self, letter: Letter) -> Result<(), String> {
        self.record(&letter, "UNSENT");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gmail_link_persists() {
        let dir = std::env::temp_dir().join(format!("ats5-mail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let config = Mail::default();
        let mailer = Mailer::new(
            config.clone(),
            dir.clone(),
            None,
            "host",
            "reply@example.com",
        );
        assert_eq!(mailer.method().0, "none");
        let gmail = Gmail {
            email: "organizer@example.com".into(),
            refresh: "token".into(),
        };
        mailer.connect(gmail).unwrap();
        let again = Mailer::new(
            config.clone(),
            dir.clone(),
            None,
            "host",
            "reply@example.com",
        );
        assert_eq!(
            again.method(),
            ("gmail", Some("organizer@example.com".into()))
        );
        again.disconnect();
        let last = Mailer::new(config, dir.clone(), None, "host", "reply@example.com");
        assert_eq!(last.method().0, "none");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
