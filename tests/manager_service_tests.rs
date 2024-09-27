use reqwest::Client;
use rocket::http::hyper::body;
use rocket::response;
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tss_network::common::{KeysToStore, MessageStatus, MessageToSignStored};
use tss_network::manager::api::{KeyGenResponseDTO, SigningResponseDTO};

fn build_project() {
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .expect("Failed to build project");

    assert!(status.success(), "Build failed");
}

#[tokio::test]
async fn test_signing_flow() {
    // Build the project
    build_project();

    // Start the manager process
    let mut manager = Command::new("./target/release/manager")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start manager");

    // Start the signer process
    let mut signer = Command::new("./target/release/signer")
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
        .post("http://127.0.0.1:8080/sign")
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
    println!("Signature: {:?}", result.signature);

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

#[tokio::test]
async fn test_keygen_flow() {
    // Build the project
    build_project();

    // Start the manager process
    let mut manager = Command::new("./target/release/manager")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start manager");

    // Give some time for the processes to start up
    sleep(Duration::from_secs(5)).await;

    let client = Client::new();

    let body = json!({
        "threshold": 2,
        "total_parties": 3,
        "manager_url": "http://127.0.0.1:8080",
    });

    let response = client
        .post("http://127.0.0.1:8080/key_gen_request")
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
    let keygen_res_dto: KeyGenResponseDTO =
        serde_json::from_value(response_body).expect("Failed to deserialize KeyGenResponseDTO");

    assert!(
        !keygen_res_dto.request_id.is_empty(),
        "request_id should not be empty"
    );
    assert_eq!(
        keygen_res_dto.keys.len(),
        3,
        "keys should contain 3 elements"
    );

    let stored_response = client
        .get(&format!(
            "http://127.0.0.1:8080/key_gen_result/{}",
            keygen_res_dto.request_id
        ))
        .send()
        .await
        .expect("Failed to send request");

    let result: Option<KeysToStore> = stored_response
        .json()
        .await
        .expect("Failed to parse response");

    assert!(result.clone().is_some(), "Result should not be None");
    assert!(
        result.clone().unwrap().keys.unwrap().len() == 3,
        "Keys should contain 3 elements"
    );
    let keys = result.unwrap().keys.unwrap();
    for key in keys {
        assert!(!key.is_empty(), "Key should not be empty");
        println!("Key: {}\n", key);
    }

    // Clean up: kill the processes
    manager.kill().expect("Failed to kill manager");
    println!("key gen Test completed successfully");
}
// Function to poll for signing result
async fn poll_signing_result(client: &Client, request_id: &str) -> MessageToSignStored {
    let url = format!("http://127.0.0.1:8080/signing_result/{}", request_id);
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
                MessageStatus::InProgress => {
                    println!("Status in progress, waiting...");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        } else {
            panic!("Unexpected empty response");
        }
    }
}
