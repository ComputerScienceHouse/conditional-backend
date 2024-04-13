use actix_web::{
    body::{BoxBody, EitherBody},
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    FromRequest, HttpMessage, HttpResponse,
};
use futures::future::LocalBoxFuture;
use jwt::{Header, Token, Unverified};
use lazy_static::lazy_static;
use log::{debug, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::env;
use std::{
    future::{ready, Ready},
    rc::Rc,
    task::{Context, Poll},
};

use crate::auth::{authorize, User};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenHeader {
    alg: String,
    kid: String,
    typ: String,
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
                // TODO: correct error message here
                info!("no user was put in the extension");
                Err(actix_web::error::ErrorUnauthorized(""))
            }
        };
        Box::pin(ready(user))
    }
}

#[doc(hidden)]
pub struct CSHAuthService<S> {
    service: Rc<S>,
    enabled: bool,
    access_level: AccessLevel,
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
        let enabled = self.enabled;
        let access_level = self.access_level;

        Box::pin(async move {
            if enabled {
                let token = match req.headers().get("Authorization").map(|x| x.to_str()) {
                    Some(Ok(x)) => x.trim_start_matches("Bearer ").to_string(),
                    _ => {
                        debug!("unauthed a");
                        let response = req.into_response(
                            HttpResponse::Unauthorized().finish().map_into_right_body(),
                        );
                        return Ok(response);
                    }
                };

                trace!("checking auth");
                match authorize(token, access_level).await {
                    Ok(user) => {
                        debug!("inserting user");
                        req.extensions_mut().insert::<User>(user);
                    }
                    Err(_) => {
                        debug!("unauthed b");
                        let response = req.into_response(
                            HttpResponse::Forbidden().finish().map_into_right_body(),
                        );
                        return Ok(response);
                    }
                }
            } else {
                // Inject a user into the routes
                if let Some(user_jwt) = (*USER_JWT).clone() {
                    warn!("Injecting user from env vars - should only be used for testing");
                    let token: Token<Header, User, Unverified<'_>> =
                        match Token::parse_unverified(user_jwt.as_str()) {
                            Ok(t) => t,
                            Err(_) => {
                                debug!("unauthed b");
                                let response = req.into_response(
                                    HttpResponse::Unauthorized().finish().map_into_right_body(),
                                );
                                return Ok(response);
                            }
                        };
                    let user = token.claims().to_owned();
                    req.extensions_mut().insert::<User>(user);
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
    static ref USER_JWT: Option<String> = env::var("USER_JWT").ok();
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
