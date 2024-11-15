use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{Request, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

use crate::config::Settings;
use crate::error::TssError;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Role {
    Public, // For general API access
    Signer, // For signer-specific endpoints
    Admin,  // For administrative functions
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    pub exp: usize,  // Expiration time
    pub role: Role,  // User role
    pub iat: usize,  // Issued at
}

#[derive(Debug)]
pub enum AuthError {
    Missing,
    Invalid,
    Expired,
    WrongRole,
    IpNotAllowed,
}

pub struct AuthenticatedUser {
    pub user_id: String,
    pub role: Role,
}

// New struct for IP-based authentication
pub struct SignerAuth;

// New struct for IP-based authentication
#[rocket::async_trait]
impl<'r> FromRequest<'r> for SignerAuth {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let settings = request
            .guard::<&State<Arc<Settings>>>()
            .await
            .expect("Settings not found in request state");

        if let Some(client_ip) = request.client_ip() {
            debug!("Signer API request from IP: {}", client_ip);
            if settings.is_ip_whitelisted(client_ip) {
                Outcome::Success(SignerAuth)
            } else {
                warn!(
                    "Unauthorized signer API access attempt from IP: {}",
                    client_ip
                );
                Outcome::Error((Status::Unauthorized, AuthError::IpNotAllowed))
            }
        } else {
            warn!("No client IP found in signer API request");
            Outcome::Error((Status::Unauthorized, AuthError::IpNotAllowed))
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let settings = request
            .guard::<&State<Arc<Settings>>>()
            .await
            .expect("Settings not found in request state");

        // Get and validate JWT token
        let token = request
            .headers()
            .get_one("Authorization")
            .map(|value| value.replace("Bearer ", ""));

        match token {
            Some(token) => match validate_token(&token, &settings.inner().security.jwt_secret) {
                Ok(claims) => {
                    // Verify role permissions for endpoint
                    if !has_permission_for_endpoint(
                        &claims.role,
                        request.uri().path().to_string().as_str(),
                    ) {
                        return Outcome::Error((Status::Forbidden, AuthError::WrongRole));
                    }

                    Outcome::Success(AuthenticatedUser {
                        user_id: claims.sub,
                        role: claims.role,
                    })
                }
                Err(_) => Outcome::Error((Status::Unauthorized, AuthError::Invalid)),
            },
            None => Outcome::Error((Status::Unauthorized, AuthError::Missing)),
        }
    }
}

fn has_permission_for_endpoint(role: &Role, path: &str) -> bool {
    match (role, path) {
        // Public endpoints
        (Role::Public, "/sign") => true,
        (Role::Public, path) if path.starts_with("/signing_result/") => true,

        // Signer endpoints
        (Role::Signer, _) => false,

        // Admin endpoints
        (Role::Admin, _) => true,

        _ => false,
    }
}

pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;

    Ok(token_data.claims)
}

pub fn create_token(user_id: &str, role: Role, settings: &Settings) -> Result<String, TssError> {
    match role {
        Role::Signer => {
            return Err(TssError::AuthError(
                "Cannot create token for Signer role".into(),
            ))
        }
        _ => (),
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: now + 3600, // Expires in 1 hour
        role,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.security.jwt_secret.as_ref()),
    )
    .map_err(|e| TssError::JWTError(e.to_string()))
}
