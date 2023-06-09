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

/// Get the HTML response from the official server.
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
            .filter_map(date_from_captures)
            .collect()
    };
    for row_element in rows {
        let Some(type_col) = row_element.select(&type_col_selector).next() else {
            continue;
        };
        let type_col_inner_html = type_col.inner_html();
        let date_col_inner_html_option = row_element
            .select(&date_col_selector)
            .next()
            .map(|date_col| date_col.inner_html());
        let bulky_waste_date_col_inner_html_option = row_element
            .select(&bulky_waste_date_col_selector)
            .next()
            .map(|date_col| date_col.inner_html());
        match (
            date_col_inner_html_option,
            bulky_waste_date_col_inner_html_option,
        ) {
            (Some(date_col_inner_html), _) if type_col_inner_html.contains(LABEL_RESIDUAL) => {
                residual_waste_dates = find_dates(&date_col_inner_html);
            }
            (Some(date_col_inner_html), _) if type_col_inner_html.contains(LABEL_ORGANIC) => {
                organic_waste_dates = find_dates(&date_col_inner_html);
            }
            (Some(date_col_inner_html), _) if type_col_inner_html.contains(LABEL_RECYCLABLE) => {
                recyclable_waste_dates = find_dates(&date_col_inner_html);
            }
            (Some(date_col_inner_html), _) if type_col_inner_html.contains(LABEL_PAPER) => {
                paper_waste_dates = find_dates(&date_col_inner_html);
            }
            (_, Some(bulky_waste_date_col_inner_html))
                if type_col_inner_html.contains(LABEL_BULKY) =>
            {
                bulky_waste_date = bulky_waste_date_regex
                    .captures(&bulky_waste_date_col_inner_html)
                    .and_then(date_from_captures);
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
    pub bulky_waste: Option<NaiveDate>,
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use ical::generator::{IcalCalendar, IcalEvent};

    use crate::garbage_client::{
        get, get_calendar, parse, WasteData, WasteTypeBitmask, LABEL_BULKY, LABEL_ORGANIC,
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
        let calendar = get("Schloßplatz", "1", WasteTypeBitmask::none())
            .await
            .unwrap();
        assert!(!calendar.events.is_empty());
    }

    fn find_event<'a>(calendar: &'a IcalCalendar, summary: &str) -> Option<&'a IcalEvent> {
        calendar.events.iter().find(|event| {
            event.properties.iter().any(|property| {
                property.name == "SUMMARY"
                    && property
                        .value
                        .as_ref()
                        .is_some_and(|value| value == summary)
            })
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
            .find(|property| property.name == property_name)
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
        assert_eq!(residual_dtstart, "20230616");
        let recyclable_rdate = get_property_value_of_event(&calendar, "RDATE", LABEL_RECYCLABLE);
        assert_eq!(recyclable_rdate, "20230607,20230622,20230706");
    }

    #[test]
    fn test_get_calendar_exclusion() {
        let waste_data = get_test_waste_data();
        let calendar = get_calendar("street", "69", waste_data, WasteTypeBitmask::Bulky);
        assert_eq!(calendar.events.len(), 4);
        let bulky_found = find_event(&calendar, LABEL_BULKY).is_some();
        assert!(!bulky_found);

        let waste_data = get_test_waste_data();
        let calendar = get_calendar(
            "street",
            "69",
            waste_data,
            WasteTypeBitmask::Recyclable | WasteTypeBitmask::Organic,
        );
        assert_eq!(calendar.events.len(), 3);
        let recyclable_found = find_event(&calendar, LABEL_RECYCLABLE).is_some();
        assert!(!recyclable_found);
        let organic_found = find_event(&calendar, LABEL_ORGANIC).is_some();
        assert!(!organic_found);
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
