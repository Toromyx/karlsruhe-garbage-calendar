use std::net::SocketAddr;

use axum::{routing::get, Router};

mod handler;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/calendar", get(handler::handler));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8008));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
