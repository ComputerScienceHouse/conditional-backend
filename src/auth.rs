use actix_web::{
    body::{BoxBody, EitherBody},
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    FromRequest, HttpMessage, HttpResponse,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use futures::future::LocalBoxFuture;
use lazy_static::lazy_static;
use log::{log, Level};
use openssl::{
    bn::BigNum,
    hash::MessageDigest,
    pkey::{PKey, Public},
    rsa::Rsa,
    sign::Verifier,
};
use reqwest::IntoUrl;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Pool, Postgres, Transaction};
use std::{
    collections::HashMap,
    future::{ready, Ready},
    rc::Rc,
    sync::Arc,
    task::{Context, Poll},
};
use std::{env, sync::Mutex};

use crate::api::lib::UserError;

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
pub struct UserBase {
    exp: u32,
    iat: u32,
    jti: String,
    aud: String,
    pub sub: String,
    typ: String,
    azp: String,
    auth_time: Option<String>,
    nonce: String,
    session_state: String,
    scope: String,
    sid: String,
    email_verified: bool,
    pub name: String,
    pub preferred_username: String,
    pub given_name: String,
    pub family_name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "iss")]
pub enum User {
    #[serde(rename = "https://sso.csh.rit.edu/auth/realms/csh")]
    CshUser {
        #[serde(flatten)]
        user: UserBase,
        groups: Vec<String>,
        uuid: String,
    },
    #[serde(rename = "https://sso.csh.rit.edu/auth/realms/intro")]
    IntroUser {
        #[serde(flatten)]
        user: UserBase,
    },
}

pub type UserInfo = User;

impl FromRequest for User {
    type Error = actix_web::error::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let user = match req.extensions().get::<User>().cloned() {
            Some(u) => Ok(u),
            None => {
                log!(Level::Info, "chom");
                Err(actix_web::error::ErrorUnauthorized(""))
            }
        };
        Box::pin(ready(user))
    }
}

impl User {
    pub fn admin(&self) -> bool {
        match self {
            User::CshUser { groups, .. } => {
                groups.contains(&String::from("eboard"))
                    || groups.contains(&String::from("admins"))
                    || groups.contains(&String::from("rtp"))
            }
            User::IntroUser { .. } => false,
        }
    }

    pub fn eboard(&self) -> bool {
        match self {
            User::CshUser { groups, .. } => {
                log!(Level::Info, "{:?}", groups);
                groups.contains(&String::from("eboard"))
            }
            User::IntroUser { .. } => false,
        }
    }

    pub fn evals(&self) -> bool {
        match self {
            User::CshUser { groups, .. } => groups.contains(&String::from("eboard-evaluations")),
            User::IntroUser { .. } => false,
        }
    }

    fn get_cache_info(&self) -> (Arc<Mutex<HashMap<String, PKey<Public>>>>, &str) {
        match self {
            User::CshUser { .. } => (
                CSH_JWT_CACHE.clone(),
                "https://sso.csh.rit.edu/auth/realms/csh/protocol/openid-connect/certs",
            ),
            User::IntroUser { .. } => (
                INTRO_JWT_CACHE.clone(),
                "https://sso.csh.rit.edu/auth/realms/intro/protocol/openid-connect/certs",
            ),
        }
    }

    pub fn get_uuid(&self) -> String {
        match self {
            User::CshUser { uuid, .. } => uuid.to_string(),
            User::IntroUser { user, .. } => user.sub.clone(),
        }
    }

    pub async fn get_uid(&self, db: &Pool<Postgres>) -> Result<i32, UserError> {
        let uuid = self.get_uuid();
        Ok(*query_as!(
            crate::schema::db::ID,
            r#"
                SELECT id
                FROM "user" u
                WHERE u.ipa_unique_id = $1::varchar
                OR u.intro_id = $1::varchar
            "#,
            uuid
        )
        .fetch_one(db)
        .await?)
    }
}

