//! This crate implements an iCalendar server serving Karlsruhe's garbage collection dates as events.
//!
//! The dates are read from <https://web6.karlsruhe.de/service/abfall/akal/akal.php>.
//! The path and query string are `/calendar?street=<your_street>&street_number=<your_street_number>`.

mod garbage_client;
mod handler;

use std::net::SocketAddr;

use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/calendar", get(handler::handler));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8008));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
