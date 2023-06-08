//! This client fetches garbage and parses it into waste data.

use std::collections::HashMap;

use anyhow::Result;
use bitmask_enum::bitmask;
use chrono::NaiveDate;
use ical::{
    generator::{IcalCalendar, IcalCalendarBuilder, IcalEvent, IcalEventBuilder, Property},
    ical_param, ical_property,
};
use regex::{Captures, Regex};
use reqwest::Response;
use scraper::{Html, Selector};

static URL: &str = "https://web6.karlsruhe.de/service/abfall/akal/akal.php";
static PROD_ID: &str = "-//Abfuhrkalender//karlsruhe.de";
static TIMEZONE: &str = "Europe/Berlin";
static FORMAT: &str = "%Y%m%d";

static LABEL_RESIDUAL: &str = "Restmüll";
static LABEL_ORGANIC: &str = "Bioabfall";
static LABEL_RECYCLABLE: &str = "Wertstoff";
static LABEL_PAPER: &str = "Papier";
static LABEL_BULKY: &str = "Sperrmüllabholung";

#[bitmask]
pub enum ExcludeWasteType {
    Residual,
    Organic,
    Recyclable,
    Paper,
    Bulky,
}

/// Get the calendar for a specific street and street number.
pub async fn get(
    street: &str,
    street_number: &str,
    exclude_waste_type: ExcludeWasteType,
) -> Result<IcalCalendar> {
    let response = get_response(street, street_number).await?;
    let waste_data = parse(&response.text().await?)?;
    let calendar = get_calendar(street, street_number, waste_data, exclude_waste_type);
    Ok(calendar)
}

async fn get_response(street: &str, street_number: &str) -> Result<Response> {
    let client = reqwest::Client::new();
    let response = client
        .post(URL)
        .form(&HashMap::from([
            ("strasse_n", street),
            ("hausnr", street_number),
        ]))
        .send()
        .await?;
    Ok(response)
}

fn get_calendar(
    street: &str,
    street_number: &str,
    waste_data: WasteData,
    exclude_waste_type: ExcludeWasteType,
) -> IcalCalendar {
    let changed = chrono::Local::now().format("%Y%m%dT%H%M%S").to_string();
    let mut calendar = IcalCalendarBuilder::version("2.0")
        .gregorian()
        .prodid(PROD_ID)
        .build();
    let build_event = |dates: Vec<NaiveDate>, summary: &str| -> Option<IcalEvent> {
        if dates.len() == 0 {
            return None;
        }
        Some(
            IcalEventBuilder::tzid(TIMEZONE)
                .uid(uid(street, street_number, summary))
                .changed(&changed)
                .one_day(dates.get(0).unwrap().format(FORMAT).to_string())
                .set(ical_property!("SUMMARY", summary))
                .set(ical_property!(
                    "RDATE",
                    dates
                        .into_iter()
                        .map(|date| date.format(FORMAT).to_string())
                        .collect::<Vec<String>>()
                        .join(","),
                    ical_param!("VALUE", "DATE")
                ))
                .set(ical_property!(
                    "LOCATION",
                    format!("{street} {street_number}, Karlsruhe")
                ))
                .set(ical_property!("DESCRIPTION", URL))
                .build(),
        )
    };
    if let (Some(event), false) = (
        build_event(waste_data.residual_waste, LABEL_RESIDUAL),
        exclude_waste_type.contains(ExcludeWasteType::Residual),
    ) {
        calendar.events.push(event);
    }
    if let (Some(event), false) = (
        build_event(waste_data.organic_waste, LABEL_ORGANIC),
        exclude_waste_type.contains(ExcludeWasteType::Organic),
    ) {
        calendar.events.push(event);
    }
    if let (Some(event), false) = (
        build_event(waste_data.recyclable_waste, LABEL_RECYCLABLE),
        exclude_waste_type.contains(ExcludeWasteType::Recyclable),
    ) {
        calendar.events.push(event);
    }
    if let (Some(event), false) = (
        build_event(waste_data.paper_waste, LABEL_PAPER),
        exclude_waste_type.contains(ExcludeWasteType::Paper),
    ) {
        calendar.events.push(event);
    }
    if let (Some(event), false) = (
        build_event(waste_data.bulky_waste.into_iter().collect(), LABEL_BULKY),
        exclude_waste_type.contains(ExcludeWasteType::Bulky),
    ) {
        calendar.events.push(event);
    }
    calendar
}

