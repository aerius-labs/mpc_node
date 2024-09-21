use std::sync::Arc;

use crate::auth::AuthenticatedUser;
use crate::common::types::{SigningRequest, SigningStatus};
use crate::common::MessageToSignStored;
use crate::error::TssError;
use crate::manager::service::ManagerService;
use anyhow::Context;
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{get, post, State};
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
// #[get("/status/<request_id>")]
// pub async fn get_status(manager: &State<ManagerService>, request_id: String, user: AuthenticatedUser) -> Result<Json<SigningResponseDTO>, Status> {
//     let status = manager.get_request_status(&request_id).await
//         .context("Failed to get request status")
//         .map_err(|_| Status::InternalServerError)?;

//     let response = SigningResponseDTO {
//         request_id,
//         status: status.to_string(),
//     };

//     Ok(Json(response))
// }

// #[get("/signature/<request_id>")]
// pub async fn get_signature(manager: &State<ManagerService>, request_id: String, user: AuthenticatedUser) -> Result<Json<String>, Status> {
//     let signature = manager.get_signature(&request_id).await
//         .context("Failed to get signature")
//         .map_err(|_| Status::InternalServerError)?;

//     match signature {
//         Some(sig) => Ok(Json(hex::encode(sig))),
//         None => Err(Status::NotFound),
//     }
// }

// #[get("/health")]
// pub async fn health_check(manager: &State<ManagerService>) -> Status {
//     match manager.health_check().await {
//         Ok(_) => Status::Ok,
//         Err(_) => Status::ServiceUnavailable,
//     }
// }
