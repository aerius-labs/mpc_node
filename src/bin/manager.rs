use rocket::routes;
use rocket::{figment::Figment, Config};
use std::sync::Arc;
use tss_network::config::Settings;
use tss_network::manager::api::{
    generate_keys, generate_test_token, get_key_gen_result, get_signing_result, sign,
};
use tss_network::manager::handlers::{get, set, signup_keygen, signup_sign, update_signing_result};
use tss_network::manager::service::ManagerService;

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
            settings.threshold,
            settings.total_parties,
        )
        .await?,
    );

    let manager_service_for_rocket = manager_service.clone();

    let ip = settings
        .manager_url
        .split("://")
        .nth(1)
        .unwrap_or(&settings.manager_url);
    let figment = Figment::from(Config::default())
        .merge(("address", ip))
        .merge(("port", settings.manager_port));
    let config: rocket::Config = figment.extract().expect("Failed to extract Rocket config");

    let rocket_future = rocket::custom(config)
        .manage(manager_service_for_rocket)
        .manage(Arc::new(settings))
        .mount(
            "/",
            routes![
                sign,
                signup_sign,
                set,
                get,
                get_signing_result,
                update_signing_result,
                generate_keys,
                signup_keygen,
                get_key_gen_result,
                generate_test_token
            ],
        )
        .launch();

    tokio::select! {
        _ = rocket_future => println!("Rocket server shut down"),
    }
    Ok(())
}
