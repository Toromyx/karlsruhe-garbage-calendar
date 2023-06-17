pub mod bulky;
pub mod organic;
pub mod paper;
pub mod recyclable;
pub mod residual;

use axum::{
    extract::Query,
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
};
use kgc_core::{garbage_client, garbage_client::WasteTypeBitmask, ical::generator::Emitter};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct QueryParams {
    #[serde(flatten)]
    street_query_params: StreetQueryParams,
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

#[derive(Debug, Clone, Deserialize)]
pub struct StreetQueryParams {
    street: String,
    street_number: String,
}

impl From<&QueryParams> for WasteTypeBitmask {
    fn from(value: &QueryParams) -> Self {
        let mut waste_type_bitmask = WasteTypeBitmask::none();
        if value.exclude_residual {
            waste_type_bitmask |= WasteTypeBitmask::Residual;
        }
        if value.exclude_organic {
            waste_type_bitmask |= WasteTypeBitmask::Organic;
        }
        if value.exclude_recyclable {
            waste_type_bitmask |= WasteTypeBitmask::Recyclable;
        }
        if value.exclude_paper {
            waste_type_bitmask |= WasteTypeBitmask::Paper;
        }
        if value.exclude_bulky {
            waste_type_bitmask |= WasteTypeBitmask::Bulky;
        }
        waste_type_bitmask
    }
}

pub async fn handle(
    street_query_params: &StreetQueryParams,
    excluded_waste_types: WasteTypeBitmask,
) -> Result<Response, (StatusCode, String)> {
    let ical_calendar = garbage_client::get(
        &street_query_params.street,
        &street_query_params.street_number,
        excluded_waste_types,
    )
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let response = ([(CONTENT_TYPE, "text/calendar")], ical_calendar.generate()).into_response();
    Ok(response)
}

/// Handle calendar requests.
///
/// The `street` and `street_number` must be given in the query string.
pub async fn handler(
    Query(query_params): Query<QueryParams>,
) -> Result<Response, (StatusCode, String)> {
    let response = handle(
        &query_params.street_query_params,
        WasteTypeBitmask::from(&query_params),
    )
    .await?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use kgc_core::garbage_client::WasteTypeBitmask;

    use super::*;

    #[test]
    fn test_from_query_params_for_exclude_waste_type() {
        let exclude_query_params = ExcludeQueryParams {
            exclude_residual: false,
            exclude_organic: false,
            exclude_recyclable: false,
            exclude_paper: false,
            exclude_bulky: false,
        };
        let exclude_from_query_params = WasteTypeBitmask::from(&exclude_query_params);
        assert_eq!(exclude_from_query_params, WasteTypeBitmask::none());
        let exclude_query_params = ExcludeQueryParams {
            exclude_residual: true,
            exclude_organic: false,
            exclude_recyclable: false,
            exclude_paper: false,
            exclude_bulky: false,
        };
        let exclude_from_query_params = WasteTypeBitmask::from(&exclude_query_params);
        assert_eq!(exclude_from_query_params, WasteTypeBitmask::Residual);
        let exclude_query_params = ExcludeQueryParams {
            exclude_residual: false,
            exclude_organic: true,
            exclude_recyclable: false,
            exclude_paper: false,
            exclude_bulky: false,
        };
        let exclude_from_query_params = WasteTypeBitmask::from(&exclude_query_params);
        assert_eq!(exclude_from_query_params, WasteTypeBitmask::Organic);
        let exclude_query_params = ExcludeQueryParams {
            exclude_residual: false,
            exclude_organic: false,
            exclude_recyclable: true,
            exclude_paper: true,
            exclude_bulky: true,
        };
        let exclude_from_query_params = WasteTypeBitmask::from(&exclude_query_params);
        assert_eq!(
            exclude_from_query_params,
            WasteTypeBitmask::Recyclable
                .or(WasteTypeBitmask::Paper)
                .or(WasteTypeBitmask::Bulky)
        );
    }
}
