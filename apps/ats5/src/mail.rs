use std::io::Write;
use std::path::PathBuf;

use lettre::message::{header::ContentType, Mailbox};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::Mail;

#[derive(Debug, Clone)]
pub struct Letter {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
}

pub struct Mailer {
    config: Mail,
    log: PathBuf,
    from_name: String,
    reply: String,
}

impl Mailer {
    pub fn new(config: Mail, log: PathBuf, from_name: &str, reply: &str) -> Self {
        Mailer {
            config,
            log,
            from_name: from_name.into(),
            reply: reply.into(),
        }
    }

    pub fn mock(&self) -> bool {
        self.config.mock
    }

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

    fn build(&self, letter: &Letter) -> Result<Message, String> {
        let parse = |s: &str| s.parse::<Mailbox>().map_err(|e| format!("{s}: {e}"));
        let mut b = Message::builder()
            .from(parse(&self.from())?)
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

    pub async fn send(&self, letter: Letter) -> Result<(), String> {
        if self.config.mock {
            self.record(&letter, "MOCK");
            return Ok(());
        }
        let message = self.build(&letter)?;
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
