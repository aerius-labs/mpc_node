use tss_network::manager::service::ManagerService;
use tss_network::config::Settings;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let settings = Settings::new().expect("Failed to load configuration");

    // Initialize ManagerService
    let manager_service = Arc::new(ManagerService::new(
        &settings.mongodb_uri,
        &settings.rabbitmq_uri,
        settings.signing_timeout,
        settings.threshold,
        settings.total_parties,
    ).await?);

    // Run the manager service
    manager_service.run().await?;

    Ok(())
}
