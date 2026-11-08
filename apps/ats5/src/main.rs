use std::net::SocketAddr;
use std::sync::Arc;

use ats5::config::Config;
use ats5::store::Store;
use ats5::{key, router, tally, App};

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    if let Err(e) = std::fs::create_dir_all(&config.data) {
        eprintln!("cannot create {}: {e}", config.data.display());
        std::process::exit(1);
    }
    let store = match Store::open(&config.data.join("ats5.db")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot open the database: {e}");
            std::process::exit(1);
        }
    };
    let key = key(&config.data).unwrap_or_else(|e| {
        eprintln!("cannot create the secret key: {e}");
        std::process::exit(1);
    });
    let bind = config.bind.clone();
    let app = match App::new(config, store, key) {
        Ok(app) => Arc::new(app),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = tally::refresh_all(&app.store, app.contest()) {
        eprintln!("cannot tabulate: {e}");
    }
    if app.config.google.is_none() {
        eprintln!(
            "warning: ATS5_GOOGLE_ID and ATS5_GOOGLE_SECRET are not set; sign-in is disabled"
        );
    }
    if app.mailer.method().0 == "none" {
        eprintln!("warning: mail is not configured; letters are written to mail.log only");
    }
    let port = bind.rsplit(':').next().unwrap_or("9000");
    eprintln!(
        "ATS-5 for {} on http://localhost:{port}",
        app.contest().name()
    );
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| {
            eprintln!("cannot listen on {bind}: {e}");
            std::process::exit(1);
        });
    let service = router(app).into_make_service_with_connect_info::<SocketAddr>();
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    if let Err(e) = axum::serve(listener, service)
        .with_graceful_shutdown(shutdown)
        .await
    {
        eprintln!("{e}");
    }
}
