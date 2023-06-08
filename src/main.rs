//! This crate implements an iCalendar server serving Karlsruhe's garbage collection dates as events.
//!
//! The dates are read from <https://web6.karlsruhe.de/service/abfall/akal/akal.php>.
//! The path and query string are `/calendar?street=<your_street>&street_number=<your_street_number>`.

mod garbage_client;

use std::net::SocketAddr;

use axum::{
    extract::Query,
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use ical::generator::Emitter;
use serde::Deserialize;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/calendar", any(handler));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8008));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, Clone, Deserialize)]
struct QueryParams {
    street: String,
    street_number: String,
}

/// Handle calendar requests.
///
/// The `street` and `street_number` must be given in the query string.
async fn handler(
    Query(query_params): Query<QueryParams>,
) -> Result<Response, (StatusCode, String)> {
    let ical_calendar = garbage_client::get(&query_params.street, &query_params.street_number)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let response = ([(CONTENT_TYPE, "text/calendar")], ical_calendar.generate()).into_response();
    Ok(response)
}
