use crate::common::{broadcast, poll_for_broadcasts, poll_for_p2p, sendp2p, Params, PartySignup, SigningRequest};
use crate::queue::rabbitmq::RabbitMQService;
use crate::error::TssError;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2018::party_i::*;
use curv::elliptic::curves::{Secp256k1, Point, Scalar};
use curv::BigInt;
use curv::arithmetic::Converter;
use std::{fs, time};
use reqwest::Client;
use tracing::{info, error};
use anyhow::{Context, Result};

pub struct SignerService {
    client: Client,
    queue: RabbitMQService,
    party_keys: Keys,
    shared_keys: SharedKeys,
    party_id: u16,
    manager_url: String,
}

impl SignerService {
    pub async fn new(manager_url: &str, rabbitmq_uri: &str, party_id: u16, key_file: &str) -> Result<Self> {
        let client = Client::new();
        let queue = RabbitMQService::new(rabbitmq_uri).await?;
        let (party_keys, shared_keys, ..): (Keys, SharedKeys, _, _, _) = serde_json::from_str(&key_file)?;

        Ok(Self {
            client,
            queue,
            party_keys,
            shared_keys,
            party_id,
            manager_url: manager_url.to_string(),
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting SignerService for party {}", self.party_id);
        loop {
            match self.queue.receive_signing_request().await {
                Ok(request) => {
                    if let Err(e) = self.handle_signing_request(request).await {
                        error!("Error handling signing request: {:?}", e);
                    }
                }
                Err(e) => {
                    error!("Error receiving signing request: {:?}", e);
                }
            }
        }
    }

    pub async fn handle_signing_request(&self, request: SigningRequest) -> Result<()> {
        info!("Handling signing request: {:?}", request);
        // let params = Params {
        //     threshold: self.party_keys.t.to_string(),
        //     parties: self.party_keys.n.to_string(),
        // };
        // self.sign(&request.message, &params).await?;
        Ok(())
    }

    async fn sign(&self, message: &[u8], params: &Params) -> Result<()> {
        let (party_num_int, uuid) = self.signup(params).await?;

        // Round 0: collect signers IDs
        self.broadcast_party_id(party_num_int, &uuid).await?;
        let signers_vec = self.collect_signer_ids(party_num_int, params, &uuid).await?;

        let sign_keys = SignKeys::create(
            &PartyPrivate::set_private(self.party_keys.clone(), self.shared_keys.clone()),
            &self.vss_scheme_vec[signers_vec[self.party_id as usize] as usize],
            signers_vec[self.party_id as usize],
            &signers_vec,
        );

        // Round 1: commit to ephemeral public keys
        let (com, decom) = sign_keys.phase1_broadcast();
        self.broadcast_commitment(party_num_int, &com, &uuid).await?;
        let commitments = self.collect_commitments(party_num_int, params, &uuid).await?;

        // Round 2: send ephemeral public keys and check commitments
        self.broadcast_decommitment(party_num_int, &decom, &uuid).await?;
        let decommitments = self.collect_decommitments(party_num_int, params, &uuid).await?;

        // Verify commitments and decommitments
        let mut j = 0;
        let mut point_vec: Vec<Point<Secp256k1>> = Vec::new();
        for i in 1..=params.parties.parse::<u16>()? {
            if i == party_num_int {
                point_vec.push(decom.g_gamma_i.clone());
            } else {
                let decom_j = &decommitments[j];
                assert_eq!(commitments[j].com, decom_j.blind_factor);
                point_vec.push(decom_j.g_gamma_i.clone());
                j += 1;
            }
        }

        let (head, tail) = point_vec.split_at(1);
        let y_sum = tail.iter().fold(head[0].clone(), |acc, x| acc + x);

        // Round 3: compute local signature share
        let message_bn = BigInt::from_bytes(message);
        let local_sig = LocalSignature::phase5_local_sig(
            &sign_keys.k_i,
            &message_bn,
            &y_sum,
            &sign_keys.gamma_i,
            &sign_keys.g_w_i
        );

        // Round 4: broadcast local signature share
        self.broadcast_local_signature(party_num_int, &local_sig, &uuid).await?;
        let local_signatures = self.collect_local_signatures(party_num_int, params, &uuid).await?;

        // Combine local signatures
        let mut local_sigs_vec = Vec::new();
        for sig in local_signatures {
            local_sigs_vec.push(sig);
        }
        local_sigs_vec.push(local_sig);

        let signature = local_sigs_vec[0].output_signature(&local_sigs_vec[1..].iter().map(|s| s.s_i.clone()).collect::<Vec<_>>()).unwrap();

        info!("Signature generated: {:?}", signature);

        // Send the signature back to the manager
        self.send_signature_to_manager(&signature).await?;

        Ok(())
    }

    async fn signup(&self, params: &Params) -> Result<(u16, String)> {
        let res_body = self.client.post(&format!("{}/signupsign", self.manager_url))
            .json(params)
            .send()
            .await?
            .text()
            .await?;
        let party_signup: PartySignup = serde_json::from_str(&res_body)?;
        Ok((party_signup.number, party_signup.uuid))
    }

    fn load_keys(key_file: &str) -> Result<(Keys, SharedKeys)> {
        let data = fs::read_to_string(key_file)?;
        let (party_keys, shared_keys, ..) = serde_json::from_str(&data)?;
        Ok((party_keys, shared_keys))
    }

    async fn broadcast_party_id(&self, party_num: u16, uuid: &str) -> Result<()> {
        broadcast(
            &self.manager_url,
            &self.client,
            party_num,
            "round0",
            self.party_id.to_string(),
            uuid.to_string(),
        ).await.context("Failed to broadcast party ID")
    }

    async fn collect_signer_ids(&self, party_num: u16, params: &Params, uuid: &str) -> Result<Vec<u16>> {
        let round0_ans_vec = poll_for_broadcasts(
            &self.manager_url,
            &self.client,
            party_num,
            params.parties.parse()?,
            time::Duration::from_millis(25),
            "round0",
            uuid.to_string(),
        ).await?;

        let mut signers_vec = vec![self.party_id];
        for ans in round0_ans_vec {
            signers_vec.push(ans.parse()?);
        }
        Ok(signers_vec)
    }

    async fn broadcast_commitment(&self, party_num: u16, com: &SignBroadcastPhase1, uuid: &str) -> Result<()> {
        broadcast(
            &self.manager_url,
            &self.client,
            party_num,
            "round1",
            serde_json::to_string(com)?,
            uuid.to_string(),
        ).await.context("Failed to broadcast commitment")
    }

    async fn collect_commitments(&self, party_num: u16, params: &Params, uuid: &str) -> Result<Vec<SignBroadcastPhase1>> {
        let round1_ans_vec = poll_for_broadcasts(
            &self.manager_url,
            &self.client,
            party_num,
            params.parties.parse()?,
            time::Duration::from_millis(25),
            "round1",
            uuid.to_string(),
        ).await?;

        let mut commitments = vec![];
        for ans in round1_ans_vec {
            commitments.push(serde_json::from_str(&ans)?);
        }
        Ok(commitments)
    }

    async fn broadcast_decommitment(&self, party_num: u16, decom: &SignDecommitPhase1, uuid: &str) -> Result<()> {
        broadcast(
            &self.manager_url,
            &self.client,
            party_num,
            "round2",
            serde_json::to_string(decom)?,
            uuid.to_string(),
        ).await.context("Failed to broadcast decommitment")
    }

    async fn collect_decommitments(&self, party_num: u16, params: &Params, uuid: &str) -> Result<Vec<SignDecommitPhase1>> {
        let round2_ans_vec = poll_for_broadcasts(
            &self.manager_url,
            &self.client,
            party_num,
            params.parties.parse()?,
            time::Duration::from_millis(25),
            "round2",
            uuid.to_string(),
        ).await?;

        let mut decommitments = vec![];
        for ans in round2_ans_vec {
            decommitments.push(serde_json::from_str(&ans)?);
        }
        Ok(decommitments)
    }

    async fn broadcast_local_signature(&self, party_num: u16, local_sig: &LocalSignature, uuid: &str) -> Result<()> {
        broadcast(
            &self.manager_url,
            &self.client,
            party_num,
            "round3",
            serde_json::to_string(local_sig)?,
            uuid.to_string(),
        ).await.context("Failed to broadcast local signature")
    }

    async fn collect_local_signatures(&self, party_num: u16, params: &Params, uuid: &str) -> Result<Vec<LocalSignature>> {
        let round3_ans_vec = poll_for_broadcasts(
            &self.manager_url,
            &self.client,
            party_num,
            params.parties.parse()?,
            time::Duration::from_millis(25),
            "round3",
            uuid.to_string(),
        ).await?;

        let mut local_signatures = vec![];
        for ans in round3_ans_vec {
            local_signatures.push(serde_json::from_str(&ans)?);
        }
        Ok(local_signatures)
    }

    async fn send_signature_to_manager(&self, signature: &SignatureRecid) -> Result<()> {
        // Implement logic to send the final signature back to the manager
        // This could involve sending an HTTP request to a specific endpoint on the manager
        todo!("Implement send_signature_to_manager")
    }
}