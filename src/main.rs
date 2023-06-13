//! This crate implements an iCalendar server serving Karlsruhe's garbage collection dates as events.
//! It also implements a CLI to just get a single iCalendar file.
//!
//! The dates are read from <https://web6.karlsruhe.de/service/abfall/akal/akal.php>.

mod cli;
mod garbage_client;
mod handler;

use std::net::SocketAddr;

use anyhow::Result;
use axum::{routing::get, Router};
use clap::Parser;
use cli::Arguments;

use crate::cli::run;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arguments::parse();
    if let Some(command) = args.command {
        run(command).await?;
    } else {
        let app = Router::new().route("/calendar", get(handler::handler));
        let addr = SocketAddr::from(([0, 0, 0, 0], 8008));
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
    Ok(())
}
