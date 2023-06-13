//! This module implement the CLI part of the application.

use std::{env::current_dir, fs::write};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use ical::generator::Emitter;

use crate::{garbage_client, garbage_client::WasteTypeBitmask};

#[derive(Debug, Parser)]
#[command()]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Cli {
        #[command(flatten)]
        args: CliArgs,
    },
}

#[derive(Debug, Args)]
pub struct CliArgs {
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

impl From<&CliArgs> for WasteTypeBitmask {
    fn from(value: &CliArgs) -> Self {
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

pub async fn run(command: Command) -> Result<()> {
    match command {
        Command::Cli { args: cli_args } => run_cli(cli_args).await?,
    };
    Ok(())
}

async fn run_cli(cli_args: CliArgs) -> Result<()> {
    let calendar = garbage_client::get(
        &cli_args.street,
        &cli_args.street_number,
        WasteTypeBitmask::from(&cli_args),
    )
    .await?;
    let mut path = current_dir()?;
    path.push("calendar.ics");
    write(path, calendar.generate())?;
    Ok(())
}
