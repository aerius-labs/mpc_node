use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub type Key = String;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MessageToSignStored {
    pub request_id: String,
    pub message: Vec<u8>,
    pub status: MessageStatus,
    pub signature: Option<SignatureData>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending,
    InProgress,
    Completed,
}

impl From<MessageStatus> for Bson {
    fn from(status: MessageStatus) -> Self {
        match status {
            MessageStatus::Pending => Bson::String("Pending".to_string()),
            MessageStatus::InProgress => Bson::String("InProgress".to_string()),
            MessageStatus::Completed => Bson::String("Completed".to_string()),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SignatureData {
    r: String,
    s: String,
    status: String,
    recid: i32,
    x: String,
    y: String,
    msg_int: Vec<u8>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SignerResult {
    pub request_id: String,
    pub signature: SignatureData,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AEAD {
    pub ciphertext: Vec<u8>,
    pub tag: Vec<u8>,
    pub nonce: Vec<u8>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PartySignup {
    pub number: u16,
    pub uuid: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Index {
    pub key: Key,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub key: Key,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Params {
    pub parties: u16,
    pub threshold: u16,
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyGenParams {
    pub parties: u16,
    pub threshold: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeysToStore {
    pub request_id: String,
    pub status: MessageStatus,
    pub key_gen_params: KeyGenParams,
    pub keys: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyGenRequest {
    pub id: String,
    pub keygen_params: KeyGenParams,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PartySignupRequestBody {
    pub threshold: u16,
    pub room_id: String,
    pub party_number: u16,
    pub party_uuid: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SigningPartySignup {
    pub party_order: u16,
    pub party_uuid: String,
    pub room_uuid: String,
    pub total_joined: u16,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ManagerError {
    pub error: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SigningRequest {
    pub id: String,
    pub message: Vec<u8>,
    // pub threshold: usize,
    // pub total_parties: usize,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SigningResult {
    pub request_id: String,
    pub signature: Option<Vec<u8>>,
    pub status: SigningStatus,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum SigningStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl Display for SigningStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SigningStatus::Pending => write!(f, "Pending"),
            SigningStatus::InProgress => write!(f, "InProgress"),
            SigningStatus::Completed => write!(f, "Completed"),
            SigningStatus::Failed => write!(f, "Failed"),
        }
    }
}

impl FromStr for SigningStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pending" => Ok(SigningStatus::Pending),
            "InProgress" => Ok(SigningStatus::InProgress),
            "Completed" => Ok(SigningStatus::Completed),
            "Failed" => Ok(SigningStatus::Failed),
            _ => Err(format!("Invalid SigningStatus: {}", s)),
        }
    }
}