/// Parse the garbage HTML to usable waste data.
fn parse(html: &str) -> Result<WasteData> {
    let dom = Html::parse_document(html);
    let row_selector = Selector::parse(".row").unwrap();
    let rows = dom.select(&row_selector);
    let mut residual_waste_dates: Vec<NaiveDate> = vec![];
    let mut organic_waste_dates: Vec<NaiveDate> = vec![];
    let mut recyclable_waste_dates: Vec<NaiveDate> = vec![];
    let mut paper_waste_dates: Vec<NaiveDate> = vec![];
    let mut bulky_waste_date: Option<NaiveDate> = None;
    let type_col_selector = Selector::parse(".col_3-2").unwrap();
    let date_col_selector = Selector::parse(".col_3-3").unwrap();
    let bulky_waste_date_col_selector = Selector::parse(".col_4-3").unwrap();
    let date_regex = Regex::new(
        r"(?x)
            >\s* # the ending of the previous tag
            \w{2}\.\s # the day of the week in short notation with a dot and a space
            den\s
            (?P<day>\d{2}) # the day
            \.
            (?P<month>\d{2}) # the month
            \.
            (?P<year>\d{4}) # the year
        ",
    )
    .unwrap();
    let bulky_waste_date_regex =
        Regex::new(r">\s*(?P<day>\d{2})\.(?P<month>\d{2})\.(?P<year>\d{4})").unwrap();
    let date_from_captures = |captures: Captures| -> Option<NaiveDate> {
        let day: u32 = captures["day"].parse().unwrap();
        let month: u32 = captures["month"].parse().unwrap();
        let year: i32 = captures["year"].parse().unwrap();
        NaiveDate::from_ymd_opt(year, month, day)
    };
    let find_dates = |inner_html: &str| -> Vec<NaiveDate> {
        date_regex
            .captures_iter(inner_html)
            .into_iter()
            .filter_map(date_from_captures)
            .collect()
    };
    for row_element in rows {
        let Some(type_col) = row_element.select(&type_col_selector).next() else {
            continue;
        };
        let type_col_inner_html = type_col.inner_html();
        match () {
            _ if type_col_inner_html.contains(LABEL_RESIDUAL) => {
                let Some(date_col) = row_element.select(&date_col_selector).next() else {
                    break;
                };
                let date_col_inner_html = date_col.inner_html();
                residual_waste_dates = find_dates(&date_col_inner_html);
            }
            _ if type_col_inner_html.contains(LABEL_ORGANIC) => {
                let Some(date_col) = row_element.select(&date_col_selector).next() else {
                    break;
                };
                let date_col_inner_html = date_col.inner_html();
                organic_waste_dates = find_dates(&date_col_inner_html);
            }
            _ if type_col_inner_html.contains(LABEL_RECYCLABLE) => {
                let Some(date_col) = row_element.select(&date_col_selector).next() else {
                    break;
                };
                let date_col_inner_html = date_col.inner_html();
                recyclable_waste_dates = find_dates(&date_col_inner_html);
            }
            _ if type_col_inner_html.contains(LABEL_PAPER) => {
                let Some(date_col) = row_element.select(&date_col_selector).next() else {
                    break;
                };
                let date_col_inner_html = date_col.inner_html();
                paper_waste_dates = find_dates(&date_col_inner_html);
            }
            _ if type_col_inner_html.contains(LABEL_BULKY) => {
                let Some(date_col) = row_element.select(&bulky_waste_date_col_selector).next() else {
                    break;
                };
                bulky_waste_date = bulky_waste_date_regex
                    .captures(&date_col.inner_html())
                    .map(date_from_captures)
                    .flatten();
            }
            _ => continue,
        }
    }
    let waste_data = WasteData {
        residual_waste: residual_waste_dates,
        organic_waste: organic_waste_dates,
        recyclable_waste: recyclable_waste_dates,
        paper_waste: paper_waste_dates,
        bulky_waste: bulky_waste_date,
    };
    Ok(waste_data)
}

/// Get a unique id for a specific waste collection type at a specific location.
///
/// Changing this function is a breaking change!  
fn uid(street: &str, street_number: &str, summary: &str) -> String {
    let whitespace_regex = Regex::new(r"\s+").unwrap();
    let whitespace_rep = "-";
    let street = whitespace_regex.replace_all(street, whitespace_rep);
    let street_number = whitespace_regex.replace_all(street_number, whitespace_rep);
    let summary = whitespace_regex.replace_all(summary, whitespace_rep);
    format!("Abfuhrkalender_{street}_{street_number}_{summary}@karlsruhe.de")
}

/// This is the data which can be extracted from the official website.
#[derive(Debug, PartialEq)]
struct WasteData {
    pub residual_waste: Vec<NaiveDate>,
    pub organic_waste: Vec<NaiveDate>,
    pub recyclable_waste: Vec<NaiveDate>,
    pub paper_waste: Vec<NaiveDate>,
    pub bulky_waste: Option<NaiveDate>,
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use ical::generator::{IcalCalendar, IcalEvent};

