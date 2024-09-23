use rocket::routes;
use std::sync::Arc;
use tokio::task;
use tss_network::config::Settings;
use tss_network::manager::api::{get_signing_result, sign};
use tss_network::manager::handlers::{get, set, signup_sign, update_signing_result};
use tss_network::manager::service::ManagerService;
use rocket::{Config, figment::Figment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let settings = Settings::new().expect("Failed to load configuration");

    // Initialize ManagerService
    let manager_service: Arc<ManagerService> = Arc::new(
        ManagerService::new(
            &settings.mongodb_uri,
            &settings.rabbitmq_uri,
            settings.signing_timeout,
            settings.threshold,
            settings.total_parties,
        )
        .await?,
    );

    // Run the manager service
    // manager_service.run().await?;

    let manager_service_for_rocket = manager_service.clone();

    // Start the ManagerService in a separate task
    let manager_task = task::spawn(async move {
        if let Err(e) = manager_service.run().await {
            eprintln!("ManagerService error: {:?}", e);
        }
    });

    let ip = settings.manager_url.split("://").nth(1).unwrap_or(&settings.manager_url);
    let figment = Figment::from(Config::default())
    .merge(("address", ip))
    .merge(("port", settings.manager_port));
    let config: rocket::Config = figment.extract().expect("Failed to extract Rocket config");

    let rocket_future = rocket::custom(config)
        .manage(manager_service_for_rocket)
        .mount(
            "/",
            routes![
                sign,
                signup_sign,
                set,
                get,
                get_signing_result,
                update_signing_result
            ],
        )
        .launch();

    tokio::select! {
        _ = manager_task => println!("ManagerService task completed"),
        _ = rocket_future => println!("Rocket server shut down"),
    }
    Ok(())
}
