use rocket::{State, post, get};
use rocket::serde::json::Json;
use rocket::response::status::Created;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use crate::manager::service::ManagerService;
use crate::common::types::{SigningRequest, SigningStatus};
use crate::auth::AuthenticatedUser;
use anyhow::Context;

#[derive(Deserialize)]
pub struct SigningRequestDTO {
    pub message: String,
}

#[derive(Serialize)]
pub struct SigningResponseDTO {
    pub request_id: String,
    pub status: String,
}

#[post("/sign", format = "json", data = "<request>")]
pub async fn sign(manager: &State<ManagerService>, request: Json<SigningRequestDTO>, user: AuthenticatedUser) -> Result<Created<Json<SigningResponseDTO>>, Status> {
    let message = hex::decode(&request.message)
        .context("Failed to decode message")
        .map_err(|_| Status::BadRequest)?;

    let signing_request = SigningRequest {
        id: uuid::Uuid::new_v4().to_string(),
        message,
        threshold: manager.threshold,
        total_parties: manager.total_parties,
    };

    manager.process_signing_request(signing_request.clone()).await
        .context("Failed to process signing request")
        .map_err(|_| Status::InternalServerError)?;

    let response = SigningResponseDTO {
        request_id: signing_request.id,
        status: "Pending".to_string(),
    };

    Ok(Created::new("/").body(Json(response)))
}

#[get("/status/<request_id>")]
pub async fn get_status(manager: &State<ManagerService>, request_id: String, user: AuthenticatedUser) -> Result<Json<SigningResponseDTO>, Status> {
    let status = manager.get_request_status(&request_id).await
        .context("Failed to get request status")
        .map_err(|_| Status::InternalServerError)?;

    let response = SigningResponseDTO {
        request_id,
        status: status.to_string(),
    };

    Ok(Json(response))
}

#[get("/signature/<request_id>")]
pub async fn get_signature(manager: &State<ManagerService>, request_id: String, user: AuthenticatedUser) -> Result<Json<String>, Status> {
    let signature = manager.get_signature(&request_id).await
        .context("Failed to get signature")
        .map_err(|_| Status::InternalServerError)?;

    match signature {
        Some(sig) => Ok(Json(hex::encode(sig))),
        None => Err(Status::NotFound),
    }
}

#[get("/health")]
pub async fn health_check(manager: &State<ManagerService>) -> Status {
    match manager.health_check().await {
        Ok(_) => Status::Ok,
        Err(_) => Status::ServiceUnavailable,
    }
}