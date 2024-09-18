use config::{Config, ConfigError, Environment, File, ConfigBuilder};
use serde::Deserialize;

pub const PARTIES: u16 = 3;
pub const THRESHOLD: u16 = 2;
pub const PATH: &str = "0/1/2";

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub mongodb_uri: String,
    pub rabbitmq_uri: String,
    pub manager_url: String,
    pub manager_port: u16,
    pub signer_port: u16,
    pub signing_timeout: u64,
    pub threshold: usize,
    pub total_parties: usize,
    pub signer_id: u16,
    pub keys_file: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        builder = builder.add_source(File::with_name("config/default"));

        let env = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        builder = builder.add_source(File::with_name(&format!("config/{}", env)).required(false));

        builder = builder.add_source(File::with_name("config/local").required(false));

        builder = builder.add_source(Environment::with_prefix("app"));
        print!("config: {:?}",  builder);
        builder.build()?.try_deserialize()
    }
}