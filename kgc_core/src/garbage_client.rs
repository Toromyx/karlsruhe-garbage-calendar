//! This client fetches garbage and parses it into waste data.

use std::{
    collections::HashMap,
    io::{BufReader, Cursor},
};

use anyhow::Result;
use bitmask_enum::bitmask;
use chrono::NaiveDate;
use ical::{
    generator::{IcalCalendar, IcalCalendarBuilder, IcalEvent, IcalEventBuilder, Property},
    ical_param, ical_property, IcalParser,
};
use regex::Regex;
use reqwest::Response;

static URL: &str = "https://web6.karlsruhe.de/service/abfall/akal/akal.php";
static PROD_ID: [&str; 2] = ["Abfuhrkalender", "karlsruhe.de"];
static TIMEZONE: &str = "Europe/Berlin";
static FORMAT: &str = "%Y%m%d";

static LABEL_RESIDUAL: &str = "Restmüll";
static LABEL_ORGANIC: &str = "Bioabfall";
static LABEL_RECYCLABLE: &str = "Wertstoff";
static LABEL_PAPER: &str = "Papier";
static LABEL_BULKY: &str = "Sperrmüllabholung";

#[bitmask]
#[bitmask_config(inverted_flags)]
pub enum WasteTypeBitmask {
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
    excluded_waste_types: WasteTypeBitmask,
) -> Result<IcalCalendar> {
    let response = get_response(street, street_number).await?;
    let waste_data = parse(&response.text().await?)?;
    let calendar = get_calendar(street, street_number, waste_data, excluded_waste_types);
    Ok(calendar)
}

/// Get the iCalendar response from the official server.
async fn get_response(street: &str, street_number: &str) -> Result<Response> {
    let client = reqwest::Client::new();
    let response = client
        .post(URL)
        .form(&HashMap::from([
            ("strasse_n", street),
            ("hausnr", street_number),
            ("ical", "+iCalendar"),
        ]))
        .send()
        .await?;
    Ok(response)
}

/// Build the calendar from the waste data.
fn get_calendar(
    street: &str,
    street_number: &str,
    waste_data: WasteData,
    excluded_waste_types: WasteTypeBitmask,
) -> IcalCalendar {
    let changed = chrono::Local::now().format("%Y%m%dT%H%M%S").to_string();
    let prod_id_label = match excluded_waste_types {
        WasteTypeBitmask::InvertedResidual => Some(String::from(LABEL_RESIDUAL)),
        WasteTypeBitmask::InvertedOrganic => Some(String::from(LABEL_ORGANIC)),
        WasteTypeBitmask::InvertedRecyclable => Some(String::from(LABEL_RECYCLABLE)),
        WasteTypeBitmask::InvertedPaper => Some(String::from(LABEL_PAPER)),
        WasteTypeBitmask::InvertedBulky => Some(String::from(LABEL_BULKY)),
        _ => None,
    };
    let mut calendar = IcalCalendarBuilder::version("2.0")
        .gregorian()
        .prodid(prod_id(prod_id_label))
        .build();
    for (label, dates, waste_type_bitmask) in [
        (
            LABEL_RESIDUAL,
            waste_data.residual_waste,
            WasteTypeBitmask::Residual,
        ),
        (
            LABEL_ORGANIC,
            waste_data.organic_waste,
            WasteTypeBitmask::Organic,
        ),
        (
            LABEL_RECYCLABLE,
            waste_data.recyclable_waste,
            WasteTypeBitmask::Recyclable,
        ),
        (LABEL_PAPER, waste_data.paper_waste, WasteTypeBitmask::Paper),
        (
            LABEL_BULKY,
            waste_data.bulky_waste.into_iter().collect(),
            WasteTypeBitmask::Bulky,
        ),
    ] {
        if let (Some(event), false) = (
            get_event(street, street_number, dates, label, &changed),
            excluded_waste_types.contains(waste_type_bitmask),
        ) {
            calendar.events.push(event);
        }
    }
    calendar
}

/// Build an event from a vector of dates.
fn get_event(
    street: &str,
    street_number: &str,
    dates: Vec<NaiveDate>,
    summary: &str,
    changed: &str,
) -> Option<IcalEvent> {
    if dates.is_empty() {
        return None;
    }
    Some(
        IcalEventBuilder::tzid(TIMEZONE)
            .uid(uid(street, street_number, summary))
            .changed(changed)
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
            .set(ical_property!("TRANSP", "TRANSPARENT"))
            .build(),
    )
}

trait GetIcalProperty {
    fn get_ical_property_value(&self, name: &str) -> Option<&String>;
}

