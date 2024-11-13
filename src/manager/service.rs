use crate::common::{
    Key, KeyGenRequest, KeysToStore, MessageToSignStored, SignerResult, SigningRequest,
};
use crate::queue::rabbitmq::RabbitMQService;
use crate::storage::mongodb::MongoDBStorage;
use anyhow::Result;
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;
use tracing::info;

use super::keygen::run_keygen;

pub struct ManagerService {
    pub storage: MongoDBStorage,
    pub queue: RabbitMQService,
    pub(crate) signing_rooms: Arc<RwLock<HashMap<Key, String>>>,
    pub threshold: u16,
    pub total_parties: u16,
}

impl ManagerService {
    pub async fn new(
        mongodb_uri: &str,
        rabbitmq_uri: &str,
        threshold: u16,
        total_parties: u16,
    ) -> Result<Self> {
        let storage = MongoDBStorage::new(mongodb_uri, "tss_network").await?;
        let queue = RabbitMQService::new(rabbitmq_uri).await?;

        Ok(Self {
            storage,
            queue,
            signing_rooms: Arc::new(RwLock::new(HashMap::new())),
            threshold,
            total_parties,
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting ManagerService");
        loop {
            // match self.queue.receive_signing_request().await {
            //     Ok(request) => {
            //         if let Err(e) = self.handle_signing_request(request).await {
            //             error!("Failed to handle signing request: {:?}", e);
            //         }
            //     }
            //     Err(e) => {
            //         error!("Failed to receive signing request: {:?}", e);
            //     }
            // }
        }
    }

    pub async fn get_signing_result(
        &self,
        request_id: &str,
    ) -> Result<Option<MessageToSignStored>> {
        self.storage.get_signing_result(request_id).await
    }

    pub async fn update_signing_result(&self, result: SignerResult) -> Result<()> {
        self.storage.update_signing_result(&result).await
    }

    pub async fn process_signing_request(&self, request: SigningRequest) -> Result<()> {
        self.storage.insert_request(&request).await?;
        self.queue.publish_signing_request(&request).await?;
        Ok(())
    }

    pub async fn process_keygen_request(
        &self,
        request: KeyGenRequest,
        manager_addr: &String,
    ) -> Result<Vec<String>> {
        self.storage.insert_key_gen_request(&request).await?;
        let total_parties = request.keygen_params.parties;
        let tasks: Vec<_> = (0..total_parties)
            .map(|_| {
                let manager_addr = manager_addr.clone();
                let request = request.clone();
                task::spawn(async move { run_keygen(&manager_addr, &request).await })
            })
            .collect();

        let results = join_all(tasks).await;

        // Collect successful results and aggregate errors
        let mut successful_results = Vec::new();
        let mut errors = Vec::new();

        for (index, res) in results.into_iter().enumerate() {
            match res {
                Ok(Ok(json_str)) => successful_results.push(json_str),
                Ok(Err(e)) => errors.push(format!("Error in party {}: {}", index, e)),
                Err(e) => errors.push(format!("Task error in party {}: {}", index, e)),
            }
        }

        if !errors.is_empty() {
            eprintln!("Errors occurred during key generation: {:?}", errors);
        }

        self.storage
            .update_key_gen_result(&request.id, successful_results.clone())
            .await?;
        Ok(successful_results)
    }

    pub async fn get_key_gen_result(&self, request_id: &str) -> Result<Option<KeysToStore>> {
        self.storage.get_key_gen_result(request_id).await
    }
}
