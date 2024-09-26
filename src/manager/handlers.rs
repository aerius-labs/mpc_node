use crate::common::{
    Entry, Index, KeyGenParams, ManagerError, PartySignup, PartySignupRequestBody, SignerResult, SigningPartySignup, SigningRoom
};
use crate::error::TssError;
use crate::manager::ManagerService;
use rocket::http::Status;
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::post;
use rocket::{Request, State};
use std::sync::Arc;

#[post("/get", format = "json", data = "<request>")]
pub async fn get(
    manager: &State<Arc<ManagerService>>,
    request: Json<Index>,
) -> Json<Result<Entry, ManagerError>> {
    let index: Index = request.into_inner();
    let signing_rooms = manager.signing_rooms.read().await;
    match signing_rooms.get(&index.key) {
        Some(value) => {
            let entry = Entry {
                key: index.key.clone(),
                value: value.clone().to_string(),
            };
            Json(Ok(entry))
        }
        None => Json(Err(ManagerError {
            error: "Key not found".to_string(),
        })),
    }
}

#[post("/set", format = "json", data = "<request>")]
pub async fn set(
    manager: &State<Arc<ManagerService>>,
    request: Json<Entry>,
) -> Json<Result<(), ManagerError>> {
    let entry: Entry = request.into_inner();
    let mut signing_rooms = manager.signing_rooms.write().await;
    signing_rooms.insert(entry.key.clone(), entry.value.clone());
    Json(Ok(()))
}

#[post("/signupkeygen", format = "json", data = "<request>")]
pub async fn signup_keygen(
    manager: &State<Arc<ManagerService>>,
    request: Json<KeyGenParams>,
) -> Json<Result<PartySignup, ManagerError>> {
    let parties = request.parties;
    let key = "signup-keygen".to_string();
    let mut hm = manager.signing_rooms.write().await;

    let client_signup = match hm.get(&key) {
        Some(o) => serde_json::from_str(o).unwrap(),
        None => PartySignup {
            number: 0,
            uuid: uuid::Uuid::new_v4().to_string(),
        },
    };

    let party_signup = {
        if client_signup.number < parties {
            PartySignup {
                number: client_signup.number + 1,
                uuid: client_signup.uuid,
            }
        } else {
            PartySignup {
                number: 1,
                uuid: uuid::Uuid::new_v4().to_string(),
            }
        }
    };

    hm.insert(key, serde_json::to_string(&party_signup).unwrap());
    Json(Ok(party_signup))
}

#[post("/signupsign", format = "json", data = "<request>")]
pub async fn signup_sign(
    manager: &State<Arc<ManagerService>>,
    request: Json<PartySignupRequestBody>,
) -> Json<Result<SigningPartySignup, ManagerError>> {
    let req = request.into_inner();
    let threshold = req.threshold;
    let room_id = req.room_id.clone();
    let party_uuid = req.party_uuid.clone();
    let new_signup_request = party_uuid.is_empty();
    let party_number = req.party_number;
    let mut key = "signup-sign-".to_owned();
    key.push_str(&room_id);

    let mut signing_rooms = manager.signing_rooms.write().await;

    let mut signing_room = match signing_rooms.get(&key) {
        Some(room) => serde_json::from_str(room).unwrap(),
        None => SigningRoom::new(room_id.clone(), threshold + 1),
    };

    if signing_room.last_stage != "signup" {
        if signing_room.has_member(party_number, party_uuid.clone()) {
            return Json(Ok(signing_room.get_signup_info(party_number)));
        }

        if signing_room.are_all_members_inactive() {
            let debug = serde_json::json!({
                "message": "All parties have been inactive. Renewed the room.",
                "room_id": room_id,
                "fragment.index": party_number,
            });
            println!("{}", serde_json::to_string_pretty(&debug).unwrap());
            signing_room = SigningRoom::new(room_id, threshold + 1);
        } else {
            return Json(Err(ManagerError {
                error: "Room signup phase is terminated".to_string(),
            }));
        }
    }

    if signing_room.is_full() && signing_room.are_all_members_active() && new_signup_request {
        return Json(Err(ManagerError {
            error: "Room is full, all members active".to_string(),
        }));
    }

    let party_signup = if !new_signup_request {
        if !signing_room.has_member(party_number, party_uuid.clone()) {
            return Json(Err(ManagerError {
                error: "No party found with the given uuid, probably replaced due to timeout"
                    .to_string(),
            }));
        }
        signing_room.update_ping(party_number)
    } else if signing_room.member_info.contains_key(&party_number) {
        if signing_room.is_member_active(party_number) {
            return Json(Err(ManagerError {
                error: "Received a re-signup request for an active party. Request ignored"
                    .to_string(),
            }));
        }
        println!(
            "Received a re-signup request for a timed-out party {:?}, thus UUID is renewed",
            party_number
        );
        signing_room.replace_party(party_number)
    } else {
        signing_room.add_party(party_number)
    };

    signing_rooms.insert(key.clone(), serde_json::to_string(&signing_room).unwrap());
    Json(Ok(party_signup))
}

#[post("/update_signing_result", format = "json", data = "<result>")]
pub async fn update_signing_result(
    manager: &State<Arc<ManagerService>>,
    result: Json<SignerResult>,
) -> Json<Result<(), ManagerError>> {
    match manager.update_signing_result(result.into_inner()).await {
        Ok(_) => {}
        Err(e) => {
            return Json(Err(ManagerError {
                error: e.to_string(),
            }));
        }
    };
    Json(Ok(()))
}

impl<'r> Responder<'r, 'static> for TssError {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        Err(Status::InternalServerError)
    }
}