    use crate::garbage_client::{
        get, get_calendar, parse, ExcludeWasteType, WasteData, LABEL_BULKY, LABEL_ORGANIC,
        LABEL_RECYCLABLE, LABEL_RESIDUAL,
    };

    fn get_test_waste_data() -> WasteData {
        WasteData {
            residual_waste: vec![
                NaiveDate::from_ymd_opt(2023, 6, 16).unwrap(),
                NaiveDate::from_ymd_opt(2023, 6, 29).unwrap(),
                NaiveDate::from_ymd_opt(2023, 7, 14).unwrap(),
            ],
            organic_waste: vec![
                NaiveDate::from_ymd_opt(2023, 6, 7).unwrap(),
                NaiveDate::from_ymd_opt(2023, 6, 14).unwrap(),
                NaiveDate::from_ymd_opt(2023, 6, 21).unwrap(),
            ],
            recyclable_waste: vec![
                NaiveDate::from_ymd_opt(2023, 6, 7).unwrap(),
                NaiveDate::from_ymd_opt(2023, 6, 22).unwrap(),
                NaiveDate::from_ymd_opt(2023, 7, 6).unwrap(),
            ],
            paper_waste: vec![
                NaiveDate::from_ymd_opt(2023, 6, 14).unwrap(),
                NaiveDate::from_ymd_opt(2023, 7, 12).unwrap(),
                NaiveDate::from_ymd_opt(2023, 8, 9).unwrap(),
            ],
            bulky_waste: Some(NaiveDate::from_ymd_opt(2023, 7, 12).unwrap()),
        }
    }

    /// Test whether requests can be sent and the resulting calendar contains something.
    ///
    /// This is an online test!
    #[tokio::test]
    async fn test_get() {
        let calendar = get("Schloßplatz", "1", ExcludeWasteType::none())
            .await
            .unwrap();
        assert!(calendar.events.len() > 0);
    }

    fn find_event<'a>(calendar: &'a IcalCalendar, summary: &str) -> Option<&'a IcalEvent> {
        calendar.events.iter().find(|event| {
            event
                .properties
                .iter()
                .find(|property| {
                    property.name == String::from("SUMMARY")
                        && property
                            .value
                            .as_ref()
                            .is_some_and(|value| value == summary)
                })
                .is_some()
        })
    }

    fn get_property_value_of_event<'a>(
        calendar: &'a IcalCalendar,
        property_name: &str,
        summary: &str,
    ) -> &'a str {
        find_event(calendar, summary)
            .unwrap()
            .properties
            .iter()
            .find(|property| property.name == String::from(property_name))
            .unwrap()
            .value
            .as_ref()
            .unwrap()
    }

    #[test]
    fn test_get_calendar_all() {
        let waste_data = get_test_waste_data();
        let calendar = get_calendar("street", "69", waste_data, ExcludeWasteType::none());
        assert_eq!(calendar.events.len(), 5);
        let residual_dtstart = get_property_value_of_event(&calendar, "DTSTART", LABEL_RESIDUAL);
        assert_eq!(residual_dtstart, "20230616");
        let recyclable_rdate = get_property_value_of_event(&calendar, "RDATE", LABEL_RECYCLABLE);
        assert_eq!(recyclable_rdate, "20230607,20230622,20230706");
    }

    #[test]
    fn test_get_calendar_exclusion() {
        let waste_data = get_test_waste_data();
        let calendar = get_calendar("street", "69", waste_data, ExcludeWasteType::Bulky);
        assert_eq!(calendar.events.len(), 4);
        let bulky_found = find_event(&calendar, LABEL_BULKY).is_some();
        assert_eq!(bulky_found, false);

        let waste_data = get_test_waste_data();
        let calendar = get_calendar(
            "street",
            "69",
            waste_data,
            ExcludeWasteType::Recyclable | ExcludeWasteType::Organic,
        );
        assert_eq!(calendar.events.len(), 3);
        let recyclable_found = find_event(&calendar, LABEL_RECYCLABLE).is_some();
        assert_eq!(recyclable_found, false);
        let organic_found = find_event(&calendar, LABEL_ORGANIC).is_some();
        assert_eq!(organic_found, false);
    }

    /// Test whether the HTML is parsed correctly.
    ///
    /// This test is offline.
    #[test]
    fn test_parse() {
        let html = include_str!("garbage_client/tests/response.html");
        let parsed = parse(html).unwrap();
        let expected = get_test_waste_data();
        assert_eq!(parsed, expected)
    }
}
