use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub mongodb_uri: String,
    pub rabbitmq_uri: String,
    pub manager_url: String,
    pub manager_port: u16,
    pub signing_timeout: u64,
    pub threshold: u16,
    pub total_parties: u16,
    pub path: String,
    pub signer_key_files: Vec<String>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        builder = builder.add_source(File::with_name("config/default"));

        let env = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        builder = builder.add_source(File::with_name(&format!("config/{}", env)).required(false));

        builder = builder.add_source(File::with_name("config/local").required(false));

        builder = builder.add_source(Environment::with_prefix("app"));
        println!("config: {:?}", builder);
        builder.build()?.try_deserialize()
    }
}
