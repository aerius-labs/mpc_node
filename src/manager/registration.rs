use crate::common::types::SignerInfo;
use crate::storage::mongodb::MongoDBStorage;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use rocket::{post, State};
use rocket::serde::json::Json;
use rocket::http::Status;

#[derive(Deserialize)]
pub struct SignerRegistrationRequest {
    pub signer_id: u16,
    pub public_key: String,
}

#[derive(Serialize)]
pub struct SignerRegistrationResponse {
    pub success: bool,
    pub message: String,
}

pub struct RegistrationService {
    storage: MongoDBStorage,
}

impl RegistrationService {
    pub fn new(storage: MongoDBStorage) -> Self {
        Self { storage }
    }

    pub async fn register_signer(&self, signer_info: SignerInfo) -> Result<(), anyhow::Error> {
        self.storage.insert_signer(&signer_info).await
            .context("Failed to register signer")?;
        Ok(())
    }

    pub async fn get_registered_signers(&self) -> Result<Vec<SignerInfo>, anyhow::Error> {
        self.storage.get_signers().await
            .context("Failed to get registered signers")
    }
}

#[post("/register", format = "json", data = "<request>")]
pub async fn register_signer(
    registration_service: &State<RegistrationService>,
    request: Json<SignerRegistrationRequest>,
) -> Result<Json<SignerRegistrationResponse>, Status> {
    let signer_info = SignerInfo {
        signer_id: request.signer_id,
        public_key: request.public_key.clone(),
    };

    match registration_service.register_signer(signer_info).await {
        Ok(_) => Ok(Json(SignerRegistrationResponse {
            success: true,
            message: "Signer registered successfully".to_string(),
        })),
        Err(e) => {
            eprintln!("Failed to register signer: {:?}", e);
            Err(Status::InternalServerError)
        }
    }
}
