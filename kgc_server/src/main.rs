use std::net::SocketAddr;

use axum::{routing::get, Router};

mod route;

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
        .route("/calendar/bulky", get(route::calendar::bulky::handler));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8008));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
