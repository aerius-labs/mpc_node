use std::sync::Arc;
use tokio::task;
use tss_network::config::Settings;
use tss_network::signer::service::SignerService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let settings = Settings::new().expect("Failed to load configuration");

    let signer1_service: Arc<SignerService> = Arc::new(
        SignerService::new(
            &settings.manager_url,
            &settings.manager_port,
            &settings.rabbitmq_uri,
            &settings.signer1_key_file,
            &settings.threshold,
            &settings.total_parties,
            &settings.path
        )
        .await?,
    );

    let signer1_task = task::spawn(async move {
        if let Err(e) = signer1_service.run().await {
            eprintln!("SignerService error: {:?}", e);
        }
    });

    let signer2_service: Arc<SignerService> = Arc::new(
        SignerService::new(
            &settings.manager_url,
            &settings.manager_port,
            &settings.rabbitmq_uri,
            &settings.signer2_key_file,
            &settings.threshold,
            &settings.total_parties,
            &settings.path
        )
        .await?,
    );

    let signer2_task = task::spawn(async move {
        if let Err(e) = signer2_service.run().await {
            eprintln!("SignerService error: {:?}", e);
        }
    });

    let signer3_service: Arc<SignerService> = Arc::new(
        SignerService::new(
            &settings.manager_url,
            &settings.manager_port,
            &settings.rabbitmq_uri,
            &settings.signer3_key_file,
            &settings.threshold,
            &settings.total_parties,
            &settings.path
        )
        .await?,
    );

    let signer3_task = task::spawn(async move {
        if let Err(e) = signer3_service.run().await {
            eprintln!("SignerService error: {:?}", e);
        }
    });

    let _ = tokio::join!(signer1_task, signer2_task, signer3_task);

    Ok(())
}
