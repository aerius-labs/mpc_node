use crate::queue::rabbitmq::RabbitMQService;
use anyhow::{anyhow, Result};
use curv::arithmetic::{BasicOps, Converter, Modulo};
use curv::cryptographic_primitives::proofs::sigma_correct_homomorphic_elgamal_enc::HomoELGamalProof;
use curv::cryptographic_primitives::proofs::sigma_dlog::DLogProof;
use curv::cryptographic_primitives::secret_sharing::feldman_vss::VerifiableSS;
use curv::elliptic::curves::Secp256k1;
use curv::BigInt;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2018::party_i::*;
use multi_party_ecdsa::utilities::mta::{MessageA, MessageB};
use paillier::EncryptionKey;
use reqwest::Client;
use serde_json::{json, Value};
use sha2::Sha256;
use std::fs::File;
use std::io::Read;
use std::{thread, time};
use tracing::{error, info};

use crate::common::{
    broadcast, poll_for_broadcasts, poll_for_p2p, postb, sendp2p, sha256_digest, ManagerError,
    Params, PartySignup, PartySignupRequestBody, SignatureData, SignerResult, SigningPartySignup,
    SigningRequest,
};
use crate::signer::hd_keys;
use crate::signer::secp256k1def::{FE, GE};

struct SignerData {
    party_keys: Keys,
    shared_keys: SharedKeys,
    party_id: u16,
    vss_scheme_vec: Vec<VerifiableSS<Secp256k1>>,
    paillier_key_vector: Vec<EncryptionKey>,
    y_sum: GE,
}

pub struct SignerService {
    queue: RabbitMQService,
    manager_url: String,
    manager_port: String,
    signer_data: SignerData,
    threshold: u16,
    total_parties: u16,
    path: String,
}

