use anyhow::Result;
use iso8601_timestamp::Timestamp;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use amber_client::app_config::AppConfig;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SiteDetails {
    active_from: Timestamp,
    channels: Vec<SiteChannels>,
    id: String,
    network: String,
    nmi: String,
    status: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SiteChannels {
    identifier: String,
    tariff: String,
    // type is a reserved word, so rename it.
    #[serde(rename = "type")]
    tariff_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CurrentPrices {
    // type is a reserved word, so rename it.
    #[serde(rename = "type")]
    interval_type: String,
    date: Timestamp,
    duration: u8,
    start_time: Timestamp,
    end_time: Timestamp,
    nem_time: Timestamp,
    per_kwh: f32,
    renewables: f32,
    spot_per_kwh: f32,
    channel_type: String,
    spike_status: String,
    tariff_information: TariffInformation,
    descriptor: String,
    estimate: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct TariffInformation {
    period: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CurrentUsage {
    #[serde(rename = "type")]
    price_type: String,
    duration: u8,
    date: Timestamp,
    end_time: Timestamp,
    quality: String,
    kwh: f32,
    nem_time: Timestamp,
    per_kwh: f32,
    channel_type: String,
    channel_identifier: String,
    cost: f32,
    renewables: f32,
    spot_per_kwh: f32,
    start_time: Timestamp,
    spike_status: String,
    tariff_information: TariffInformation,
    descriptor: String,
}

#[derive(Clone)]
struct RestClient {
    url: String,
    auth_token: String,
    client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    impl RestClient {
        pub fn new_client(url: String, auth_token: String) -> Self {
            Self {
                url,
                auth_token,
                client: Client::new(),
            }
        }

        pub async fn get_site_data(&mut self) -> Result<Vec<SiteDetails>> {
            let auth_token_header = format!("Bearer {}", &self.auth_token);

            let response = self
                .client
                .get(&self.url)
                .header("AUTHORIZATION", auth_token_header)
                .header("CONTENT_TYPE", "application/json")
                .header("ACCEPT", "application/json")
                .send()
                .await?
                .json::<Vec<SiteDetails>>()
                .await?;

            Ok(response)
        }

        pub async fn get_current_price_data(&mut self) -> Result<Vec<CurrentPrices>> {
            let auth_token_header = format!("Bearer {}", &self.auth_token);

            let response = self
                .client
                .get(&self.url)
                .header("AUTHORIZATION", auth_token_header)
                .header("CONTENT_TYPE", "application/json")
                .header("ACCEPT", "application/json")
                .send()
                .await?
                .json::<Vec<CurrentPrices>>()
                .await?;

            Ok(response)
        }

        pub async fn get_usage_data(&mut self) -> Result<Vec<CurrentUsage>> {
            let auth_token_header = format!("Bearer {}", &self.auth_token);

            let response = self
                .client
                .get(&self.url)
                .header("AUTHORIZATION", auth_token_header)
                .header("CONTENT_TYPE", "application/json")
                .header("ACCEPT", "application/json")
                .send()
                .await?
                .json::<Vec<CurrentUsage>>()
                .await?;

            Ok(response)
        }
    }

    // get config
    let config = AppConfig::get().await?;
    let auth_token = config.apitoken.psk;
    let base_url = config.amberconfig.base_url;

    // get site details
    let sites_url = format!("{}/sites", base_url);
    let mut user_site_details = RestClient::new_client(sites_url, auth_token.clone());
    let user_site_data = user_site_details.get_site_data().await?;

    // one account can only have one site, so extract from array
    let user_site_data = user_site_data
        .get(0)
        .expect("Malformed array/invalid index[0]");

    let site_id = &user_site_data.id;

    // end site details

    // get current price details
    let current_price_url = format!(
        "{}/sites/{}/prices/current?&resolution=30",
        base_url, site_id
    );
    let mut current_price_details = RestClient::new_client(current_price_url, auth_token.clone());
    let current_price_data = current_price_details.get_current_price_data().await?;

    // One site can only have one set of current prices so extract from array
    let current_price_data = current_price_data
        .get(0)
        .expect("Malformed array/invalid index[0]");

    // end current price details

    // get usage dat
    let usage_data_url = format!(
        "{}/sites/{}/usage?startDate=2023-09-12&endDate=2023-09-13&resolution=30'",
        base_url, site_id
    );
    let mut usage_details = RestClient::new_client(usage_data_url, auth_token.clone());
    let usage_data = usage_details.get_usage_data().await?;

    // end usage data

    println!("-------------------------------------------------------------------");
    println!("My site details");
    println!("Grid network: {}", &user_site_data.network);
    println!("My house meter NMI number: {}", &user_site_data.nmi);
    println!("Status: {}", &user_site_data.status);
    println!("-------------------------------------------------------------------");
    println!("Current 30min price window rate");
    println!("Window stats at: {}", &current_price_data.start_time);
    println!("Window ends at: {}", &current_price_data.end_time);
    println!("Per KWH price(c/kWh): {}", &current_price_data.per_kwh);
    println!(
        "Is this window in a spike?: {}",
        &current_price_data.spike_status
    );
    println!("Overall rate status: {}", &current_price_data.descriptor);
    println!("-------------------------------------------------------------------");
    //println!("{:#?}", usage_data);

    Ok(())
}