impl GetIcalProperty for IcalEvent {
    fn get_ical_property_value(&self, name: &str) -> Option<&String> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .and_then(|property| property.value.as_ref())
    }
}

/// Parse the official iCalendar file to extract the waste data.
fn parse(ics: &str) -> Result<WasteData> {
    let parser = IcalParser::new(BufReader::new(Cursor::new(ics)));
    let mut residual_waste_dates: Vec<NaiveDate> = vec![];
    let mut organic_waste_dates: Vec<NaiveDate> = vec![];
    let mut recyclable_waste_dates: Vec<NaiveDate> = vec![];
    let mut paper_waste_dates: Vec<NaiveDate> = vec![];
    let mut bulky_waste_dates: Vec<NaiveDate> = vec![];
    for ical_calendar_result in parser {
        let ical_calendar = ical_calendar_result?;
        for ical_event in ical_calendar.events {
            let summary_option = ical_event.get_ical_property_value("SUMMARY");
            let date_option = ical_event
                .get_ical_property_value("DTSTART")
                .and_then(|dt_start| {
                    NaiveDate::from_ymd_opt(
                        dt_start[0..4].parse().ok()?,
                        dt_start[4..6].parse().ok()?,
                        dt_start[6..8].parse().ok()?,
                    )
                });
            let (Some(summary), Some(date)) = (summary_option, date_option) else {
                continue;
            };
            if summary.contains(LABEL_RESIDUAL) {
                residual_waste_dates.push(date);
            }
            if summary.contains(LABEL_ORGANIC) {
                organic_waste_dates.push(date);
            }
            if summary.contains(LABEL_RECYCLABLE) {
                recyclable_waste_dates.push(date);
            }
            if summary.contains(LABEL_PAPER) {
                paper_waste_dates.push(date);
            }
            if summary.contains(LABEL_BULKY) {
                bulky_waste_dates.push(date);
            }
        }
    }
    let waste_data = WasteData {
        residual_waste: residual_waste_dates,
        organic_waste: organic_waste_dates,
        recyclable_waste: recyclable_waste_dates,
        paper_waste: paper_waste_dates,
        bulky_waste: bulky_waste_dates,
    };
    Ok(waste_data)
}

