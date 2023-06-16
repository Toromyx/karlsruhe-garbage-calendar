use std::net::SocketAddr;

use axum::{routing::get, Router};

mod route;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/calendar", get(route::calendar::handler));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8008));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
