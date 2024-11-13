use std::net::IpAddr;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub jwt_expiration: u64,
    pub allowed_signer_ips: Vec<String>,
}
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
    // New secuirty configuration section
    pub security: SecurityConfig,
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

    // Helper method to validate IP against whitelist
    pub fn is_ip_whitelisted(&self, ip: IpAddr) -> bool {
        self.security
            .allowed_signer_ips
            .iter()
            .any(|allowed_ip| allowed_ip.parse::<IpAddr>().ok() == Some(ip))
    }
}
