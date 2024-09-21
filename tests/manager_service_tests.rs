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
            }
        } else {
            panic!("Unexpected empty response");
        }
    }
}
