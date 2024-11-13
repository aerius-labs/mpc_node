use crate::common::types::SigningRequest;
use crate::common::{KeyGenRequest, KeysToStore, MessageStatus, MessageToSignStored, SignerResult};
use crate::error::TssError;
use crate::manager::constants::MAX_MESSAGE_SIZE;
use anyhow::Result;
use mongodb::bson::{doc, to_document};
use mongodb::{Client, Collection};

pub struct MongoDBStorage {
    requests: Collection<MessageToSignStored>,
    keys_gen_requests: Collection<KeysToStore>,
}

impl MongoDBStorage {
    pub async fn new(uri: &str, db_name: &str) -> Result<Self> {
        let client = Client::with_uri_str(uri).await?;
        let db = client.database(db_name);

        Ok(Self {
            requests: db.collection::<MessageToSignStored>("messages_to_sign"),
            keys_gen_requests: db.collection::<KeysToStore>("keys_gen_requests"),
        })
    }

    pub async fn insert_request(&self, request: &SigningRequest) -> Result<()> {
        if request.message.len() > MAX_MESSAGE_SIZE {
            return Err(TssError::MessageTooLarge.into());
        }

        let message_to_sign = MessageToSignStored {
            request_id: request.id.clone(),
            message: request.message.clone(),
            status: MessageStatus::Pending,
            signature: None,
        };
        self.requests.insert_one(message_to_sign, None).await?;
        Ok(())
    }

    pub async fn insert_key_gen_request(&self, request: &KeyGenRequest) -> Result<()> {
        let keys_to_store = KeysToStore {
            request_id: request.id.clone(),
            status: MessageStatus::Pending,
            key_gen_params: request.keygen_params.clone(),
            keys: None,
        };
        self.keys_gen_requests
            .insert_one(keys_to_store, None)
            .await?;
        Ok(())
    }

    pub async fn update_key_gen_result(&self, request_id: &str, keys: Vec<String>) -> Result<()> {
        let filter = doc! { "request_id": request_id };
        if let Some(mut stored_keys) = self
            .keys_gen_requests
            .find_one(filter.clone(), None)
            .await?
        {
            if stored_keys.status == MessageStatus::Pending {
                stored_keys.keys = Some(keys);
                stored_keys.status = MessageStatus::Completed;
                let update_doc = to_document(&stored_keys)?;
                self.keys_gen_requests
                    .update_one(filter, doc! { "$set": update_doc }, None)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_key_gen_result(&self, request_id: &str) -> Result<Option<KeysToStore>> {
        // Validate UUID
        if uuid::Uuid::parse_str(&request_id).is_err() {
            return Err(TssError::InvalidUuid(request_id.to_string()).into());
        }
        let filter = doc! { "request_id": request_id };
        if let Some(doc) = self.keys_gen_requests.find_one(filter, None).await? {
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }

    pub async fn get_signing_result(&self, id: &str) -> Result<Option<MessageToSignStored>> {
        // Validate UUID
        if uuid::Uuid::parse_str(&id).is_err() {
            return Err(TssError::InvalidUuid(id.to_string()).into());
        }
        let filter = doc! { "request_id": id };
        if let Some(doc) = self.requests.find_one(filter, None).await? {
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }

    pub async fn update_signing_result(&self, result: &SignerResult) -> Result<()> {
        // Validate UUID
        if uuid::Uuid::parse_str(&result.request_id).is_err() {
            return Err(TssError::InvalidUuid(result.request_id.clone()).into());
        }
        let filter = doc! { "request_id": &result.request_id };
        // Find the current document in the collection
        if let Some(mut stored_message) = self.requests.find_one(filter.clone(), None).await? {
            // Check if the current status is `Pending`
            if stored_message.status == MessageStatus::Pending {
                // Update the signature and status to `Completed`
                stored_message.signature = Some(result.signature.clone());
                stored_message.status = MessageStatus::Completed;

                // Perform the update in the collection
                let update_doc = to_document(&stored_message)?;
                self.requests
                    .update_one(filter, doc! { "$set": update_doc }, None)
                    .await?;
            }
        }
        // If the status is not pending, simply return Ok()
        Ok(())
    }
}
