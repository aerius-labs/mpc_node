use std::sync::Arc;

use crate::common::types::SigningRequest;
use crate::common::{KeyGenParams, KeyGenRequest, KeysToStore, MessageToSignStored};
use crate::error::TssError;
use crate::manager::service::ManagerService;
use anyhow::Context;
use rocket::http::hyper::request;
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{get, post, response, State};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SigningRequestDTO {
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct SigningResponseDTO {
    pub request_id: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct KeyGenRequestDTO {
    pub manager_url: String,
    pub threshold: u16,
    pub total_parties: u16,
}

#[derive(Serialize, Deserialize)]
pub struct KeyGenResponseDTO {
    pub request_id: String,
    pub keys: Vec<String>,
}

#[post("/sign", format = "json", data = "<request>")]
pub async fn sign(
    manager: &State<Arc<ManagerService>>,
    request: Json<SigningRequestDTO>,
) -> Result<Created<Json<SigningResponseDTO>>, Status> {
    let message: Vec<u8> = request.message.as_bytes().to_vec();

    let signing_request = SigningRequest {
        id: uuid::Uuid::new_v4().to_string(),
        message,
    };

    manager
        .process_signing_request(signing_request.clone())
        .await
        .context("Failed to process signing request")
        .map_err(|_| Status::InternalServerError)?;

    let response = SigningResponseDTO {
        request_id: signing_request.id,
        status: "Pending".to_string(),
    };

    Ok(Created::new("/").body(Json(response)))
}

#[get("/signing_result/<request_id>")]
pub async fn get_signing_result(
    manager: &State<Arc<ManagerService>>,
    request_id: String,
) -> Result<Json<Option<MessageToSignStored>>, TssError> {
    let result = manager.get_signing_result(&request_id).await?;
    Ok(Json(result))
}

#[post("/key_gen_request", format = "json", data = "<request>")]
pub async fn generate_keys(
    manager: &State<Arc<ManagerService>>,
    request: Json<KeyGenRequestDTO>,
) -> Result<Created<Json<KeyGenResponseDTO>>, Status> {
    let threshold = request.threshold;
    let total_parties = request.total_parties;
    let manger_addr = request.manager_url.clone();

    if threshold > total_parties {
        return Err(Status::BadRequest);
    }

    let keygen_request = KeyGenRequest {
        id: uuid::Uuid::new_v4().to_string(),
        keygen_params: KeyGenParams {
            parties: total_parties,
            threshold,
        },
    };

    let result = manager
        .process_keygen_request(keygen_request.clone(), &manger_addr)
        .await
        .context("Failed to process key generation request")
        .map_err(|_| Status::InternalServerError)?;

    let response = KeyGenResponseDTO {
        request_id: keygen_request.id,
        keys: result,
    };

    Ok(Created::new("/").body(Json(response)))
}

#[get("/key_gen_result/<request_id>")]
pub async fn get_key_gen_result(
    manager: &State<Arc<ManagerService>>,
    request_id: String,
) -> Result<Json<Option<KeysToStore>>, TssError> {
    let result = manager.get_key_gen_result(&request_id).await?;
    Ok(Json(result))
}