#[doc(hidden)]
pub struct CSHAuthService<S> {
    service: Rc<S>,
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

async fn authorize(token: String, access_level: AccessLevel) -> Result<User, actix_web::Error> {
    let (token_header, token_header_base64, token_payload, token_payload_base64, token_signature) =
        get_token_pieces(token).unwrap();

    match token_payload {
        User::CshUser { .. } => log!(Level::Info, "CSH user"),
        User::IntroUser { .. } => log!(Level::Info, "INTRO user"),
    }
    let unauthorized_err = Err(actix_web::error::ErrorUnauthorized(""));

    let verified = verify_token(
        &token_header,
        &token_header_base64,
        &token_payload,
        &token_payload_base64,
        &token_signature,
    )
    .await;

    if !verified {
        return unauthorized_err;
    }
    log!(Level::Info, "valid token");

    match token_payload {
        User::CshUser { .. } => {
            if access_level == AccessLevel::Admin && !token_payload.admin() {
                return unauthorized_err;
            }
            if access_level == AccessLevel::Eboard && !token_payload.eboard() {
                return unauthorized_err;
            }
            if access_level == AccessLevel::Evals && !token_payload.evals() {
                return unauthorized_err;
            }
            if access_level == AccessLevel::IntroOnly {
                return unauthorized_err;
            }
            return Ok(token_payload);
        }
        User::IntroUser { .. } => match access_level {
            AccessLevel::MemberAndIntro | AccessLevel::IntroOnly | AccessLevel::Public => {
                return Ok(token_payload);
            }
            _ => return unauthorized_err,
        },
    }
}

// #[allow(unused_must_use)]
async fn verify_token(
    header: &TokenHeader,
    header_64: &String,
    payload: &User,
    payload_64: &String,
    key: &[u8],
) -> bool {
    let expiry = match payload {
        User::CshUser { user, .. } | User::IntroUser { user, .. } => user.exp,
    };
    if expiry < chrono::Utc::now().timestamp() as u32 {
        log!(Level::Info, "expired token");
        return false;
    }
    if header.alg != "RS256" {
        log!(Level::Info, "wrong alg");
        return false;
    }
    log!(Level::Info, "unexpired and valid alg");

    let (data_cache, cert_url) = payload.get_cache_info();

    let mut cache = data_cache.lock().unwrap();
    let pkey = match cache.get(header.kid.as_str()).clone() {
        Some(x) => Some(x),
        None => {
            update_cache(&mut cache, cert_url).await.unwrap();
            cache.get(header.kid.as_str()).clone()
        }
    };
    log!(Level::Info, "got pkey {:?}", pkey);

    let pkey = match pkey {
        Some(p) => p,
        None => return false,
    };

    let mut verifier = Verifier::new(MessageDigest::sha256(), pkey).unwrap();
    verifier.update(header_64.as_bytes()).unwrap();
    verifier.update(b".").unwrap();
    verifier.update(payload_64.as_bytes()).unwrap();
    verifier.verify(key).unwrap_or(false)
}

impl<S, B> Service<ServiceRequest> for CSHAuthService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
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
        let srv = self.service.clone();
        let enabled = self.enabled.clone();
        let access_level = self.access_level.clone();

        Box::pin(async move {
            if enabled {
                let token = match req.headers().get("Authorization").map(|x| x.to_str()) {
                    Some(Ok(x)) => x.trim_start_matches("Bearer ").to_string(),
                    _ => {
                        log!(Level::Info, "unauthed a");
                        let response = req.into_response(
                            HttpResponse::Unauthorized().finish().map_into_right_body(),
                        );
                        return Ok(response);
                    }
                };

                log!(Level::Info, "checking auth");
                match authorize(token, access_level).await {
                    Ok(user) => {
                        log!(Level::Info, "inserting user");
                        req.extensions_mut().insert::<User>(user);
                    }
                    Err(_) => {
                        log!(Level::Info, "unauthed b");
                        let response = req.into_response(
                            HttpResponse::Unauthorized().finish().map_into_right_body(),
                        );
                        return Ok(response);
                    }
                }
            }
            let future = srv.call(req);
            let response = future.await?.map_into_left_body();
            Ok(response)
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AccessLevel {
    Admin,
    Eboard,
    Evals,
    MemberOnly,
    MemberAndIntro,
    IntroOnly,
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

    pub fn member_only() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            access_level: AccessLevel::MemberOnly,
        }
    }

    pub fn member_and_intro() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            access_level: AccessLevel::MemberAndIntro,
        }
    }
    pub fn enabled() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            access_level: AccessLevel::MemberAndIntro,
        }
    }

    pub fn intro_only() -> Self {
        Self {
            enabled: *SECURITY_ENABLED,
            access_level: AccessLevel::IntroOnly,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            access_level: AccessLevel::Public,
        }
    }
}

impl<S: 'static, B> Transform<S, ServiceRequest> for CSHAuth
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
            service: Rc::new(service),
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

pub async fn update_cache<T>(cache: &mut HashMap<String, PKey<Public>>, cert_url: T) -> Result<()>
where
    T: IntoUrl,
{
    // let (cache, cert_url) = user.get_cache_info();
    log!(Level::Info, "Update cache start");
    let cert_data: CertData = reqwest::get(cert_url).await?.json().await?;
    log!(Level::Info, "Update cache finished request");
    // let mut cache = cache.lock().unwrap();
    log!(Level::Info, "got cache");
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
    log!(Level::Info, "Finished updating cache");
    Ok(())
}
