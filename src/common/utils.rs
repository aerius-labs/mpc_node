use crate::{common::types::*, manager::constants::NONCE_SIZE};
use aes_gcm::{
    aead::{Aead, NewAead, Payload},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use rand::{rngs::OsRng, RngCore};
use reqwest::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{thread, time};

pub fn aes_encrypt(key: &[u8], plaintext: &[u8]) -> AEAD {
    let mut key_sized = [0u8; 32];
    key_sized[(32 - key.len())..].copy_from_slice(key);
    let aes_key = aes_gcm::Key::from_slice(&key_sized);
    let cipher = Aes256Gcm::new(aes_key);

    let mut nonce_bytes: [u8; NONCE_SIZE] = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let out_tag = [0u8; 16];

    let text_payload = Payload {
        msg: plaintext,
        aad: &out_tag.as_slice(), // out tag is set to default value, no authentication data
    };

    let ciphertext = cipher
        .encrypt(nonce, text_payload)
        .expect("encryption failure!");

    AEAD {
        ciphertext,
        tag: out_tag.to_vec(),
        nonce: nonce.to_vec(),
    }
}

pub fn aes_decrypt(key: &[u8], aead_pack: AEAD) -> Vec<u8> {
    let mut key_sized = [0u8; 32];
    key_sized[(32 - key.len())..].copy_from_slice(key);
    let aes_key = aes_gcm::Key::from_slice(&key_sized);
    let gcm = Aes256Gcm::new(aes_key);

    let nonce = Nonce::from_slice(&aead_pack.nonce);

    let text_payload = Payload {
        msg: aead_pack.ciphertext.as_slice(),
        aad: aead_pack.tag.as_slice(),
    };

    let out = gcm.decrypt(nonce, text_payload);
    out.unwrap_or_default()
}

pub async fn postb<T>(addr: &str, client: &Client, path: &str, body: T) -> Option<String>
where
    T: serde::ser::Serialize,
{
    let retries = 3;
    let retry_delay = time::Duration::from_millis(250);
    let endpoint = format!("{}/{}", addr, path);
    for _ in 0..retries {
        match client.post(&endpoint).json(&body).send().await {
            Ok(response) => return Some(response.text().await.unwrap()),
            Err(_) => thread::sleep(retry_delay),
        }
    }
    None
}

pub async fn broadcast(
    addr: &str,
    client: &Client,
    party_num: u16,
    round: &str,
    data: String,
    sender_uuid: String,
) -> anyhow::Result<()> {
    let key = format!("{}-{}-{}", party_num, round, sender_uuid);
    let entry = Entry {
        key: key.clone(),
        value: data,
    };

    let res_body = postb(addr, client, "set", entry).await.unwrap();
    let parsed: Value = serde_json::from_str(&res_body)
        .map_err(|err| anyhow!("Failed to parse JSON response: {}", err))?;
    match parsed {
        Value::Object(map) if map.contains_key("Ok") => Ok(()),
        _ => Err(anyhow!("Unexpected response structure: {}", res_body)),
    }
}

pub async fn sendp2p(
    addr: &str,
    client: &Client,
    party_from: u16,
    party_to: u16,
    round: &str,
    data: String,
    sender_uuid: String,
) -> Result<(), ()> {
    let key = format!("{}-{}-{}-{}", party_from, party_to, round, sender_uuid);
    let entry = Entry {
        key: key.clone(),
        value: data,
    };

    let res_body = postb(addr, client, "set", entry).await.unwrap();
    println!("res_body: {}", res_body);
    serde_json::from_str(&res_body).unwrap()
}

pub async fn poll_for_broadcasts(
    addr: &str,
    client: &Client,
    party_num: u16,
    n: u16,
    delay: std::time::Duration,
    round: &str,
    sender_uuid: String,
) -> Vec<String> {
    let mut ans_vec = Vec::new();
    for i in 1..=n {
        if i != party_num {
            let key = format!("{}-{}-{}", i, round, sender_uuid);
            let index = Index { key };
            loop {
                thread::sleep(delay);
                let res_body = postb(addr, client, "get", index.clone()).await.unwrap();
                let answer: Result<Entry, ManagerError> = serde_json::from_str(&res_body).unwrap();
                match answer {
                    Ok(entry) => {
                        ans_vec.push(entry.value);
                        break;
                    }
                    Err(_) => continue,
                }
            }
        }
    }
    ans_vec
}

pub async fn poll_for_p2p(
    addr: &str,
    client: &Client,
    party_num: u16,
    n: u16,
    delay: std::time::Duration,
    round: &str,
    sender_uuid: String,
) -> Vec<String> {
    let mut ans_vec = Vec::new();
    for i in 1..=n {
        if i != party_num {
            let key = format!("{}-{}-{}-{}", i, party_num, round, sender_uuid);
            let index = Index { key };
            loop {
                thread::sleep(delay);
                let res_body = postb(addr, client, "get", index.clone()).await.unwrap();
                let answer: Result<Entry, ManagerError> = serde_json::from_str(&res_body).unwrap();
                match answer {
                    Ok(entry) => {
                        ans_vec.push(entry.value);
                        break;
                    }
                    Err(_) => continue,
                }
            }
        }
    }
    ans_vec
}

pub fn sha256_digest(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}
