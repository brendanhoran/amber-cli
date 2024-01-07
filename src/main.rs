//! This crate provides a CLI interface to interface to Amber Energy's REST API.
//! Amber energy is an Australia power retailer.
//! This tool will let you query the available endpoint's offered.
//! Fetching the data and returning it as a JSON encoded dataset.
//!
//! [Amber Energy]: https://www.amber.com.au
//! [Amber API documentation]: https://app.amber.com.au/developers/documentation

use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::path::PathBuf;

use tracing::{debug, Instrument};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{prelude::*, EnvFilter};

use amber_client::app_config::AppConfig;
use amber_client::{
    get_prices, get_renewables, get_site_data, get_usage_by_date, get_user_site_id,
    write_data_as_csv_to_file,
};

// Main CLI options
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Full path to config.toml file.
    #[arg(short, long, value_name = "config.toml")]
    config_file: PathBuf,

    /// Enable debug logging, defaults to off.
    #[arg(short, long, default_missing_value("true"), default_value("false"))]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

/// Commands to interact with the Amber API
#[derive(Subcommand, Debug)]
enum Commands {
    /// Display details about your site.
    SiteDetails,
    #[command(subcommand)]
    Price(Window),
    #[command(subcommand)]
    Usage(Dates),
    #[command(subcommand)]
    Renewables(Window),
}

