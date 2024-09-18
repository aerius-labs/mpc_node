// use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
// use serde::{Serialize, Deserialize};
// use rocket::http::Status;
// use rocket::request::{FromRequest, Outcome};
// use rocket::Request;
// use std::time::{SystemTime, UNIX_EPOCH};
// // use rocket::outcome::Outcome;
//
// #[derive(Debug, Serialize, Deserialize)]
// struct Claims {
//     sub: String,
//     exp: usize,
// }
//
// pub struct AuthenticatedUser {
//     pub user_id: String,
// }
//
// #[derive(Debug)]
// pub enum AuthError {
//     Missing,
//     Invalid,
// }
//
// #[rocket::async_trait]
// impl<'r> FromRequest<'r> for AuthenticatedUser {
//     type Error = AuthError;
//
//     async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
//         let token = request.headers().get_one("Authorization");
//         match token {
//             Some(token) if token.starts_with("Bearer ") => {
//                 let token = token[7..].to_string();
//                 if let Ok(claims) = decode_token(&token) {
//                     Outcome::Success(AuthenticatedUser { user_id: claims.sub })
//                 } else {
//                     Outcome::Error((Status::Unauthorized, AuthError::Invalid))
//                 }
//             }
//             _ => Outcome::Error((Status::Unauthorized, AuthError::Missing)),
//         }
//     }
// }
//
// pub fn create_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
//     let expiration = SystemTime::now()
//         .duration_since(UNIX_EPOCH)
//         .unwrap()
//         .as_secs() + 3600; // Token valid for 1 hour
//
//     let claims = Claims {
//         sub: user_id.to_owned(),
//         exp: expiration as usize,
//     };
//
//     encode(&Header::default(), &claims, &EncodingKey::from_secret("your-secret-key".as_ref()))
// }
//
// fn decode_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
//     let validation = Validation::default();
//     let token_data = decode::<Claims>(token, &DecodingKey::from_secret("your-secret-key".as_ref()), &validation)?;
//     Ok(token_data.claims)
// }

use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};
use rocket::http::Status;
use rocket::request::{Outcome, FromRequest};
use rocket::Request;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub struct AuthenticatedUser {
    // pub user_id: String,
}

#[derive(Debug)]
pub enum AuthError {
    Missing,
    Invalid,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    // type Error = AuthError;

    // async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
    //     // let token = request.headers().get_one("Authorization");
    //     // match token {
    //     //     Some(token) if token.starts_with("Bearer ") => {
    //     //         let token = token[7..].to_string();
    //     //         if let Ok(claims) = decode_token(&token) {
    //     //             Outcome::Success(AuthenticatedUser { user_id: claims.sub })
    //     //         } else {
    //     //             Outcome::Error((Status::Unauthorized, AuthError::Invalid))
    //     //         }
    //     //     }
    //     //     _ => Outcome::Error((Status::Unauthorized, AuthError::Missing)),
    //     // }
    //     match request.headers().get_one("Authorization") {
    //         Some(_) => Outcome::Success(AuthenticatedUser { user_id: "claims.sub".to_string()}),
    //         None =>Outcome::Error((Status::Unauthorized, AuthError::Invalid))
    //     }
    // }

    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // For now, we'll just check if an "Authorization" header is present
        // In a real application, you'd verify the token here
        match request.headers().get_one("Authorization") {
            Some(_) => Outcome::Success(AuthenticatedUser {}),
            None => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}

pub fn create_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() + 3600; // Token valid for 1 hour

    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration as usize,
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret("your-secret-key".as_ref()))
}

fn decode_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(token, &DecodingKey::from_secret("your-secret-key".as_ref()), &validation)?;
    Ok(token_data.claims)
}