use rocket::{Request, State};
use rocket::serde::json::Json;
use crate::common::{Entry, Index, ManagerError, PartySignupRequestBody, SigningPartySignup, SigningResult};
use crate::manager::ManagerService;
use crate::error::TssError;
use rocket::{post, get};
use rocket::http::Status;
use rocket::response::Responder;

#[post("/get", format = "json", data = "<request>")]
pub async fn get(manager: &State<ManagerService>, request: Json<Index>) -> Json<Result<Entry, ManagerError>> {
    let index: Index = request.into_inner();
    match manager.storage.get(&index.key).await {
        Ok(Some(value)) => Json(Ok(Entry { key: index.key, value })),
        Ok(None) => Json(Err(ManagerError { error: format!("Key not found: {}", index.key) })),
        Err(e) => Json(Err(ManagerError { error: format!("Database error: {}", e) })),
    }
}

#[post("/set", format = "json", data = "<request>")]
pub async fn set(manager: &State<ManagerService>, request: Json<Entry>) -> Json<Result<(), ManagerError>> {
    let entry: Entry = request.into_inner();
    match manager.storage.set(&entry.key, &entry.value).await {
        Ok(_) => Json(Ok(())),
        Err(e) => Json(Err(ManagerError { error: format!("Database error: {}", e) })),
    }
}

#[post("/signupsign", format = "json", data = "<request>")]
pub async fn signup_sign(
    manager: &State<ManagerService>,
    request: Json<PartySignupRequestBody>,
) -> Json<Result<SigningPartySignup, ManagerError>> {
    let req = request.into_inner();
    let mut signing_rooms = manager.signing_rooms.write().await;

    let signing_room = match signing_rooms.get_mut(&req.room_id) {
        Some(room) => room,
        None => return Json(Err(ManagerError { error: "Room not found".to_string() })),
    };

    match signing_room.add_party(req.party_number, req.party_uuid) {
        Ok(party_signup) => Json(Ok(party_signup)),
        Err(e) => Json(Err(ManagerError { error: e })),
    }
}

#[get("/signing_result/<request_id>")]
pub async fn get_signing_result(manager: &State<ManagerService>, request_id: String) -> Result<Json<Option<SigningResult>>, TssError> {
    let result = manager.get_signing_result(&request_id).await?;
    Ok(Json(result))
}

#[post("/update_signing_result", format = "json", data = "<result>")]
pub async fn update_signing_result(manager: &State<ManagerService>, result: Json<SigningResult>) -> Result<Json<()>, TssError> {
    manager.update_signing_result(result.into_inner()).await?;
    Ok(Json(()))
}

impl<'r> Responder<'r, 'static> for TssError {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        Err(Status::InternalServerError)
    }
}