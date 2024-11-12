use std::sync::Arc;

use crate::auth::{AuthenticatedUser, Role};
use crate::common::types::SigningRequest;
use crate::common::{KeyGenParams, KeyGenRequest, KeysToStore, MessageToSignStored};
use crate::error::TssError;
use crate::manager::service::ManagerService;
use anyhow::Context;
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{get, post, State};
use serde::{Deserialize, Serialize};
use crate::{auth::create_token, config::Settings};

use super::constants::MAX_MESSAGE_SIZE;

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

#[derive(Serialize)]
pub struct TokenResponse {
    token: String,
    expires_in: u64,
    token_type: String,
}



#[post("/sign", format = "json", data = "<request>")]
pub async fn sign(
    auth: AuthenticatedUser,
    manager: &State<Arc<ManagerService>>,
    request: Json<SigningRequestDTO>,
) -> Result<Created<Json<SigningResponseDTO>>, Status> {
    // Verify that we have a public role
    if auth.role != Role::Public {
        return Err(Status::Forbidden);
    }

    // validate messgage size
    if request.message.len() > MAX_MESSAGE_SIZE {
        return Err(Status::PayloadTooLarge);
    }

    let message: Vec<u8> = request.message.as_bytes().to_vec();
    let signing_request = SigningRequest {
        id: uuid::Uuid::new_v4().to_string(),
        message,
    };

    match manager.process_signing_request(signing_request.clone()).await {
        Ok(_) => {
            let response = SigningResponseDTO {
                request_id: signing_request.id,
                status: "Pending".to_string(),
            };
            Ok(Created::new("/").body(Json(response)))
        }
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/signing_result/<request_id>")]
pub async fn get_signing_result(
    auth: AuthenticatedUser,
    manager: &State<Arc<ManagerService>>,
    request_id: String,
) -> Result<Json<Option<MessageToSignStored>>, Status> {
    // Verify that we have a public role
    if auth.role != Role::Public {
        return Err(Status::Forbidden);
    }

    // Validate UUID
    if uuid::Uuid::parse_str(&request_id).is_err() {
        return Err(Status::BadRequest);
    }

    match manager.get_signing_result(&request_id).await {
        Ok(result) => Ok(Json(result)),
        Err(_) => Err(Status::InternalServerError),
    }
}

// For testing and development purposes
// Only compile these endpoints in debug/development mode
#[cfg(debug_assertions)]
#[get("/generate_test_token/<role>")]
pub async fn generate_test_token(
    role: String,
    settings: &rocket::State<Arc<Settings>>
) -> Result<Json<TokenResponse>, rocket::http::Status> {
    let role = match role.to_lowercase().as_str() {
        "public" => Role::Public,
        "signer" => Role::Signer,
        "admin" => Role::Admin,
        _ => return Err(rocket::http::Status::BadRequest),
    };
    
    match create_token("test-user", role, settings) {
        Ok(token) => Ok(Json(TokenResponse {
            token,
            expires_in: settings.security.jwt_expiration,
            token_type: "Bearer".to_string(),
        })),
        Err(_) => Err(rocket::http::Status::InternalServerError),
    }
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
