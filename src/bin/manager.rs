use rocket::{route, routes, Build, Rocket, Route};
use tss_network::manager::service::ManagerService;
use tss_network::config::Settings;
use tokio::task;
use std::sync::Arc;
use tss_network::manager::api::sign;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let settings = Settings::new().expect("Failed to load configuration");

    // Initialize ManagerService
    let manager_service: Arc<ManagerService> = Arc::new(ManagerService::new(
        &settings.mongodb_uri,
        &settings.rabbitmq_uri,
        settings.signing_timeout,
        settings.threshold,
        settings.total_parties,
    ).await?);

    // Run the manager service
    // manager_service.run().await?;

    let manager_service_for_rocket = manager_service.clone();

    // Start the ManagerService in a separate task
    let manager_task = task::spawn(async move {
        if let Err(e) = manager_service.run().await {
            eprintln!("ManagerService error: {:?}", e);
        }
    });

    // Configure and launch the Rocket server
    let rocket_future = rocket::build()
        .manage(manager_service_for_rocket)
        .mount("/", routes![sign])
        .launch();


        tokio::select! {
            _ = manager_task => println!("ManagerService task completed"),
            _ = rocket_future => println!("Rocket server shut down"),
        }
    Ok(())
}
