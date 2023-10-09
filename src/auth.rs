use crate::app::AppState;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    web::Data,
    HttpMessage, HttpResponse,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use futures::{executor::block_on, future::LocalBoxFuture, lock::Mutex};
use lazy_static::lazy_static;
use openssl::{
    bn::BigNum,
    hash::MessageDigest,
    pkey::{PKey, Public},
    rsa::Rsa,
    sign::Verifier,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::{
    collections::HashMap,
    future::{ready, Ready},
    sync::Arc,
    task::{Context, Poll},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenHeader {
    alg: String,
    kid: String,
    typ: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    exp: u32,
    iat: u32,
    auth_time: u32,
    jti: String,
    iss: String,
    aud: String,
    sub: String,
    typ: String,
    azp: String,
    nonce: String,
    session_state: String,
    scope: String,
    sid: String,
    email_verified: bool,
    pub name: String,
    pub groups: Vec<String>,
    pub preferred_username: String,
    pub given_name: String,
    pub family_name: String,
    pub email: String,
}

impl User {
    fn admin(&self) -> bool {
        self.groups.contains(&String::from("/eboard"))
            || self.groups.contains(&String::from("/admins/rtp"))
    }

    fn eboard(&self) -> bool {
        self.groups.contains(&String::from("/eboard"))
    }

    fn evals(&self) -> bool {
        self.groups.contains(&String::from("/eboard/evals"))
    }
}

#[doc(hidden)]
pub struct CSHAuthService<S> {
    service: S,
    enabled: bool,
    admin_only: bool,
    eboard_only: bool,
    evals_only: bool,
}

fn get_token_pieces(token: String) -> Result<(TokenHeader, String, User, String, Vec<u8>)> {
    let mut it = token.split('.');
    let token_header_base64 = it.next().ok_or(anyhow!("!header"))?;
    let token_header = general_purpose::URL_SAFE_NO_PAD.decode(token_header_base64)?;
    let token_header: TokenHeader = serde_json::from_slice(&token_header)?;
    let token_payload_base64 = it.next().ok_or(anyhow!("!body"))?;
    let token_payload = general_purpose::URL_SAFE_NO_PAD.decode(token_payload_base64)?;
    let token_payload: User = serde_json::from_slice(&token_payload)?;
    let token_signature = it.next().ok_or(anyhow!("signature"))?;
    let token_signature = general_purpose::URL_SAFE_NO_PAD.decode(token_signature)?;
    Ok((
        token_header,
        token_header_base64.to_owned(),
        token_payload,
        token_payload_base64.to_owned(),
        token_signature.into(),
    ))
}

impl<S> Service<ServiceRequest> for CSHAuthService<S>
where
    S: Service<
        ServiceRequest,
        Response = ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
{
    type Response = ServiceResponse<actix_web::body::BoxBody>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    #[allow(unused_must_use)]
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let app_data: &Data<AppState> = req.app_data().unwrap();
        if self.enabled {
            let unauthorized = |req: ServiceRequest| -> Self::Future {
                Box::pin(async { Ok(req.into_response(HttpResponse::Unauthorized().finish())) })
            };

            let token = match req.headers().get("Authorization").map(|x| x.to_str()) {
                Some(Ok(x)) => x.trim_start_matches("Bearer ").to_string(),
                _ => return unauthorized(req),
            };

            let (
                token_header,
                token_header_base64,
                token_payload,
                token_payload_base64,
                token_signature,
            ) = get_token_pieces(token).unwrap();
            if token_payload.exp < (chrono::Utc::now().timestamp() as u32) {
                return unauthorized(req);
            }
            if token_header.alg != "RS256" {
                return unauthorized(req);
            }

            let data_cache = &app_data.clone().into_inner().jwt_cache;
            let cache = block_on(data_cache.lock());
            let pkey = match cache.get(token_header.kid.as_str()) {
                Some(x) => Some(x),
                None => {
                    let data_cache = &app_data.clone().into_inner().jwt_cache;
                    block_on(update_cache(data_cache.clone())).unwrap();
                    cache.get(token_header.kid.as_str())
                }
            };

            if pkey.is_none() {
                return unauthorized(req);
            }

            let mut verifier = Verifier::new(MessageDigest::sha256(), &pkey.unwrap()).unwrap();
            verifier.update(token_header_base64.as_bytes());
            verifier.update(b".");
            verifier.update(token_payload_base64.as_bytes());
            let verified = verifier.verify(&token_signature).unwrap();
            if verified {
                req.extensions_mut().insert(token_payload.clone());
            } else {
                return unauthorized(req);
            }

            if self.admin_only && !token_payload.admin() {
                return unauthorized(req);
            }

            if self.eboard_only && !token_payload.eboard() {
                return unauthorized(req);
            }

            if self.evals_only && !token_payload.evals() {
                return unauthorized(req);
            }

            let future = self.service.call(req);
            return Box::pin(async move {
                let response = future.await?;
                Ok(response)
            });
        }
        let future = self.service.call(req);
        Box::pin(async move {
            let response = future.await?;
            Ok(response)
        })
    }
}

#[derive(Clone, Debug)]
pub struct CSHAuth {
    enabled: bool,
    admin: bool,
    eboard: bool,
    evals: bool,
}

lazy_static! {
    static ref SECURITY_ENABLED: bool = env::var("SECURITY_ENABLED")
        .map(|x| x.parse::<bool>().unwrap_or(true))
        .unwrap_or(true);
}

impl CSHAuth {
    pub fn admin_only() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            admin: true,
            eboard: false,
            evals: false,
        }
    }

    pub fn eboard_only() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            admin: false,
            eboard: true,
            evals: false,
        }
    }

    pub fn evals_only() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            admin: false,
            eboard: false,
            evals: true,
        }
    }

    pub fn enabled() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            admin: false,
            eboard: false,
            evals: false,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            admin: false,
            eboard: false,
            evals: false,
        }
    }
}

impl<S> Transform<S, ServiceRequest> for CSHAuth
where
    S: Service<
        ServiceRequest,
        Response = ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
{
    type Response = ServiceResponse<actix_web::body::BoxBody>;
    type Error = actix_web::Error;
    type Transform = CSHAuthService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CSHAuthService {
            service,
            enabled: self.enabled,
            admin_only: self.admin,
            eboard_only: self.eboard,
            evals_only: self.evals,
        }))
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CertKey {
    kid: String,
    kty: String,
    alg: String,
    r#use: String,
    n: String,
    e: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CertData {
    keys: Vec<CertKey>,
}

pub async fn update_cache(cache: Arc<Mutex<HashMap<String, PKey<Public>>>>) -> Result<()> {
    let cert_data: CertData =
        reqwest::get("https://sso.csh.rit.edu/auth/realms/csh/protocol/openid-connect/certs")
            .await?
            .json()
            .await?;

    let mut cache = cache.lock().await;
    for key in cert_data.keys {
        if cache.contains_key(key.kid.as_str()) {
            continue;
        }
        let n: Vec<String> = general_purpose::URL_SAFE_NO_PAD
            .decode(key.n.as_bytes())?
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect();
        let e: Vec<String> = general_purpose::URL_SAFE_NO_PAD
            .decode(key.e.as_bytes())?
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect();
        let n = BigNum::from_hex_str(&n.join(""))?;
        let e = BigNum::from_hex_str(&e.join(""))?;
        let rsa = Rsa::from_public_components(n, e)?;
        cache.insert(key.kid, PKey::from_rsa(rsa)?);
    }
    Ok(())
}
