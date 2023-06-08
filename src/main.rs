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

use crate::garbage_client::ExcludeWasteType;

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
    #[serde(default)]
    exclude_residual: bool,
    #[serde(default)]
    exclude_organic: bool,
    #[serde(default)]
    exclude_recyclable: bool,
    #[serde(default)]
    exclude_paper: bool,
    #[serde(default)]
    exclude_bulky: bool,
}

impl From<&QueryParams> for ExcludeWasteType {
    fn from(value: &QueryParams) -> Self {
        let mut exclude_waste_type = ExcludeWasteType::none();
        if value.exclude_residual {
            exclude_waste_type |= ExcludeWasteType::Residual;
        }
        if value.exclude_organic {
            exclude_waste_type |= ExcludeWasteType::Organic;
        }
        if value.exclude_recyclable {
            exclude_waste_type |= ExcludeWasteType::Recyclable;
        }
        if value.exclude_paper {
            exclude_waste_type |= ExcludeWasteType::Paper;
        }
        if value.exclude_bulky {
            exclude_waste_type |= ExcludeWasteType::Bulky;
        }
        exclude_waste_type
    }
}

/// Handle calendar requests.
///
/// The `street` and `street_number` must be given in the query string.
async fn handler(
    Query(query_params): Query<QueryParams>,
) -> Result<Response, (StatusCode, String)> {
    let ical_calendar = garbage_client::get(
        &query_params.street,
        &query_params.street_number,
        ExcludeWasteType::from(&query_params),
    )
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let response = ([(CONTENT_TYPE, "text/calendar")], ical_calendar.generate()).into_response();
    Ok(response)
}

#[cfg(test)]
mod tests {
    use crate::{garbage_client::ExcludeWasteType, QueryParams};

    #[test]
    fn test_from_query_params_for_exclude_waste_type() {
        let exclude_query_params = QueryParams {
            street: "".to_string(),
            street_number: "".to_string(),
            exclude_residual: false,
            exclude_organic: false,
            exclude_recyclable: false,
            exclude_paper: false,
            exclude_bulky: false,
        };
        let exclude_from_query_params = ExcludeWasteType::from(&exclude_query_params);
        assert_eq!(exclude_from_query_params, ExcludeWasteType::none());
        let exclude_query_params = QueryParams {
            street: "".to_string(),
            street_number: "".to_string(),
            exclude_residual: true,
            exclude_organic: false,
            exclude_recyclable: false,
            exclude_paper: false,
            exclude_bulky: false,
        };
        let exclude_from_query_params = ExcludeWasteType::from(&exclude_query_params);
        assert_eq!(exclude_from_query_params, ExcludeWasteType::Residual);
        let exclude_query_params = QueryParams {
            street: "".to_string(),
            street_number: "".to_string(),
            exclude_residual: false,
            exclude_organic: true,
            exclude_recyclable: false,
            exclude_paper: false,
            exclude_bulky: false,
        };
        let exclude_from_query_params = ExcludeWasteType::from(&exclude_query_params);
        assert_eq!(exclude_from_query_params, ExcludeWasteType::Organic);
        let exclude_query_params = QueryParams {
            street: "".to_string(),
            street_number: "".to_string(),
            exclude_residual: false,
            exclude_organic: false,
            exclude_recyclable: true,
            exclude_paper: true,
            exclude_bulky: true,
        };
        let exclude_from_query_params = ExcludeWasteType::from(&exclude_query_params);
        assert_eq!(
            exclude_from_query_params,
            ExcludeWasteType::Recyclable
                .or(ExcludeWasteType::Paper)
                .or(ExcludeWasteType::Bulky)
        );
    }
}
