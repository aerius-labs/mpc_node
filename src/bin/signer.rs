use clap::Parser;
use std::{path::PathBuf, sync::Arc};
use tss_network::config::Settings;
use tss_network::signer::service::SignerService;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the key file
    #[arg(short, long)]
    key_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();
    let settings = Settings::new().expect("Failed to load configuration");

    // Get and validate key file
    let key_file = get_key_file(&args, &settings).map_err(|e| e)?;

    let signer_service: Arc<SignerService> = Arc::new(
        SignerService::new(
            &settings.manager_url,
            &settings.manager_port,
            &settings.rabbitmq_uri,
            &key_file,
            &settings.threshold,
            &settings.total_parties,
            &settings.path,
        )
        .await?,
    );

    signer_service.run().await?;
    Ok(())
}

fn get_key_file(args: &Args, settings: &Settings) -> Result<String, String> {
    // First try CLI argument
    if let Some(key_path) = &args.key_file {
        // Validate the CLI provided path
        if !key_path.exists() {
            return Err(format!(
                "Key file from CLI argument not found: {}",
                key_path.display()
            ));
        }
        return Ok(key_path
            .to_str()
            .ok_or("Invalid path encoding")?
            .to_string());
    }

    // Then try config file
    if !settings.signer_key_file.is_empty() {
        let config_path = PathBuf::from(&settings.signer_key_file);
        if !config_path.exists() {
            return Err(format!(
                "Key file from config not found: {}",
                settings.signer_key_file
            ));
        }
        return Ok(settings.signer_key_file.clone());
    }

    // If neither is provided, return error
    Err(
        "No key file provided. Please specify either in config file or via --key-file argument"
            .to_string(),
    )
}
