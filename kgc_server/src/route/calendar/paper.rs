use axum::{extract::Query, http::StatusCode, response::Response};
use kgc_core::garbage_client::WasteTypeBitmask;

use crate::route::calendar::{handle, StreetQueryParams};

pub async fn handler(
    Query(street_query_params): Query<StreetQueryParams>,
) -> Result<Response, (StatusCode, String)> {
    handle(&street_query_params, WasteTypeBitmask::PaperInverted).await
}
