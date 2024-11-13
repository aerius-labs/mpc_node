use std::sync::Arc;
use tokio::task;
use tss_network::config::Settings;
use tss_network::signer::service::SignerService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let settings = Settings::new().expect("Failed to load configuration");

    let num_signers = settings.total_parties as usize;
    let total_keys = settings.signer_key_files.len();
    if num_signers != total_keys {
        panic!("Number of signers does not match the number of keys provided");
    }
    let mut signer_tasks = Vec::with_capacity(num_signers);

    for (i, key_file) in settings.signer_key_files.iter().enumerate() {
        let signer_service: Arc<SignerService> = Arc::new(
            SignerService::new(
                &settings.manager_url,
                &settings.manager_port,
                &settings.rabbitmq_uri,
                key_file,
                &settings.threshold,
                &settings.total_parties,
                &settings.path,
            )
            .await?,
        );

        let signer_task = task::spawn(async move {
            if let Err(e) = signer_service.run().await {
                eprintln!("SignerService {} error: {:?}", i + 1, e);
            }
        });
        signer_tasks.push(signer_task);
    }

    futures::future::join_all(signer_tasks).await;
    Ok(())
}