fn prod_id(label: Option<String>) -> String {
    let mut strings: Vec<String> = Vec::from(PROD_ID).into_iter().map(String::from).collect();
    if let Some(label) = label {
        strings.splice(0..0, [label]);
    }
    strings.splice(0..0, [String::from("-")]);
    strings.join("//")
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
    pub bulky_waste: Vec<NaiveDate>,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::NaiveDate;
    use ical::generator::{IcalCalendar, IcalEvent};

    use crate::garbage_client::{
        get, get_calendar, parse, WasteData, WasteTypeBitmask, LABEL_BULKY, LABEL_ORGANIC,
        LABEL_RECYCLABLE, LABEL_RESIDUAL,
    };

    fn get_test_waste_data() -> WasteData {
        WasteData {
            residual_waste: vec![
                NaiveDate::from_str("2023-06-30").unwrap(),
                NaiveDate::from_str("2023-07-14").unwrap(),
                NaiveDate::from_str("2023-07-28").unwrap(),
                NaiveDate::from_str("2023-08-11").unwrap(),
                NaiveDate::from_str("2023-08-25").unwrap(),
                NaiveDate::from_str("2023-09-08").unwrap(),
                NaiveDate::from_str("2023-09-22").unwrap(),
                NaiveDate::from_str("2023-10-06").unwrap(),
                NaiveDate::from_str("2023-10-20").unwrap(),
                NaiveDate::from_str("2023-11-03").unwrap(),
                NaiveDate::from_str("2023-11-17").unwrap(),
                NaiveDate::from_str("2023-12-01").unwrap(),
                NaiveDate::from_str("2023-12-15").unwrap(),
                NaiveDate::from_str("2023-12-30").unwrap(),
            ],
            organic_waste: vec![
                NaiveDate::from_str("2023-06-28").unwrap(),
                NaiveDate::from_str("2023-07-05").unwrap(),
                NaiveDate::from_str("2023-07-12").unwrap(),
                NaiveDate::from_str("2023-07-19").unwrap(),
                NaiveDate::from_str("2023-07-26").unwrap(),
                NaiveDate::from_str("2023-08-02").unwrap(),
                NaiveDate::from_str("2023-08-09").unwrap(),
                NaiveDate::from_str("2023-08-16").unwrap(),
                NaiveDate::from_str("2023-08-23").unwrap(),
                NaiveDate::from_str("2023-08-30").unwrap(),
                NaiveDate::from_str("2023-09-06").unwrap(),
                NaiveDate::from_str("2023-09-13").unwrap(),
                NaiveDate::from_str("2023-09-20").unwrap(),
                NaiveDate::from_str("2023-09-27").unwrap(),
                NaiveDate::from_str("2023-10-05").unwrap(),
                NaiveDate::from_str("2023-10-11").unwrap(),
                NaiveDate::from_str("2023-10-18").unwrap(),
                NaiveDate::from_str("2023-10-25").unwrap(),
                NaiveDate::from_str("2023-11-02").unwrap(),
                NaiveDate::from_str("2023-11-08").unwrap(),
                NaiveDate::from_str("2023-11-15").unwrap(),
                NaiveDate::from_str("2023-11-22").unwrap(),
                NaiveDate::from_str("2023-11-29").unwrap(),
                NaiveDate::from_str("2023-12-06").unwrap(),
                NaiveDate::from_str("2023-12-13").unwrap(),
                NaiveDate::from_str("2023-12-20").unwrap(),
                NaiveDate::from_str("2023-12-29").unwrap(),
            ],
            recyclable_waste: vec![
                NaiveDate::from_str("2023-07-06").unwrap(),
                NaiveDate::from_str("2023-07-20").unwrap(),
                NaiveDate::from_str("2023-08-03").unwrap(),
                NaiveDate::from_str("2023-08-17").unwrap(),
                NaiveDate::from_str("2023-08-31").unwrap(),
                NaiveDate::from_str("2023-09-14").unwrap(),
                NaiveDate::from_str("2023-09-28").unwrap(),
                NaiveDate::from_str("2023-10-12").unwrap(),
                NaiveDate::from_str("2023-10-26").unwrap(),
                NaiveDate::from_str("2023-11-09").unwrap(),
                NaiveDate::from_str("2023-11-23").unwrap(),
                NaiveDate::from_str("2023-12-07").unwrap(),
                NaiveDate::from_str("2023-12-21").unwrap(),
            ],
            paper_waste: vec![
                NaiveDate::from_str("2023-07-12").unwrap(),
                NaiveDate::from_str("2023-08-09").unwrap(),
                NaiveDate::from_str("2023-09-06").unwrap(),
                NaiveDate::from_str("2023-10-04").unwrap(),
                NaiveDate::from_str("2023-10-31").unwrap(),
                NaiveDate::from_str("2023-11-29").unwrap(),
                NaiveDate::from_str("2023-12-28").unwrap(),
            ],
            bulky_waste: vec![NaiveDate::from_str("2023-07-12").unwrap()],
        }
    }

    /// Test whether requests can be sent and the resulting calendar contains something.
    ///
    /// This is an online test!
    #[tokio::test]
    async fn test_get() {
        let calendar = get("Schloßplatz", "1", WasteTypeBitmask::none())
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
        let calendar = get_calendar("street", "69", waste_data, WasteTypeBitmask::none());
        assert_eq!(calendar.events.len(), 5);
        let residual_dtstart = get_property_value_of_event(&calendar, "DTSTART", LABEL_RESIDUAL);
        assert_eq!(residual_dtstart, "20230630");
        let recyclable_rdate = get_property_value_of_event(&calendar, "RDATE", LABEL_RECYCLABLE);
        assert_eq!(recyclable_rdate, "20230706,20230720,20230803,20230817,20230831,20230914,20230928,20231012,20231026,20231109,20231123,20231207,20231221");
    }

    #[test]
    fn test_get_calendar_exclusion() {
        let waste_data = get_test_waste_data();
        let calendar = get_calendar("street", "69", waste_data, WasteTypeBitmask::Bulky);
        assert_eq!(calendar.events.len(), 4);
        let bulky_found = find_event(&calendar, LABEL_BULKY).is_some();
        assert_eq!(bulky_found, false);

        let waste_data = get_test_waste_data();
        let calendar = get_calendar(
            "street",
            "69",
            waste_data,
            WasteTypeBitmask::Recyclable | WasteTypeBitmask::Organic,
        );
        assert_eq!(calendar.events.len(), 3);
        let recyclable_found = find_event(&calendar, LABEL_RECYCLABLE).is_some();
        assert_eq!(recyclable_found, false);
        let organic_found = find_event(&calendar, LABEL_ORGANIC).is_some();
        assert_eq!(organic_found, false);
    }

    /// Test whether the ics is parsed correctly.
    ///
    /// This test is offline.
    #[test]
    fn test_parse() {
        let ics = include_str!("garbage_client/tests/ical_calendar.ics");
        let parsed = parse(ics).unwrap();
        let expected = get_test_waste_data();
        println!("{:#?}", parsed);
        assert_eq!(parsed, expected)
    }
}
