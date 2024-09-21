// use tss_network::manager::service::ManagerService;
// use tss_network::common::types::{SigningRequest, SigningResult, SigningStatus};
// use tss_network::tss_error::TssResult;
// use mockall::predicate::*;
// use mockall::mock;
// use std::collections::HashMap;
//
// mock! {
//     pub MongoDBStorage {}
//     impl MongoDBStorage {
//         pub async fn insert_request(&self, request: &SigningRequest) -> TssResult<()>;
//         pub async fn update_request(&self, result: &SigningResult) -> TssResult<()>;
//         pub async fn get_partial_signatures(&self, request_id: &str) -> TssResult<HashMap<u16, Vec<u8>>>;
//         pub async fn get_request(&self, request_id: &str) -> TssResult<SigningRequest>;
//     }
// }
//
// mock! {
//     pub RabbitMQService {}
//     impl RabbitMQService {
//         pub async fn publish_request(&self, request: &SigningRequest) -> TssResult<()>;
//         pub async fn publish_result(&self, result: &SigningResult) -> TssResult<()>;
//     }
// }
//
// mock! {
//     pub RegistrationService {}
//     impl RegistrationService {
//         pub async fn register_signer(&self, signer_info: SignerInfo) -> Result<(), anyhow::Error>;
//         pub async fn get_registered_signers(&self) -> Result<Vec<SignerInfo>, anyhow::Error>;
//     }
// }
//
// #[tokio::test]
// async fn test_process_signing_request() {
//     let mut mock_storage = MockMongoDBStorage::new();
//     let mut mock_queue = MockRabbitMQService::new();
//     let mut mock_registration_service = MockRegistrationService::new();
//
//     mock_storage
//         .expect_insert_request()
//         .with(eq(SigningRequest {
//             id: "test_id".to_string(),
//             message: vec![1, 2, 3],
//             threshold: 2,
//             total_parties: 3,
//         }))
//         .returning(|_| Ok(()));
//
//     mock_queue
//         .expect_publish_request()
//         .with(eq(SigningRequest {
//             id: "test_id".to_string(),
//             message: vec![1, 2, 3],
//             threshold: 2,
//             total_parties: 3,
//         }))
//         .returning(|_| Ok(()));
//
//     let manager_service = ManagerService {
//         storage: mock_storage,
//         queue: mock_queue,
//         registration_service: mock_registration_service,
//         signing_timeout: std::time::Duration::from_secs(60),
//         threshold: 2,
//         total_parties: 3,
//     };
//
//     let request = SigningRequest {
//         id: "test_id".to_string(),
//         message: vec![1, 2, 3],
//         threshold: 2,
//         total_parties: 3,
//     };
//
//     let result = manager_service.process_signing_request(request).await;
//     assert!(result.is_ok());
// }
//
// #[tokio::test]
// async fn test_handle_signing_result() {
//     let mut mock_storage = MockMongoDBStorage::new();
//     let mock_queue = MockRabbitMQService::new();
//     let mock_registration_service = MockRegistrationService::new();
//
//     mock_storage
//         .expect_update_request()
//         .with(eq(SigningResult {
//             request_id: "test_id".to_string(),
//             signature: Some(vec![4, 5, 6]),
//             status: SigningStatus::Completed,
//         }))
//         .returning(|_| Ok(()));
//
//     mock_storage
//         .expect_get_partial_signatures()
//         .with(eq("test_id"))
//         .returning(|_| {
//             let mut signatures = HashMap::new();
//             signatures.insert(1, vec![1, 2, 3]);
//             signatures.insert(2, vec![4, 5, 6]);
//             Ok(signatures)
//         });
//
//     mock_storage
//         .expect_get_request()
//         .with(eq("test_id"))
//         .returning(|_| {
//             Ok(SigningRequest {
//                 id: "test_id".to_string(),
//                 message: vec![1, 2, 3],
//                 threshold: 2,
//                 total_parties: 3,
//             })
//         });
//
//     let manager_service = ManagerService {
//         storage: mock_storage,
//         queue: mock_queue,
//         registration_service: mock_registration_service,
//         signing_timeout: std::time::Duration::from_secs(60),
//         threshold: 2,
//         total_parties: 3,
//     };
//
//     let result = SigningResult {
//         request_id: "test_id".to_string(),
//         signature: Some(vec![4, 5, 6]),
//         status: SigningStatus::Completed,
//     };
//
//     let handle_result = manager_service.handle_signing_result(result).await;
//     assert!(handle_result.is_ok());
// }
//
// #[tokio::test]
// async fn test_reconstruct_signature() {
//     let mock_storage = MockMongoDBStorage::new();
//     let mock_queue = MockRabbitMQService::new();
//     let mock_registration_service = MockRegistrationService::new();
//
//     let manager_service = ManagerService {
//         storage: mock_storage,
//         queue: mock_queue,
//         registration_service: mock_registration_service,
//         signing_timeout: std::time::Duration::from_secs(60),
//         threshold: 2,
//         total_parties: 3,
//     };
//
//     let mut partial_signatures = HashMap::new();
//     partial_signatures.insert(1, vec![1, 2, 3]);
//     partial_signatures.insert(2, vec![4, 5, 6]);
//
//     let result = manager_service.reconstruct_signature("test_id", partial_signatures).await;
//     assert!(result.is_ok());
//     // Note: This test might need to be adjusted based on the actual implementation of reconstruct_signature
// }

