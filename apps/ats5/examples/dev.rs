use std::net::SocketAddr;
use std::sync::Arc;

use ats5::config::Config;
use ats5::store::Store;
use ats5::{auth, key, router, App};
use axum::extract::Query;
use axum::response::{IntoResponse, Redirect};
use axum::routing::get;
use axum_extra::extract::cookie::SignedCookieJar;
use serde::Deserialize;

#[derive(Deserialize)]
struct Login {
    email: String,
}

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    std::fs::create_dir_all(&config.data).unwrap();
    let store = Store::open(&config.data.join("ats5.db")).unwrap();
    let key = key(&config.data).unwrap();
    let bind = config.bind.clone();
    let app = Arc::new(App::new(config, store, key).unwrap());
    let dev = app.clone();
    let login = move |q: Query<Login>| {
        let app = dev.clone();
        async move {
            let jar = auth::sign_in(&app, SignedCookieJar::new(app.key.clone()), &q.email);
            (jar, Redirect::to("/")).into_response()
        }
    };
    let service = router(app)
        .route("/dev/login", get(login))
        .into_make_service_with_connect_info::<SocketAddr>();
    let listener = tokio::net::TcpListener::bind(&bind).await.unwrap();
    eprintln!("dev server on http://{bind}");
    axum::serve(listener, service).await.unwrap();
}
