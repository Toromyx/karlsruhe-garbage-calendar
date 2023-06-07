//! This client fetches garbage and parses it into waste data.

use std::collections::HashMap;

use anyhow::Result;
use chrono::NaiveDate;
use ical::{
    generator::{IcalCalendar, IcalCalendarBuilder, IcalEventBuilder, Property},
    ical_property,
};
use regex::{Captures, Regex};
use scraper::{Html, Selector};

static PROD_ID: &str = "-//Abfuhrkalender//karlsruhe.de";
static TIMEZONE: &str = "Europe/Berlin";
static FORMAT: &str = "%Y%m%d";

/// Get the calendar for a specific street and street number.
pub async fn get(street: &str, street_number: &str) -> Result<IcalCalendar> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://web6.karlsruhe.de/service/abfall/akal/akal.php")
        .form(&HashMap::from([
            ("strasse_n", street),
            ("hausnr", street_number),
        ]))
        .send()
        .await?;
    let waste_data = parse(&response.text().await?)?;
    let changed = chrono::Local::now().format(FORMAT).to_string();
    let mut calendar = IcalCalendarBuilder::version("2.0")
        .gregorian()
        .prodid(PROD_ID)
        .build();
    let build_event = |date: NaiveDate, summary: &str| {
        IcalEventBuilder::tzid(TIMEZONE)
            .uid(uid(street, street_number, summary, &date))
            .changed(&changed)
            .one_day(date.format(FORMAT).to_string())
            .set(ical_property!("SUMMARY", summary))
            .build()
    };
    for date in waste_data.residual_waste {
        calendar.events.push(build_event(date, "Restmüll"));
    }
    for date in waste_data.organic_waste {
        calendar.events.push(build_event(date, "Bioabfall"));
    }
    for date in waste_data.recyclable_waste {
        calendar.events.push(build_event(date, "Wertstoff"));
    }
    for date in waste_data.paper_waste {
        calendar.events.push(build_event(date, "Papier"));
    }
    if let Some(date) = waste_data.bulky_waste {
        calendar.events.push(build_event(date, "Sperrmüllabholung"));
    }
    Ok(calendar)
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
            >\s* # the ending of the previous closing tag
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
        Regex::new(r"(?P<day>\d{2})\.(?P<month>\d{2})\.(?P<year>\d{4})").unwrap();
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
            _ if type_col_inner_html.contains("Restmüll") => {
                let Some(date_col) = row_element.select(&date_col_selector).next() else {
                    break;
                };
                let date_col_inner_html = date_col.inner_html();
                residual_waste_dates = find_dates(&date_col_inner_html);
            }
            _ if type_col_inner_html.contains("Bioabfall") => {
                let Some(date_col) = row_element.select(&date_col_selector).next() else {
                    break;
                };
                let date_col_inner_html = date_col.inner_html();
                organic_waste_dates = find_dates(&date_col_inner_html);
            }
            _ if type_col_inner_html.contains("Wertstoff") => {
                let Some(date_col) = row_element.select(&date_col_selector).next() else {
                    break;
                };
                let date_col_inner_html = date_col.inner_html();
                recyclable_waste_dates = find_dates(&date_col_inner_html);
            }
            _ if type_col_inner_html.contains("Papier") => {
                let Some(date_col) = row_element.select(&date_col_selector).next() else {
                    break;
                };
                let date_col_inner_html = date_col.inner_html();
                paper_waste_dates = find_dates(&date_col_inner_html);
            }
            _ if type_col_inner_html.contains("Sperrmüllabholung") => {
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

/// Get a unique id for a specific waste collection date at a specific location.
///
/// Changing this function is a breaking change!  
fn uid(street: &str, street_number: &str, summary: &str, date: &NaiveDate) -> String {
    format!("{} {} {} {}", street, street_number, summary, date)
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

    use crate::garbage_client::{get, parse, WasteData};

    /// Test whether requests can be sent and the resulting calendar contains something.
    ///
    /// This is an online test!
    #[tokio::test]
    async fn test_get() {
        let calendar = get("Schloßplatz", "1").await.unwrap();
        assert!(calendar.events.len() > 0);
    }

    /// Test whether the HTML is parsed correctly.
    ///
    /// This test is offline.
    #[test]
    fn test_parse() {
        let html = include_str!("garbage_client/tests/response.html");
        let parsed = parse(html).unwrap();
        let expected = WasteData {
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
        };
        assert_eq!(parsed, expected)
    }
}