#[allow(non_snake_case)]
impl SignerService {
    pub async fn new(
        manager_url: &str,
        manager_port: &u16,
        rabbitmq_uri: &str,
        key_file: &str,
        threshold: &u16,
        total_parties: &u16,
        path: &str,
    ) -> Result<Self> {
        let queue = RabbitMQService::new(rabbitmq_uri).await?;
        let mut file = File::open(key_file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let (party_keys, shared_keys, party_id, vss_scheme_vec, paillier_key_vector, y_sum): (
            Keys,
            SharedKeys,
            u16,
            Vec<VerifiableSS<Secp256k1>>,
            Vec<EncryptionKey>,
            GE,
        ) = serde_json::from_str(&contents).unwrap();

        Ok(Self {
            queue,
            manager_url: manager_url.to_string(),
            manager_port: manager_port.to_string(),
            signer_data: SignerData {
                party_keys,
                shared_keys,
                party_id,
                vss_scheme_vec,
                paillier_key_vector,
                y_sum,
            },
            threshold: *threshold,
            total_parties: *total_parties,
            path: path.to_string(),
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting SignerService for party {}",
            self.signer_data.party_id
        );
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
        // this can be dynamic as well
        let params = Params {
            threshold: self.threshold,
            parties: self.total_parties,
            path: self.path.clone(),
        };
        self.sign(&request.message, &request.id, &params).await;
        Ok(())
    }

    pub async fn sign(&self, message: &[u8], request_id: &str, params: &Params) {
        let client = Client::new();
        let delay = time::Duration::from_millis(250);
        let room_id = sha256_digest(message);
        let path_is_empty = params.path.is_empty();
        let (f_l_new, y_sum) = match path_is_empty {
            true => (FE::zero(), self.signer_data.y_sum.clone()),
            false => call_hd_key(&params.path, self.signer_data.y_sum.clone()),
        };

        let addr = format!("{}:{}", self.manager_url, self.manager_port);
        let party_keys = self.signer_data.party_keys.clone();
        let shared_keys = self.signer_data.shared_keys.clone();
        let party_id = self.signer_data.party_id;
        let mut vss_scheme_vec = self.signer_data.vss_scheme_vec.clone();
        let paillier_key_vector = self.signer_data.paillier_key_vector.clone();
        let sign_at_path = !path_is_empty;

        // Signup
        let (party_num_int, uuid, total_parties) =
            match Self::signup(&addr, &client, self.threshold, room_id, party_id)
                .await
                .unwrap()
            {
                (PartySignup { number, uuid }, total_parties) => (number, uuid, total_parties),
            };

        let debug = json!({"manager_addr": &addr, "party_num": party_num_int, "uuid": uuid});
        println!("{}", serde_json::to_string_pretty(&debug).unwrap());

        // round 0: collect signers IDs
        assert!(broadcast(
            &addr,
            &client,
            party_num_int,
            "round0",
            serde_json::to_string(&party_id).unwrap(),
            uuid.clone(),
        )
        .await
        .is_ok());

        let round0_ans_vec = poll_for_broadcasts(
            &addr,
            &client,
            party_num_int,
            total_parties,
            delay,
            "round0",
            uuid.clone(),
        )
        .await;

        let mut j = 0;
        let mut signers_vec: Vec<u16> = Vec::new();
        for i in 1..=total_parties {
            if i == party_num_int {
                signers_vec.push(party_id - 1);
            } else {
                let signer_j: u16 = serde_json::from_str(&round0_ans_vec[j]).unwrap();
                signers_vec.push(signer_j - 1);
                j += 1;
            }
        }

        if sign_at_path {
            // optimize!
            let g: GE = GE::generator().to_point();
            // apply on first commitment for leader (leader is party with num=1)
            let com_zero_new = &vss_scheme_vec[0].commitments[0] + g * &f_l_new;
            // println!("old zero: {:?}, new zero: {:?}", vss_scheme_vec[0].commitments[0], com_zero_new);
            // get iterator of all commitments and skip first zero commitment
            let mut com_iter_unchanged = vss_scheme_vec[0].commitments.iter();
            com_iter_unchanged.next().unwrap();
            // iterate commitments and inject changed commitments in the beginning then aggregate into vector
            let com_vec_new = (0..vss_scheme_vec[1].commitments.len())
                .map(|i| {
                    if i == 0 {
                        com_zero_new.clone()
                    } else {
                        com_iter_unchanged.next().unwrap().clone()
                    }
                })
                .collect::<Vec<GE>>();
            let new_vss = VerifiableSS {
                parameters: vss_scheme_vec[0].parameters.clone(),
                commitments: com_vec_new,
            };
            // replace old vss_scheme for leader with new one at position 0
            //    println!("comparing vectors: \n{:?} \nand \n{:?}", vss_scheme_vec[0], new_vss);

            vss_scheme_vec.remove(0);
            vss_scheme_vec.insert(0, new_vss);
            //    println!("NEW VSS VECTOR: {:?}", vss_scheme_vec);
        }

        let mut private = PartyPrivate::set_private(party_keys.clone(), shared_keys);

        if sign_at_path {
            if party_num_int == 1 {
                // update u_i and x_i for leader
                private = private.update_private_key(&f_l_new, &f_l_new);
            } else {
                // only update x_i for non-leaders
                private = private.update_private_key(&FE::zero(), &f_l_new);
            }
        }

        let sign_keys = SignKeys::create(
            &private,
            &vss_scheme_vec[signers_vec[(party_num_int - 1) as usize] as usize],
            signers_vec[(party_num_int - 1) as usize],
            &signers_vec,
        );

        //////////////////////////////////////////////////////////////////////////////
        let (com, decommit) = sign_keys.phase1_broadcast();
        let (m_a_k, _) = MessageA::a(&sign_keys.k_i, &party_keys.ek, &[]);
        assert!(broadcast(
            &addr,
            &client,
            party_num_int,
            "round1",
            serde_json::to_string(&(com.clone(), m_a_k.clone())).unwrap(),
            uuid.clone(),
        )
        .await
        .is_ok());
        let round1_ans_vec = poll_for_broadcasts(
            &addr,
            &client,
            party_num_int,
            total_parties,
            delay,
            "round1",
            uuid.clone(),
        )
        .await;

        let mut j = 0;
        let mut bc1_vec: Vec<SignBroadcastPhase1> = Vec::new();
        let mut m_a_vec: Vec<MessageA> = Vec::new();

        for i in 1..total_parties + 1 {
            if i == party_num_int {
                bc1_vec.push(com.clone());
                //   m_a_vec.push(m_a_k.clone());
            } else {
                //     if signers_vec.contains(&(i as usize)) {
                let (bc1_j, m_a_party_j): (SignBroadcastPhase1, MessageA) =
                    serde_json::from_str(&round1_ans_vec[j]).unwrap();
                bc1_vec.push(bc1_j);
                m_a_vec.push(m_a_party_j);

                j += 1;
                //       }
            }
        }
        assert_eq!(signers_vec.len(), bc1_vec.len());

        //////////////////////////////////////////////////////////////////////////////
        let mut m_b_gamma_send_vec: Vec<MessageB> = Vec::new();
        let mut beta_vec: Vec<FE> = Vec::new();
        let mut m_b_w_send_vec: Vec<MessageB> = Vec::new();
        let mut ni_vec: Vec<FE> = Vec::new();
        let mut j = 0;
        for i in 1..(total_parties as usize) + 1 {
            if i != party_num_int as usize {
                let (m_b_gamma, beta_gamma, _, _) = MessageB::b(
                    &sign_keys.gamma_i,
                    &paillier_key_vector[signers_vec[i - 1] as usize],
                    m_a_vec[j].clone(),
                    &[],
                )
                .unwrap();
                let (m_b_w, beta_wi, _, _) = MessageB::b(
                    &sign_keys.w_i,
                    &paillier_key_vector[signers_vec[i - 1] as usize],
                    m_a_vec[j].clone(),
                    &[],
                )
                .unwrap();
                m_b_gamma_send_vec.push(m_b_gamma);
                m_b_w_send_vec.push(m_b_w);
                beta_vec.push(beta_gamma);
                ni_vec.push(beta_wi);
                j += 1;
            }
        }

        let mut j = 0;
        for i in 1..total_parties + 1 {
            if i != party_num_int {
                assert!(sendp2p(
                    &addr,
                    &client,
                    party_num_int,
                    i,
                    "round2",
                    serde_json::to_string(&(
                        m_b_gamma_send_vec[j].clone(),
                        m_b_w_send_vec[j].clone()
                    ))
                    .unwrap(),
                    uuid.clone(),
                )
                .await
                .is_ok());
                j += 1;
            }
        }

        let round2_ans_vec = poll_for_p2p(
            &addr,
            &client,
            party_num_int,
            total_parties,
            delay,
            "round2",
            uuid.clone(),
        )
        .await;

        let mut m_b_gamma_rec_vec: Vec<MessageB> = Vec::new();
        let mut m_b_w_rec_vec: Vec<MessageB> = Vec::new();

        for i in 0..total_parties - 1 {
            //  if signers_vec.contains(&(i as usize)) {
            let (m_b_gamma_i, m_b_w_i): (MessageB, MessageB) =
                serde_json::from_str(&round2_ans_vec[i as usize]).unwrap();
            m_b_gamma_rec_vec.push(m_b_gamma_i);
            m_b_w_rec_vec.push(m_b_w_i);
            //     }
        }

        let mut alpha_vec: Vec<FE> = Vec::new();
        let mut miu_vec: Vec<FE> = Vec::new();

        let xi_com_vec = Keys::get_commitments_to_xi(&vss_scheme_vec);
        let mut j = 0;
        for i in 1..(total_parties as usize) + 1 {
            //        println!("mbproof p={}, i={}, j={}", party_num_int, i, j);
            if i != party_num_int as usize {
                //            println!("verifying: p={}, i={}, j={}", party_num_int, i, j);
                let m_b = m_b_gamma_rec_vec[j].clone();

                let alpha_ij_gamma = m_b
                    .verify_proofs_get_alpha(&party_keys.dk, &sign_keys.k_i)
                    .expect("wrong dlog or m_b");
                let m_b = m_b_w_rec_vec[j].clone();
                let alpha_ij_wi = m_b
                    .verify_proofs_get_alpha(&party_keys.dk, &sign_keys.k_i)
                    .expect("wrong dlog or m_b");
                alpha_vec.push(alpha_ij_gamma.0);
                miu_vec.push(alpha_ij_wi.0);
                let g_w_i = Keys::update_commitments_to_xi(
                    &xi_com_vec[signers_vec[i - 1] as usize],
                    &vss_scheme_vec[signers_vec[i - 1] as usize],
                    signers_vec[i - 1],
                    &signers_vec,
                );
                //println!("Verifying client {}", party_num_int);
                assert_eq!(m_b.b_proof.pk.clone(), g_w_i);
                //println!("Verified client {}", party_num_int);
                j += 1;
            }
        }
        //////////////////////////////////////////////////////////////////////////////
        let delta_i = sign_keys.phase2_delta_i(&alpha_vec, &beta_vec);
        let sigma = sign_keys.phase2_sigma_i(&miu_vec, &ni_vec);

        assert!(broadcast(
            &addr,
            &client,
            party_num_int,
            "round3",
            serde_json::to_string(&delta_i).unwrap(),
            uuid.clone(),
        )
        .await
        .is_ok());
        let round3_ans_vec = poll_for_broadcasts(
            &addr,
            &client,
            party_num_int,
            total_parties,
            delay,
            "round3",
            uuid.clone(),
        )
        .await;
        let mut delta_vec: Vec<FE> = Vec::new();
        format_vec_from_reads(
            &round3_ans_vec,
            party_num_int as usize,
            delta_i,
            &mut delta_vec,
        );
        let delta_inv = SignKeys::phase3_reconstruct_delta(&delta_vec);

        //////////////////////////////////////////////////////////////////////////////
        // decommit to gamma_i
        assert!(broadcast(
            &addr,
            &client,
            party_num_int,
            "round4",
            serde_json::to_string(&decommit).unwrap(),
            uuid.clone(),
        )
        .await
        .is_ok());
        let round4_ans_vec = poll_for_broadcasts(
            &addr,
            &client,
            party_num_int,
            total_parties,
            delay,
            "round4",
            uuid.clone(),
        )
        .await;

        let mut decommit_vec: Vec<SignDecommitPhase1> = Vec::new();
        format_vec_from_reads(
            &round4_ans_vec,
            party_num_int as usize,
            decommit,
            &mut decommit_vec,
        );
        let decomm_i = decommit_vec.remove((party_num_int - 1) as usize);
        bc1_vec.remove((party_num_int - 1) as usize);
        let b_proof_vec = (0..m_b_gamma_rec_vec.len())
            .map(|i| &m_b_gamma_rec_vec[i].b_proof)
            .collect::<Vec<&DLogProof<Secp256k1, Sha256>>>();

        let R = SignKeys::phase4(&delta_inv, &b_proof_vec, decommit_vec, &bc1_vec)
            .expect("bad gamma_i decommit");

        // adding local g_gamma_i
        let R = R + decomm_i.g_gamma_i * &delta_inv;

        // we assume the message is already hashed (by the signer).
        let message_bn = BigInt::from_bytes(message);
        //    println!("message_bn INT: {}", message_bn);
        let message_int = BigInt::from_bytes(message);
        let two = BigInt::from(2);
        let message_bn = message_bn.modulus(&two.pow(256));
        let local_sig =
            LocalSignature::phase5_local_sig(&sign_keys.k_i, &message_bn, &R, &sigma, &y_sum);

        let (phase5_com, phase_5a_decom, helgamal_proof, dlog_proof_rho) =
            local_sig.phase5a_broadcast_5b_zkproof();

        //phase (5A)  broadcast commit
        assert!(broadcast(
            &addr,
            &client,
            party_num_int,
            "round5",
            serde_json::to_string(&phase5_com).unwrap(),
            uuid.clone(),
        )
        .await
        .is_ok());
        let round5_ans_vec = poll_for_broadcasts(
            &addr,
            &client,
            party_num_int,
            total_parties,
            delay,
            "round5",
            uuid.clone(),
        )
        .await;

        let mut commit5a_vec: Vec<Phase5Com1> = Vec::new();
        format_vec_from_reads(
            &round5_ans_vec,
            party_num_int as usize,
            phase5_com,
            &mut commit5a_vec,
        );

        //phase (5B)  broadcast decommit and (5B) ZK proof
        assert!(broadcast(
            &addr,
            &client,
            party_num_int,
            "round6",
            serde_json::to_string(&(
                phase_5a_decom.clone(),
                helgamal_proof.clone(),
                dlog_proof_rho.clone()
            ))
            .unwrap(),
            uuid.clone(),
        )
        .await
        .is_ok());
        let round6_ans_vec = poll_for_broadcasts(
            &addr,
            &client,
            party_num_int,
            total_parties,
            delay,
            "round6",
            uuid.clone(),
        )
        .await;

        let mut decommit5a_and_elgamal_and_dlog_vec: Vec<(
            Phase5ADecom1,
            HomoELGamalProof<Secp256k1, Sha256>,
            DLogProof<Secp256k1, Sha256>,
        )> = Vec::new();
        format_vec_from_reads(
            &round6_ans_vec,
            party_num_int as usize,
            (
                phase_5a_decom.clone(),
                helgamal_proof.clone(),
                dlog_proof_rho.clone(),
            ),
            &mut decommit5a_and_elgamal_and_dlog_vec,
        );
        let decommit5a_and_elgamal_vec_includes_i = decommit5a_and_elgamal_and_dlog_vec.clone();
        decommit5a_and_elgamal_and_dlog_vec.remove((party_num_int - 1) as usize);
        commit5a_vec.remove((party_num_int - 1) as usize);
        let phase_5a_decomm_vec = (0..total_parties - 1)
            .map(|i| decommit5a_and_elgamal_and_dlog_vec[i as usize].0.clone())
            .collect::<Vec<Phase5ADecom1>>();
        let phase_5a_elgamal_vec = (0..total_parties - 1)
            .map(|i| decommit5a_and_elgamal_and_dlog_vec[i as usize].1.clone())
            .collect::<Vec<HomoELGamalProof<Secp256k1, Sha256>>>();
        let phase_5a_dlog_vec = (0..total_parties - 1)
            .map(|i| decommit5a_and_elgamal_and_dlog_vec[i as usize].2.clone())
            .collect::<Vec<DLogProof<Secp256k1, Sha256>>>();
        let (phase5_com2, phase_5d_decom2) = local_sig
            .phase5c(
                &phase_5a_decomm_vec,
                &commit5a_vec,
                &phase_5a_elgamal_vec,
                &phase_5a_dlog_vec,
                &phase_5a_decom.V_i,
                &R.clone(),
            )
            .expect("error phase5");

        //////////////////////////////////////////////////////////////////////////////
        assert!(broadcast(
            &addr,
            &client,
            party_num_int,
            "round7",
            serde_json::to_string(&phase5_com2).unwrap(),
            uuid.clone(),
        )
        .await
        .is_ok());
        let round7_ans_vec = poll_for_broadcasts(
            &addr,
            &client,
            party_num_int,
            total_parties,
            delay,
            "round7",
            uuid.clone(),
        )
        .await;

        let mut commit5c_vec: Vec<Phase5Com2> = Vec::new();
        format_vec_from_reads(
            &round7_ans_vec,
            party_num_int as usize,
            phase5_com2,
            &mut commit5c_vec,
        );

        //phase (5B)  broadcast decommit and (5B) ZK proof
        assert!(broadcast(
            &addr,
            &client,
            party_num_int,
            "round8",
            serde_json::to_string(&phase_5d_decom2).unwrap(),
            uuid.clone(),
        )
        .await
        .is_ok());
        let round8_ans_vec = poll_for_broadcasts(
            &addr,
            &client,
            party_num_int,
            total_parties,
            delay,
            "round8",
            uuid.clone(),
        )
        .await;

        let mut decommit5d_vec: Vec<Phase5DDecom2> = Vec::new();
        format_vec_from_reads(
            &round8_ans_vec,
            party_num_int as usize,
            phase_5d_decom2.clone(),
            &mut decommit5d_vec,
        );

        let phase_5a_decomm_vec_includes_i = (0..total_parties)
            .map(|i| decommit5a_and_elgamal_vec_includes_i[i as usize].0.clone())
            .collect::<Vec<Phase5ADecom1>>();
        let s_i = local_sig
            .phase5d(
                &decommit5d_vec,
                &commit5c_vec,
                &phase_5a_decomm_vec_includes_i,
            )
            .expect("bad com 5d");

        //////////////////////////////////////////////////////////////////////////////
        assert!(broadcast(
            &addr,
            &client,
            party_num_int,
            "round9",
            serde_json::to_string(&s_i).unwrap(),
            uuid.clone(),
        )
        .await
        .is_ok());
        let round9_ans_vec = poll_for_broadcasts(
            &addr,
            &client,
            party_num_int,
            total_parties,
            delay,
            "round9",
            uuid.clone(),
        )
        .await;

        let mut s_i_vec: Vec<FE> = Vec::new();
        format_vec_from_reads(&round9_ans_vec, party_num_int as usize, s_i, &mut s_i_vec);

        s_i_vec.remove((party_num_int - 1) as usize);
        let sig = local_sig
            .output_signature(&s_i_vec)
            .expect("verification failed");
        //    println!(" \n");
        //    println!("party {:?} Output Signature: \n", party_num_int);
        //    println!("SIG msg: {:?}", sig.m);
        //    println!("R: {:?}", sig.r);
        //    println!("s: {:?} \n", sig.s);
        //    println!("child pubkey: {:?} \n", y_sum);

        //    println!("pubkey: {:?} \n", y_sum);
        //    println!("verifying signature with public key");
        verify(&sig, &y_sum, &message_bn).expect("false");
        //    println!("verifying signature with child pub key");
        //    verify(&sig, &new_key, &message_bn).expect("false");

        //    println!("{:?}", sig.recid.clone());
        //    print(sig.recid.clone()

        let ret_dict = json!({
            "r": (BigInt::from_bytes(sig.r.to_bytes().as_ref())).to_str_radix(16),
            "s": (BigInt::from_bytes(sig.s.to_bytes().as_ref())).to_str_radix(16),
            "status": "signature_ready",
            "recid": sig.recid.clone(),
            "x": &y_sum.x_coord().unwrap().to_hex(),
            "y": &y_sum.y_coord().unwrap().to_hex(),
            "msg_int": message_int,
        });
        let signature: SignatureData = serde_json::from_value(ret_dict).unwrap();
        match self
            .send_signature_to_manager(&addr, &client, &signature, request_id)
            .await
        {
            Ok(_) => info!("Signature sent to manager"),
            Err(e) => error!("Error sending signature to manager: {:?}", e),
        }
        //    fs::write("signature".to_string(), sign_json).expect("Unable to save !");

        //    println!("Public key Y: {:?}", to_bitcoin_public_key(y_sum.get_element()).to_bytes());
        //    println!("Public child key X: {:?}", &new_key.x_coord());
        //    println!("Public child key Y: {:?}", &new_key.y_coord());
        //    println!("Public key big int: {:?}", &y_sum.bytes_compressed_to_big_int());
        //    println!("Public key ge: {:?}", &y_sum.get_element().serialize());
        //    println!("Public key ge: {:?}", PK::serialize_uncompressed(&y_sum.get_element()));
        //    println!("New public key: {:?}", &y_sum.x_coor);
    }

    async fn signup(
        addr: &String,
        client: &Client,
        threshold: u16,
        room_id: String,
        party_id: u16,
    ) -> Result<(PartySignup, u16), ()> {
        let mut request_body = PartySignupRequestBody {
            threshold,
            room_id: room_id.clone(),
            party_number: party_id,
            party_uuid: "".to_string(),
        };
        let path = "signupsign";
        let delay = time::Duration::from_millis(100);
        let timeout = std::env::var("TSS_CLI_SIGNUP_TIMEOUT")
            .unwrap_or("30".to_string())
            .parse::<u64>()
            .unwrap();
        let res_body: String = postb(addr, client, path, request_body.clone())
            .await
            .unwrap();

        let answer: Result<SigningPartySignup, ManagerError> =
            serde_json::from_str(&res_body).unwrap();
        let (output, total_parties) = match answer {
            Ok(SigningPartySignup {
                party_order,
                party_uuid,
                room_uuid,
                total_joined,
            }) => {
                println!(
                    "Signed up, party order: {:?}, joined so far: {:?}, waiting for room uuid",
                    party_order, total_joined
                );
                let mut now = time::SystemTime::now();
                let mut last_total_joined = total_joined;
                let mut party_signup = PartySignup {
                    number: party_order,
                    uuid: room_uuid,
                };
                while party_signup.uuid.is_empty() {
                    thread::sleep(delay);
                    request_body.party_uuid = party_uuid.clone();
                    let res_body = postb(addr, client, path, request_body.clone())
                        .await
                        .unwrap();
                    let answer: Result<SigningPartySignup, ManagerError> =
                        serde_json::from_str(&res_body).unwrap();
                    match answer {
                        Ok(SigningPartySignup {
                            party_order,
                            party_uuid,
                            room_uuid,
                            total_joined,
                        }) => {
                            request_body.party_uuid = party_uuid;
                            if party_signup.number != party_order {
                                println!("Order is changed: {:?}", party_order);
                                party_signup.number = party_order;
                            }
                            party_signup.uuid = room_uuid;
                            if total_joined != last_total_joined {
                                println!("Joined so far: {:?}", total_joined);
                                last_total_joined = total_joined;
                                //Reset the signup timeout
                                now = time::SystemTime::now();
                            }
                        }
                        Err(ManagerError { error }) => {
                            panic!("{}", error);
                        }
                    };
                    if now.elapsed().unwrap().as_secs() > timeout {
                        break;
                    }
                }
                if party_signup.uuid.is_empty() {
                    panic!(
                        "Could not get room uuid after {:?} seconds of tries",
                        timeout
                    );
                }
                (party_signup, last_total_joined)
            }
            Err(ManagerError { error }) => {
                panic!("{}", error);
            }
        };

        Ok((output, total_parties))
    }

    async fn send_signature_to_manager(
        &self,
        addr: &str,
        client: &Client,
        signature: &SignatureData,
        request_id: &str,
    ) -> Result<()> {
        let signer_result = SignerResult {
            request_id: request_id.to_string(),
            signature: signature.clone(),
        };
        let res_body = postb::<SignerResult>(addr, client, "update_signing_result", signer_result)
            .await
            .unwrap();
        let parsed: Value = serde_json::from_str(&res_body)
            .map_err(|err| anyhow!("Failed to parse response from manager: {:?}", err))?;

        match parsed {
            Value::Object(map) if map.contains_key("Ok") => {
                info!("Signature sent to manager");
                Ok(())
            }
            _ => Err(anyhow!("Failed to send signature to manager: {:?}", parsed)),
        }
    }
}

fn call_hd_key(path: &str, public_key: GE) -> (FE, GE) {
    let path_vector: Vec<BigInt> = path
        .split('/')
        .map(|s| BigInt::from_str_radix(s.trim(), 10).unwrap())
        .collect();
    let (public_key_child, f_l_new) = hd_keys::get_hd_key(&public_key, path_vector.clone());
    (f_l_new, public_key_child.clone())
}

fn format_vec_from_reads<'a, T: serde::Deserialize<'a> + Clone>(
    ans_vec: &'a Vec<String>,
    party_num: usize,
    value_i: T,
    new_vec: &'a mut Vec<T>,
) {
    let mut j = 0;
    for i in 1..ans_vec.len() + 2 {
        if i == party_num {
            new_vec.push(value_i.clone());
        } else {
            let value_j: T = serde_json::from_str(&ans_vec[j]).unwrap();
            new_vec.push(value_j);
            j += 1;
        }
    }
}
