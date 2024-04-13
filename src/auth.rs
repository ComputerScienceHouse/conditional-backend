use async_mutex::Mutex;
use base64::{engine::general_purpose, Engine as _};
use derive_more::{Display, Error};
use jwt::algorithm::openssl::PKeyWithDigest;
use jwt::{Header, Token, Unverified, VerifyWithKey};
use lazy_static::lazy_static;
use log::{error, info, trace};
use openssl::{
    bn::BigNum,
    hash::MessageDigest,
    pkey::{PKey, Public},
    rsa::Rsa,
};
use reqwest::IntoUrl;
use serde::{Deserialize, Serialize};
use sqlx::{query_as, Pool, Postgres};
use std::{collections::HashMap, sync::Arc};

use crate::auth_service::AccessLevel;

#[derive(Debug, Display, Error)]
pub enum AuthError {
    #[display(fmt = "You are not authorized to view this resource.")]
    Unauthorized,
    #[display(fmt = "Failed to authenticate.")]
    AuthenticationError,
}

impl From<jwt::Error> for AuthError {
    fn from(value: jwt::Error) -> Self {
        error!("{}", value.to_string());
        AuthError::AuthenticationError
    }
}

impl From<reqwest::Error> for AuthError {
    fn from(value: reqwest::Error) -> Self {
        error!("{}", value.to_string());
        AuthError::AuthenticationError
    }
}

impl From<base64::DecodeError> for AuthError {
    fn from(value: base64::DecodeError) -> Self {
        error!("{}", value.to_string());
        AuthError::AuthenticationError
    }
}

impl From<openssl::error::ErrorStack> for AuthError {
    fn from(value: openssl::error::ErrorStack) -> Self {
        error!("{}", value.to_string());
        AuthError::AuthenticationError
    }
}

type JwtCache = Arc<Mutex<HashMap<String, PKey<Public>>>>;

lazy_static! {
    static ref CSH_JWT_CACHE: JwtCache = Arc::new(Mutex::new(HashMap::new()));
    static ref INTRO_JWT_CACHE: JwtCache = Arc::new(Mutex::new(HashMap::new()));
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
                info!("{:?}", groups);
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

    fn get_cache_info(&self) -> (JwtCache, &str) {
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

    pub async fn get_uid(&self, db: &Pool<Postgres>) -> Result<i32, sqlx::Error> {
        let uuid = self.get_uuid();
        query_as!(
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
        .await
        .map(|r| r.id)
    }
}

pub async fn authorize(token: String, access_level: AccessLevel) -> Result<User, AuthError> {
    let token: Token<Header, User, Unverified<'_>> = Token::parse_unverified(token.as_str())?;
    let user = token.claims().to_owned();
    let header = token.header();

    match user {
        User::CshUser { .. } => info!("CSH user"),
        User::IntroUser { .. } => info!("INTRO user"),
    }

    let (data_cache, cert_url) = user.get_cache_info();
    let mut cache = data_cache.lock().await;
    let key_id = header
        .key_id
        .clone()
        .ok_or(AuthError::AuthenticationError)?;
    let pkey = match cache.get(&key_id) {
        Some(x) => x,
        None => {
            update_cache(&mut cache, cert_url).await.unwrap();
            cache.get(&key_id).ok_or(AuthError::AuthenticationError)?
        }
    };
    let key = PKeyWithDigest {
        digest: MessageDigest::sha256(),
        key: pkey.to_owned(),
    };
    token.verify_with_key(&key)?;

    let user = user.to_owned();
    match user {
        User::CshUser { .. } => {
            if access_level == AccessLevel::Admin && !user.admin() {
                return Err(AuthError::Unauthorized);
            }
            if access_level == AccessLevel::Eboard && !user.eboard() {
                return Err(AuthError::Unauthorized);
            }
            if access_level == AccessLevel::Evals && !user.evals() {
                return Err(AuthError::Unauthorized);
            }
            if access_level == AccessLevel::IntroOnly {
                return Err(AuthError::Unauthorized);
            }
            Ok(user)
        }
        User::IntroUser { .. } => match access_level {
            AccessLevel::MemberAndIntro | AccessLevel::IntroOnly | AccessLevel::Public => Ok(user),
            _ => Err(AuthError::Unauthorized),
        },
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

async fn update_cache<T>(
    cache: &mut HashMap<String, PKey<Public>>,
    cert_url: T,
) -> Result<(), AuthError>
where
    T: IntoUrl,
{
    trace!("Update cache start");
    let cert_data: CertData = reqwest::get(cert_url).await?.json().await?;
    trace!("Got cache, processing keys");
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
    trace!("Finished updating cache");
    Ok(())
}