/// Price window to query for data (current, next, previous)
#[derive(Parser, Debug)]
enum Window {
    /// Current interval data.
    Current,
    /// Previous interval data.
    Previous,
    /// Forecast interval data.
    Next,
}
/// Date range to query history data for (Using: yyyy-mm-dd format)
// Not super keen on the way this is structured, but works for now.
// Would like the export options to be more obvious.
#[derive(Clone, Debug, Subcommand)]
enum Dates {
    DateRange {
        /// Start date to query from.
        start_date: String,
        /// End date of query from.
        end_date: String,
        /// [Optional] Path to save/export data in CSV format.
        filename_to_export_to: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    /*
    Not done but would like to do this correctly, hence the code block comment.

    // Set up a date format for tracing
    let tracing_timestamp_format = time::format_description::parse(
        "[year]-[month padding:zero]-[day padding:zero] [hour]:[minute]:[second]",
    )?;

    The following is Not supported..
    https://docs.rs/time/latest/time/struct.UtcOffset.html#method.current_local_offset
    Needs, local crate feature local-offset only
    This is known to cause segfaults, since getenv in localtime_r is not thread safe
    https://github.com/time-rs/time/issues/293

    // Get users local UTC offset
    let local_offset = time::UtcOffset::current_local_offset()?;

    // Build the time stamp format
    let tracing_time = fmt::time::OffsetTime::new(local_offset, tracing_timestamp_format);
    // Then dd tracing_time to fmt::Layer as .with_timer().
    */

    // parse cli input
    let cli_args = Cli::parse();

    // Less then ideal as tracing_subscriber::reload can not update a Layer as a async task
    // https://github.com/tokio-rs/tracing/issues/738#issuecomment-635517004
    // For now if the "-d / --debug" flag is present/true then just overwrite the "RUST_LOG".
    // This will overwrite anything the user as set for this env_var.
    // Print warning via println as tracing_subscriber is not Initializing yet.
    match cli_args.debug {
        true => {
            println!("WARNING!");
            println!(
                "WARNING!{:>25} mode overrides the RUST_LOG environmental variable!",
                "DEBUG"
            );
            println!(
                "WARNING!{0:>25} will be set as the RUST_LOG environmental variable.",
                "DEBUG"
            );
            env::set_var("RUST_LOG", "DEBUG");
            println!(
                "WARNING!{0:>28} environmental variable has been set.",
                "RUST_LOG"
            );
            println!("WARNING!");
        }
        false => (),
    }

    let logging_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    // Set up a default layer for formatting trace/log messages
    let default_layer_format = tracing_subscriber::fmt::layer()
        .compact()
        .with_span_events(FmtSpan::ACTIVE)
        //.with_timer(tracing_time)
        .with_target(false);

    // Configure tracing registry and enable tracing
    tracing_subscriber::registry()
        .with(default_layer_format)
        // Read environment variable "RUST_LOG" to determine log level
        //.with(EnvFilter::from_default_env())
        .with(logging_filter)
        .try_init()?;

    // show actual trace level set for tracing_subscriber.
    debug!(
        "Trace logging enabled. Configured log level is: {}",
        tracing_subscriber::filter::LevelFilter::current()
    );

    // Do not print users API keys to any log event.
    // Due to skipping the 'api_token' field on configured instruments.
    debug!("Debug traces/spans will not print your API key. Due to purposefully skipping the 'api_token' field.");

    // map the CLI argument of "config_file: PathBuf" to a string, using lossy conversion
    // making this is safe to use with non uni-code data.
    // https://doc.rust-lang.org/std/path/struct.Path.html#method.display
    let app_config_file = cli_args.config_file.display().to_string();

    // read config file
    let config = AppConfig::get(app_config_file.clone())
        .instrument(tracing::debug_span!(
            "Load config file",
            "File={}",
            app_config_file
        ))
        .await?;

    // map API token, Amber url and users state from config
    let auth_token = config.apitoken.psk;
    let base_url = config.amberconfig.base_url;
    let users_state = config.userconfig.state;

    // Get the Site ID first, so tha`t we can reuse it later without an additonal API call.
    let site_id = get_user_site_id(base_url.clone(), auth_token.clone()).await?;

    match cli_args.command {
        Commands::Price(Window::Current) => {
            let _window = "current".to_string();
            let current_price_data = get_prices(base_url, auth_token, site_id, _window).await?;
            let current_price_data_json = serde_json::to_string(&current_price_data)?;
            println!("{}", current_price_data_json);
        }
        Commands::Price(Window::Previous) => {
            let _window = "current?previous=1".to_string();
            let current_price_data = get_prices(base_url, auth_token, site_id, _window).await?;
            let current_price_data_json = serde_json::to_string(&current_price_data)?;
            println!("{}", current_price_data_json);
        }

        Commands::Price(Window::Next) => {
            let _window = "current?next=1".to_string();
            let current_price_data = get_prices(base_url, auth_token, site_id, _window).await?;
            let current_price_data_json = serde_json::to_string(&current_price_data)?;
            println!("{}", current_price_data_json);
        }

        Commands::Renewables(Window::Current) => {
            let _window = "current".to_string();
            let renewables_percent_in_grid_data =
                get_renewables(base_url, auth_token, users_state, _window).await?;
            let renewables_percent_in_grid_json =
                serde_json::to_string(&renewables_percent_in_grid_data)?;
            println!("{}", renewables_percent_in_grid_json);
        }

        Commands::Renewables(Window::Previous) => {
            let _window = "current?previous=1".to_string();
            let renewables_percent_in_grid_data =
                get_renewables(base_url, auth_token, users_state, _window).await?;
            let renewables_percent_in_grid_json =
                serde_json::to_string(&renewables_percent_in_grid_data)?;
            println!("{}", renewables_percent_in_grid_json);
        }

        Commands::Renewables(Window::Next) => {
            let _window = "current?next=1".to_string();
            let renewables_percent_in_grid_data =
                get_renewables(base_url, auth_token, users_state, _window).await?;
            let renewables_percent_in_grid_json =
                serde_json::to_string(&renewables_percent_in_grid_data)?;
            println!("{}", renewables_percent_in_grid_json);
        }

        Commands::SiteDetails => {
            let site_data = get_site_data(base_url, auth_token).await?;
            let site_data_json = serde_json::to_string(&site_data)?;
            println!("{}", site_data_json);
        }

        Commands::Usage(Dates::DateRange {
            start_date,
            end_date,
            filename_to_export_to,
        }) => {
            let usage =
                get_usage_by_date(base_url, auth_token, site_id, start_date, end_date).await?;

            // If the Option<path> contains a value then we enter export/save to file mode.
            // Otherwise None will fall back to print to stdout as normal.
            match filename_to_export_to {
                Some(filename) => {
                    let filename_as_string = filename.display().to_string();
                    write_data_as_csv_to_file(filename_as_string, usage.clone()).await?;
                    //println!("file: {:?}", filename);
                }
                None => {
                    let usage_json = serde_json::to_string(&usage)?;
                    println!("{}", usage_json);
                }
            }
        }
    }

    Ok(())
}
