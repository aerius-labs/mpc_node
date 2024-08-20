use crate::common::{broadcast, poll_for_broadcasts, poll_for_p2p, sendp2p, Params, PartySignup};
use crate::queue::rabbitmq::RabbitMQService;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2018::party_i::*;
use curv::elliptic::curves::{Secp256k1, Point, Scalar};
use std::{fs, time};
use reqwest::Client;

pub struct SignerService {
    client: Client,
    queue: RabbitMQService,
    party_keys: Keys,
    shared_keys: SharedKeys,
    party_id: u16,
    manager_url: String,
}

impl SignerService {
    pub async fn new(manager_url: &str, rabbitmq_uri: &str, party_id: u16, key_file: &str) -> Result<Self, anyhow::Error> {
        let client = Client::new();
        let queue = RabbitMQService::new(rabbitmq_uri).await?;
        let (party_keys, shared_keys) = Self::load_keys(key_file)?;

        Ok(Self {
            client,
            queue,
            party_keys,
            shared_keys,
            party_id,
            manager_url: manager_url.to_string(),
        })
    }

    pub async fn run(&self) -> Result<(), anyhow::Error> {
        // Implement the main loop for the signer
        // This should handle incoming signing requests and participate in the signing process
        // ...

        Ok(())
    }

    async fn sign(&self, message: &[u8], params: &Params) -> Result<(), anyhow::Error> {
        // Implement the signing process from gg18_sign_client.rs
        // ...

        Ok(())
    }

    fn load_keys(key_file: &str) -> Result<(Keys, SharedKeys), anyhow::Error> {
        let data = fs::read_to_string(key_file)?;
        let (party_keys, shared_keys, ..) = serde_json::from_str(&data)?;
        Ok((party_keys, shared_keys))
    }

    // Implement other methods as needed
    // ...
}