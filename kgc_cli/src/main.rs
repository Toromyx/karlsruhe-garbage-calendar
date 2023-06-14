use std::{env::current_dir, fs::write};

use anyhow::Result;
use clap::Parser;
use kgc_core::{garbage_client, garbage_client::WasteTypeBitmask, ical::generator::Emitter};

#[derive(Debug, Parser)]
pub struct Arguments {
    /// the street
    pub street: String,
    /// the street number
    pub street_number: String,
    /// exclude residual waste collection dates
    #[arg(long)]
    pub exclude_residual: bool,
    /// exclude organic waste collection dates
    #[arg(long)]
    pub exclude_organic: bool,
    /// exclude recyclable waste collection dates
    #[arg(long)]
    pub exclude_recyclable: bool,
    /// exclude paper waste collection dates
    #[arg(long)]
    pub exclude_paper: bool,
    /// exclude bulky waste collection dates
    #[arg(long)]
    pub exclude_bulky: bool,
}

impl From<&Arguments> for WasteTypeBitmask {
    fn from(value: &Arguments) -> Self {
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arguments::parse();
    let calendar = garbage_client::get(
        &args.street,
        &args.street_number,
        WasteTypeBitmask::from(&args),
    )
    .await?;
    let mut path = current_dir()?;
    path.push("calendar.ics");
    write(path, calendar.generate())?;
    Ok(())
}
