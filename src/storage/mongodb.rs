use crate::common::types::SigningRequest;
use crate::common::{MessageStatus, MessageToSignStored, SignerResult};
use anyhow::Result;
use mongodb::bson::{doc, to_document};
use mongodb::{Client, Collection};

pub struct MongoDBStorage {
    requests: Collection<MessageToSignStored>,
}

impl MongoDBStorage {
    pub async fn new(uri: &str, db_name: &str) -> Result<Self> {
        let client = Client::with_uri_str(uri).await?;
        let db = client.database(db_name);

        Ok(Self {
            requests: db.collection::<MessageToSignStored>("messages_to_sign"),
        })
    }

    pub async fn insert_request(&self, request: &SigningRequest) -> Result<()> {
        let message_to_sign = MessageToSignStored {
            request_id: request.id.clone(),
            message: request.message.clone(),
            status: MessageStatus::Pending,
            signature: None,
        };
        self.requests.insert_one(message_to_sign, None).await?;
        Ok(())
    }

    pub async fn get_signing_result(&self, id: &str) -> Result<Option<MessageToSignStored>> {
        let filter = doc! { "request_id": id };
        if let Some(doc) = self.requests.find_one(filter, None).await? {
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }

    pub async fn update_signing_result(&self, result: &SignerResult) -> Result<()> {
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