use reqwest::Client;
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tss_network::common::{MessageStatus, MessageToSignStored};
use tss_network::manager::api::SigningResponseDTO;

#[tokio::test]
async fn test_signing_flow() {
    // Start the manager process
    let mut manager = Command::new("cargo")
        .args(["run", "--bin", "manager"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start manager");

    // Start the signer process
    let mut signer = Command::new("cargo")
        .args(["run", "--bin", "signer"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start signer");

    // Give some time for the processes to start up
    sleep(Duration::from_secs(5)).await;

    let client = Client::new();
    let body = json!({
        "message": "test_message"
    });

    let response = client
        .post("http://127.0.0.1:8000/sign")
        .json(&body)
        .send()
        .await
        .expect("Failed to send request");

    // Check if the request was successful
    assert!(
        response.status().is_success(),
        "Request failed with status: {}",
        response.status()
    );

    let response_body: Value = response
        .json()
        .await
        .expect("Failed to parse response body");
    let signing_res_dto: SigningResponseDTO =
        serde_json::from_value(response_body).expect("Failed to deserialize SigningResponseDTO");

    // Add assertions to check the response
    assert!(
        !signing_res_dto.request_id.is_empty(),
        "request_id should not be empty"
    );
    assert_eq!(
        signing_res_dto.status, "Pending",
        "status should be 'pending'"
    );

    let timeout_duration = Duration::from_secs(60); // Adjust as needed
    let result = timeout(
        timeout_duration,
        poll_signing_result(&client, &signing_res_dto.request_id),
    )
    .await
    .unwrap();
    println!("Sigature: {:?}", result.signature);

    // For demonstration, we'll just check if the processes are still running
    assert!(
        manager.try_wait().expect("manager wait failed").is_none(),
        "Manager process exited prematurely"
    );
    assert!(
        signer.try_wait().expect("signer wait failed").is_none(),
        "Signer process exited prematurely"
    );
    // Clean up: kill the processes
    manager.kill().expect("Failed to kill manager");
    signer.kill().expect("Failed to kill signer");

    println!("Test completed successfully");
}

// Function to poll for signing result
async fn poll_signing_result(client: &Client, request_id: &str) -> MessageToSignStored {
    let url = format!("http://127.0.0.1:8000/signing_result/{}", request_id);
    loop {
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Failed to send request");
        let result: Option<MessageToSignStored> =
            response.json().await.expect("Failed to parse response");

        if let Some(stored_message) = result {
            match stored_message.status {
                MessageStatus::Completed => return stored_message,
                MessageStatus::Pending => {
                    println!("Status still pending, waiting...");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        } else {
            panic!("Unexpected empty response");
        }
    }
}

// async fn test_signing_flow() {
//     // Load test configuration
//     let settings = Settings::new().expect("Failed to load configuration");

//     // Initialize ManagerService
//     let manager = ManagerService::new(
//         &settings.mongodb_uri,
//         &settings.rabbitmq_uri,
//         settings.signing_timeout,
//         settings.threshold,
//         settings.total_parties,
//     )
//     .await
//     .expect("Failed to initialize ManagerService");

//     // Initialize SignerServices
//     let mut signers = Vec::new();
//     for i in 1..=settings.total_parties {
//         let signer = SignerService::new(
//             &settings.manager_url,
//             &settings.rabbitmq_uri,
//             &format!("test_keys_{}.json", i),
//         )
//         .await
//         .expect("Failed to initialize SignerService");
//         signers.push(signer);
//     }

//     // Create a signing request
//     let request = SigningRequest {
//         id: "test_request".to_string(),
//         message: vec![1, 2, 3, 4, 5],
//     };

//     // Publish the signing request
//     manager
//         .handle_signing_request(request.clone())
//         .await
//         .expect("Failed to handle signing request");

//     // Simulate signers processing the request
//     for signer in &signers {
//         signer
//             .handle_signing_request(request.clone())
//             .await
//             .expect("Signer failed to handle signing request");
//     }

//     // Check the signing result
//     let result = manager
//         .get_signing_result(&request.id)
//         .await
//         .expect("Failed to get signing result");

//     assert!(result.is_some(), "Signing result should exist");
//     let result = result.unwrap();
//     assert_eq!(result.request_id, request.id);
//     assert!(result.signature.is_some(), "Signature should be present");
//     assert_eq!(result.status, MessageStatus::Completed);
// }
