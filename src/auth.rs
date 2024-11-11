use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::hyper::request;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Check IP whitelist for signer endpoints
        if is_signer_endpoint(request.uri().path().to_string().as_str()) {
            if let Some(client_ip) = request.client_ip() {
                if !is_ip_whitelisted(client_ip) {
                    return Outcome::Error((Status::Unauthorized, AuthError::IpNotAllowed));
                }
            }
        }

        // Get and validate JWT token
        let token = request
            .headers()
            .get_one("Authorization")
            .map(|value| value.replace("Bearer ", ""));

        match token {
            Some(token) => match validate_token(&token) {
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
                Err(e) => Outcome::Error((Status::Unauthorized, AuthError::Invalid)),
            },
            None => Outcome::Error((Status::Unauthorized, AuthError::Missing)),
        }
    }
}

fn is_ip_whitelisted(ip: IpAddr) -> bool {
    if let Ok(allowed_ips) = std::env::var("ALLOWED_SIGNER_IPS") {
        allowed_ips
            .split(',')
            .any(|allowed_ip| allowed_ip.trim().parse::<IpAddr>().ok() == Some(ip))
    } else {
        false // If no IPs are configured, deny all requests
    }
}

fn is_signer_endpoint(path: &str) -> bool {
    matches!(
        path,
        "/signupkeygen" | "/signupsign" | "/set" | "/get" | "/update_signing_result"
    )
}

fn has_permission_for_endpoint(role: &Role, path: &str) -> bool {
    match (role, path) {
        // Public endpoints
        (Role::Public, "/sign") => true,
        (Role::Public, path) if path.starts_with("/signing_result/") => true,

        // Signer endpoints
        (Role::Signer, "/signupkeygen") => true,
        (Role::Signer, "/signupsign") => true,
        (Role::Signer, "/set") => true,
        (Role::Signer, "/get") => true,
        (Role::Signer, "/update_signing_result") => true,

        // Admin endpoints
        (Role::Admin, _) => true,

        _ => false,
    }
}

pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "default-secret-change-me-in-production".to_string());

    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;

    Ok(token_data.claims)
}

pub fn create_token(user_id: &str, role: Role) -> Result<String, TssError> {
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "default-secret-change-me-in-production".to_string());

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
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|e| TssError::JWTError(e.to_string()))
}

fn decode_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret("your-secret-key".as_ref()),
        &validation,
    )?;
    Ok(token_data.claims)
}

// Helper function to generate tokens for testing/developement
#[cfg(debug_assertions)]
pub fn generate_test_token(role: Role) -> String {
    todo!()
}
