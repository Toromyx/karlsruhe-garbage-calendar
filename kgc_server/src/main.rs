use std::net::SocketAddr;

use axum::{routing::get, Router};
use tower_http::services::{ServeDir, ServeFile};

mod route;

#[cfg(debug_assertions)]
const SERVE_DIR: &str = "kgc_server/frontend/dist";
#[cfg(not(debug_assertions))]
const SERVE_DIR: &str = "dist";

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/calendar", get(route::calendar::handler))
        .route(
            "/calendar/residual",
            get(route::calendar::residual::handler),
        )
        .route("/calendar/organic", get(route::calendar::organic::handler))
        .route(
            "/calendar/recyclable",
            get(route::calendar::recyclable::handler),
        )
        .route("/calendar/paper", get(route::calendar::paper::handler))
        .route("/calendar/bulky", get(route::calendar::bulky::handler))
        .route_service("/*path", ServeDir::new(SERVE_DIR))
        .route_service("/", ServeFile::new(format!("{}/index.html", SERVE_DIR)));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8008));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
