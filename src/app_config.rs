use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AmberConfig {
    pub base_url: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct ApiToken {
    pub name: String,
    pub psk: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AppConfig {
    pub amberconfig: AmberConfig,
    pub apitoken: ApiToken,
}

impl AppConfig {
    pub async fn get() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("config.toml"))
            .build()?;

        config.try_deserialize()
    }
}
