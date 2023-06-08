//! This crate tries to implement a CalDAV server serving Karlsruhe's garbage collection dates as events.
//!
//! The dates are read from <https://web6.karlsruhe.de/service/abfall/akal/akal.php>.
//! The path and query string are `/calendar?street=<your_street>&street_number=<your_street_number>`.

mod garbage_client;

use std::{collections::HashMap, net::SocketAddr};

use axum::{
    extract::Query,
    http::{header::CONTENT_TYPE, HeaderValue, Request},
    response::Response,
    routing::any,
    Router,
};
use dav_server::{
    davpath::DavPath,
    fs::{DavFileSystem, OpenOptions},
    memfs::MemFs,
    memls::MemLs,
    DavHandler, DavMethodSet,
};
use ical::generator::Emitter;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/calendar", any(handler));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8008));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

/// Handle calendar requests.
///
/// The `street` and `street_number` must be given in the query string.
async fn handler(
    Query(params): Query<HashMap<String, String>>,
    request: Request<axum::body::Body>,
) -> Response<dav_server::body::Body> {
    let street = params.get("street").unwrap();
    let street_number = params.get("street_number").unwrap();
    let ical_calendar = garbage_client::get(&street, &street_number).await.unwrap();
    let mem_fs = MemFs::new();
    let calendar_dav_path = DavPath::new(request.uri().path()).unwrap();
    let mut dav_file = mem_fs
        .open(
            &calendar_dav_path,
            OpenOptions {
                create: true,
                write: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    dav_file
        .write_bytes(ical_calendar.generate().into())
        .await
        .unwrap();
    let dav_handler = DavHandler::builder()
        .filesystem(mem_fs)
        .locksystem(MemLs::new())
        .methods(DavMethodSet::WEBDAV_RO)
        .build_handler();
    let mut response = dav_handler.handle(request.into()).await;
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE.as_str(),
        HeaderValue::from_str("text/calendar").unwrap(),
    );
    response.into()
}
