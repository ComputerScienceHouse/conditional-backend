use crate::app::AppState;
use actix_web::{
    body::{BoxBody, EitherBody},
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    web::Data,
    FromRequest, HttpMessage, HttpResponse,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use futures::{executor::block_on, future::LocalBoxFuture, lock::Mutex};
use lazy_static::lazy_static;
use log::{log, Level};
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

lazy_static! {
    static ref CSH_JWT_CACHE: Arc<Mutex<HashMap<String, PKey<Public>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    static ref INTRO_JWT_CACHE: Arc<Mutex<HashMap<String, PKey<Public>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenHeader {
    alg: String,
    kid: String,
    typ: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum User {
    CshUser {
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
        name: String,
        groups: Vec<String>,
        preferred_username: String,
        given_name: String,
        family_name: String,
        uuid: String,
        email: String,
    },
    IntroUser {
        exp: u32,
        iat: u32,
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
        name: String,
        preferred_username: String,
        given_name: String,
        family_name: String,
        email: String,
    },
}

impl FromRequest for User {
    type Error = actix_web::error::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let unauthorized = || {
            log!(
                Level::Info,
                "Unauthorized Request (from_request): {:?}",
                req
            );
            Box::pin(async {
                <Result<Self, Self::Error>>::Err(actix_web::error::ErrorUnauthorized(""))
            })
        };

        let h = match req.headers().get("Authorization").map(|h| {
            h.to_str()
                .unwrap_or("")
                .to_string()
                .trim_start_matches("Bearer ")
                .to_string()
        }) {
            Some(h) => h,
            None => return unauthorized(),
        };

        let (head, head_64, user, user_64, sig) = match get_token_pieces(h) {
            Ok(vals) => vals,
            Err(_) => return unauthorized(),
        };

        if verify_token(&head, &head_64, &user, &user_64, &sig) {
            Box::pin(async { Ok(user) })
        } else {
            unauthorized()
        }
    }
}

impl User {
    pub fn admin(&self) -> bool {
        match self {
            User::CshUser { groups, .. } => {
                groups.contains(&String::from("/eboard"))
                    || groups.contains(&String::from("/admins/rtp"))
            }
            User::IntroUser { .. } => false,
        }
    }

    pub fn eboard(&self) -> bool {
        match self {
            User::CshUser { groups, .. } => groups.contains(&String::from("/eboard")),
            User::IntroUser { .. } => false,
        }
    }

    pub fn evals(&self) -> bool {
        match self {
            User::CshUser { groups, .. } => groups.contains(&String::from("/eboard/evals")),
            User::IntroUser { .. } => false,
        }
    }
}

#[doc(hidden)]
pub struct CSHAuthService<S> {
    service: S,
    enabled: bool,
    access_level: AccessLevel,
}

fn get_token_pieces(token: String) -> Result<(TokenHeader, String, User, String, Vec<u8>)> {
    // log!(Level::Info, "{}", token);
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

#[allow(unused_must_use)]
fn verify_token(
    header: &TokenHeader,
    header_64: &String,
    payload: &User,
    payload_64: &String,
    key: &[u8],
) -> bool {
    let expiry = match payload {
        User::CshUser { exp, .. } | User::IntroUser { exp, .. } => exp,
    };
    if expiry < &(chrono::Utc::now().timestamp() as u32) {
        return false;
    }
    if header.alg != "RS256" {
        return false;
    }

    let (data_cache, cert_url) = match payload {
        User::CshUser { .. } => (
            CSH_JWT_CACHE.clone(),
            "https://sso.csh.rit.edu/auth/realms/csh/protocol/openid-connect/certs",
        ),
        User::IntroUser { .. } => (
            INTRO_JWT_CACHE.clone(),
            "https://sso.csh.rit.edu/auth/realms/intro/protocol/openid-connect/certs",
        ),
    };

    let cache = block_on(data_cache.lock());
    let pkey = match cache.get(header.kid.as_str()) {
        Some(x) => Some(x),
        None => {
            let data_cache = CSH_JWT_CACHE.clone();
            block_on(update_cache(data_cache.clone(), cert_url)).unwrap();
            cache.get(header.kid.as_str())
        }
    };

    let pkey = match pkey {
        Some(p) => p,
        None => return false,
    };

    let mut verifier = Verifier::new(MessageDigest::sha256(), pkey).unwrap();
    verifier.update(header_64.as_bytes());
    verifier.update(b".");
    verifier.update(payload_64.as_bytes());
    verifier.verify(key).unwrap_or(false)
}

impl<S, B> Service<ServiceRequest> for CSHAuthService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let _app_data: &Data<AppState> = req.app_data().unwrap();
        let authorized = |req: ServiceRequest| -> Self::Future {
            let future = self.service.call(req);
            return Box::pin(async move {
                let response = future.await?.map_into_left_body();
                Ok(response)
            });
        };

        if self.enabled {
            let unauthorized = |req: ServiceRequest| -> Self::Future {
                log!(Level::Info, "Unauthorized Request (call): {:?}", req);
                Box::pin(async move {
                    let chom = req
                        .into_response(HttpResponse::Unauthorized().finish().map_into_right_body());
                    Ok(chom)
                })
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

            // panic!("{:?}", token_payload);
            // log!(Level::Info, "{:?}", token_payload);
            let verified = verify_token(
                &token_header,
                &token_header_base64,
                &token_payload,
                &token_payload_base64,
                &token_signature,
            );

            if verified {
                req.extensions_mut().insert(token_payload.clone());
            } else {
                return unauthorized(req);
            }

            if self.access_level == AccessLevel::Admin && !token_payload.admin() {
                return unauthorized(req);
            }

            if self.access_level == AccessLevel::Eboard && !token_payload.eboard() {
                return unauthorized(req);
            }

            if self.access_level == AccessLevel::Evals && !token_payload.evals() {
                return unauthorized(req);
            }

            if self.access_level == AccessLevel::Freshman {
                return unauthorized(req);
            }

            return authorized(req);
        }
        return authorized(req);
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AccessLevel {
    Admin,
    Eboard,
    Evals,
    Member,
    Freshman,
    Public,
}

#[derive(Clone, Debug)]
pub struct CSHAuth {
    enabled: bool,
    access_level: AccessLevel,
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
            access_level: AccessLevel::Admin,
        }
    }

    pub fn eboard_only() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            access_level: AccessLevel::Eboard,
        }
    }

    pub fn evals_only() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            access_level: AccessLevel::Evals,
        }
    }

    pub fn enabled() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            access_level: AccessLevel::Member,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            access_level: AccessLevel::Public,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for CSHAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = CSHAuthService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CSHAuthService {
            service,
            enabled: self.enabled,
            access_level: self.access_level,
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

pub async fn update_cache(
    cache: Arc<Mutex<HashMap<String, PKey<Public>>>>,
    cert_url: &str,
) -> Result<()> {
    let cert_data: CertData = reqwest::get(cert_url).await?.json().await?;

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
