use anyhow::{bail, Result};
use iso8601_timestamp::Timestamp;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SiteDetails {
    pub active_from: Timestamp,
    pub channels: Vec<SiteChannels>,
    pub id: String,
    pub network: String,
    pub nmi: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SiteChannels {
    pub identifier: String,
    pub tariff: String,
    // type is a reserved word, so rename it.
    #[serde(rename = "type")]
    pub tariff_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPrices {
    // type is a reserved word, so rename it.
    #[serde(rename = "type")]
    pub interval_type: String,
    pub date: Timestamp,
    pub duration: u8,
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub nem_time: Timestamp,
    pub per_kwh: f32,
    pub renewables: f32,
    pub spot_per_kwh: f32,
    pub channel_type: String,
    pub spike_status: String,
    pub tariff_information: TariffInformation,
    pub descriptor: String,
    pub estimate: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TariffInformation {
    pub period: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUsage {
    #[serde(rename = "type")]
    pub price_type: String,
    pub duration: u8,
    pub date: Timestamp,
    pub end_time: Timestamp,
    pub quality: String,
    pub kwh: f32,
    pub nem_time: Timestamp,
    pub per_kwh: f32,
    pub channel_type: String,
    pub channel_identifier: String,
    pub cost: f32,
    pub renewables: f32,
    pub spot_per_kwh: f32,
    pub start_time: Timestamp,
    pub spike_status: String,
    pub tariff_information: TariffInformation,
    pub descriptor: String,
}

#[derive(Clone)]
pub struct RestClient {
    pub url: String,
    pub auth_token: String,
    pub client: reqwest::Client,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP Request failed: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Serde failed to decode json: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Received a non 200 status code of {status_code:?} with message body: {body:?} ")]
    HttpNon200Status { status_code: String, body: String },
}

impl RestClient {
    pub fn new_client(url: String, auth_token: String) -> Self {
        Self {
            url,
            auth_token,
            client: Client::new(),
        }
    }

    pub async fn get_site_data(&mut self) -> Result<Vec<SiteDetails>, Error> {
        let auth_token_header = format!("Bearer {}", &self.auth_token);

        let response = self
            .client
            .get(&self.url)
            .header("AUTHORIZATION", auth_token_header)
            .header("CONTENT_TYPE", "application/json")
            .header("ACCEPT", "application/json")
            .send()
            .await?;
        match response.status() {
            reqwest::StatusCode::OK => {
                let response = response.json::<Vec<SiteDetails>>().await?;
                return Ok(response);
            }
            //_ => return Err(Error::FuckedOut(response.status().to_string())),
            _ => {
                return Err(Error::HttpNon200Status {
                    status_code: (response.status().to_string()),
                    body: (response.text().await)?,
                })
            }
        }
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
