use mongodb::{Client, Database, Collection};
use mongodb::bson::{doc, Document};
use crate::common::types::{SigningRequest, SigningResult, SigningStatus};
use crate::error::TssError;
use anyhow::Result;
use std::convert::TryFrom;
use mongodb::bson;

pub struct MongoDBStorage {
    requests: Collection<Document>,
    results: Collection<Document>,
}

impl MongoDBStorage {
    pub async fn new(uri: &str, db_name: &str) -> Result<Self> {
        let client = Client::with_uri_str(uri).await?;
        let db = client.database(db_name);

        Ok(Self {
            requests: db.collection("signing_requests"),
            results: db.collection("signing_results"),
        })
    }

    pub async fn insert_request(&self, request: &SigningRequest) -> Result<()> {
        let doc = doc! {
            "id": &request.id,
            "message": bson::Binary { subtype: bson::spec::BinarySubtype::Generic, bytes: request.message.clone() },
        };
        self.requests.insert_one(doc, None).await?;
        Ok(())
    }

    pub async fn get_request(&self, id: &str) -> Result<Option<SigningRequest>> {
        let filter = doc! { "id": id };
        if let Some(doc) = self.requests.find_one(filter, None).await? {
            let id = doc.get_str("id")?.to_string();
            let message = doc.get_binary_generic("message")?.clone();
            Ok(Some(SigningRequest { id, message }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_signing_result(&self, result: &SigningResult) -> Result<()> {
        let filter = doc! { "request_id": &result.request_id };
        let update = doc! {
            "$set": {
                "signature": result.signature.as_ref().map(|s| bson::Binary { subtype: bson::spec::BinarySubtype::Generic, bytes: s.clone() }),
                "status": result.status.to_string(),
            }
        };
        self.results.update_one(filter, update, None).await?;
        Ok(())
    }

    pub async fn get_signing_result(&self, request_id: &str) -> Result<Option<SigningResult>> {
        let filter = doc! { "request_id": request_id };
        if let Some(doc) = self.results.find_one(filter, None).await? {
            let request_id = doc.get_str("request_id")?.to_string();
            let signature = doc.get_binary_generic("signature").ok().map(|b| b.clone());
            let status = SigningStatus::try_from(doc.get_str("status")?.to_string())?;
            Ok(Some(SigningResult { request_id, signature, status }))
        } else {
            Ok(None)
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let filter = doc! { "key": key };
        if let Some(doc) = self.requests.find_one(filter, None).await? {
            Ok(doc.get_str("value").ok().map(|s| s.to_string()))
        } else {
            Ok(None)
        }
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        let filter = doc! { "key": key };
        let update = doc! { "$set": { "value": value } };
        self.requests.update_one(filter, update, None).await?;
        Ok(())
    }
}