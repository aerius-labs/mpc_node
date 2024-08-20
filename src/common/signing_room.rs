use crate::common::types::*;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

pub struct SigningRoom {
    pub room_id: String,
    pub room_uuid: String,
    pub room_size: u16,
    pub member_info: HashMap<u16, SigningPartyInfo>,
    pub last_stage: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SigningPartyInfo {
    pub party_id: String,
    pub party_order: u16,
    pub last_ping: u64,
}

impl SigningRoom {
    pub fn new(room_id: String, size: u16) -> Self {
        SigningRoom {
            room_size: size,
            member_info: Default::default(),
            room_id,
            last_stage: "signup".to_string(),
            room_uuid: Uuid::new_v4().to_string(),
        }
    }

    pub fn is_full(&self) -> bool {
        self.member_info.len() >= usize::from(self.room_size)
    }

    pub fn add_party(&mut self, party_number: u16, party_uuid: String) -> Result<SigningPartySignup, String> {
        if self.is_full() {
            return Err("Room is full".to_string());
        }

        let party_order = self.member_info.len() as u16 + 1;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        self.member_info.insert(party_number, SigningPartyInfo {
            party_id: party_uuid.clone(),
            party_order,
            last_ping: now,
        });

        Ok(SigningPartySignup {
            party_order,
            party_uuid,
            room_uuid: self.room_uuid.clone(),
            total_joined: self.member_info.len() as u16,
        })
    }

    pub fn update_ping(&mut self, party_number: u16) -> Result<SigningPartySignup, String> {
        let party_info = self.member_info.get_mut(&party_number)
            .ok_or_else(|| "Party not found".to_string())?;

        party_info.last_ping = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        Ok(SigningPartySignup {
            party_order: party_info.party_order,
            party_uuid: party_info.party_id.clone(),
            room_uuid: self.room_uuid.clone(),
            total_joined: self.member_info.len() as u16,
        })
    }

    pub fn close_signup_window(&mut self) {
        if self.is_full() {
            self.last_stage = "closed".to_string();
        }
    }
}