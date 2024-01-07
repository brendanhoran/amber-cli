pub mod app_config;
pub mod rest_client;

use anyhow::Result;
use chrono::NaiveDate;
use csv::WriterBuilder;
use std::process;

use tracing::info;

use rest_client::{PriceData, RenewablesData, RestClient, SiteDetails, UsageData};

/// Function to get and return only the users Site ID.
#[tracing::instrument(level = "debug", skip(auth_token))]
pub async fn get_user_site_id(base_url: String, auth_token: String) -> Result<String> {
    let user_site_data = get_site_data(base_url, auth_token).await?;
    let user_site_id = user_site_data[0].id.clone();
    Ok(user_site_id)
}

/// Function to get the Site data
#[tracing::instrument(level = "debug", skip(auth_token))]
pub async fn get_site_data(base_url: String, auth_token: String) -> Result<Vec<SiteDetails>> {
    let sites_url = format!("{}/sites", base_url);
    let mut user_site_details = RestClient::new_client(sites_url, auth_token.clone());
    let user_site_data = user_site_details.get_site_data().await?;

    Ok(user_site_data)
}

// Current
// https://api.amber.com.au/v1/sites/SITE_ID/prices/current?resolution=30

// next
// https://api.amber.com.au/v1/sites/SITE_ID/prices/current?next=1&resolution=30

// previous
// https://api.amber.com.au/v1/sites/SITE_ID/prices/current?previous=1&resolution=30'

/// Function to get a window of prices. Based the users input.
#[tracing::instrument(level = "debug", skip(auth_token))]
pub async fn get_prices(
    base_url: String,
    auth_token: String,
    site_id: String,
    window: String,
) -> Result<Vec<PriceData>> {
    let price_url = format!(
        "{}/sites/{}/prices/{}?&resolution=30",
        base_url, site_id, window
    );
    let mut current_price_details = RestClient::new_client(price_url, auth_token.clone());
    let current_price_data = current_price_details.get_price_data().await?;

    Ok(current_price_data)
}

// get historical usage
// https://api.amber.com.au/v1/sites/SITE_ID/usage?startDate=2023-12-18&endDate=2023-12-19&resolution=30'

/// Function to retrieve historical price data based on a date range supplied by the user.
#[tracing::instrument(level = "debug", skip(auth_token))]
pub async fn get_usage_by_date(
    base_url: String,
    auth_token: String,
    site_id: String,
    start_date: String,
    end_date: String,
) -> Result<Vec<UsageData>> {
    let start_date = parse_date_naive(start_date).await?;
    let end_date = parse_date_naive(end_date).await?;
    let usage_data_url = format!(
        "{}/sites/{}/usage?startDate={}&endDate={}&resolution=30'",
        base_url, site_id, start_date, end_date
    );
    let mut usage_details = RestClient::new_client(usage_data_url, auth_token.clone());
    let usage_data = usage_details.get_usage_data().await?;
    Ok(usage_data)
}

// https://api.amber.com.au/v1/state/STATE/renewables/current=30'
// also supports "previous=" and "next=" , "/renewables/current?previous=1&resolution=30"

/// Function to get percentage of renewables used in the grid for a given state and a given window.
#[tracing::instrument(level = "debug", skip(auth_token))]
pub async fn get_renewables(
    base_url: String,
    auth_token: String,
    state: String,
    window: String,
) -> Result<Vec<RenewablesData>> {
    let price_url = format!(
        "{}/state/{}/renewables/{}?&resolution=30",
        base_url, state, window
    );
    let renewables_request = RestClient::new_client(price_url, auth_token.clone());
    let renewables_data = renewables_request.get_renewables_data().await?;
    Ok(renewables_data)
}

/// Function to validate the user has supplied the date in the correct format and that
/// the date is a valid calender date.
/// Will exit the application of the date format is wrong or invalid.
#[tracing::instrument(level = "debug")]
pub async fn parse_date_naive(date: String) -> Result<String> {
    let naive_date = match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_error) => {
            eprintln!("Date must be in the format of year-month-day/yyyy-mm-dd, input of {}, does not match requirements", date);
            eprintln!("Can not querity Amber Api, exiting.");
            //FIXME: unwind stack correctly
            // this is not ideal, as it will not unwind the stack and makes it hard to test
            // See sysexits.h for exit code 65: "EX_DATAERR".
            process::exit(65);
        }
    };

    let valid_date = naive_date.to_string();
    Ok(valid_date)
}

/// CVS writer for historical data
#[tracing::instrument(level = "debug")]
pub async fn write_data_as_csv_to_file(file_name: String, data: Vec<UsageData>) -> Result<()> {
    // We must use the builder to disable auto header generation.
    // See: https://docs.rs/csv/latest/csv/struct.Writer.html#structs
    // struct felid that causes this is "pub tariff_information: TariffInformation"
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .from_path(file_name.clone())?;

    info!("Writing to file: {}", file_name);

    // Write headers for usage data
    writer.write_record([
        "Type",
        "duration",
        "date",
        "end_date",
        "quality",
        "kwh",
        "nem_time",
        "per_kwh",
        "channel_type",
        "channel_identifier",
        "cost",
        "renewables",
        "spot_per_kwh",
        "start_time",
        "spike_status",
        "tariff_information",
        "descriptor",
    ])?;

    info!("Writing dataset headers to file...");

    info!("Startng to write records to file...");
    for data_point in data {
        writer.serialize(data_point)?;
        writer.flush()?;
    }
    info!("Finished writing records to file");
    Ok(())
}
