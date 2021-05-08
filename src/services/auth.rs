use crate::error::{bail, Result};
use crate::services::RequestWrapper;
use crate::util::ResponseBuilderExt;
use data_encoding::BASE64;
use futures::{future, prelude::*};
use headers::authorization::Bearer;
use headers::{Authorization, ContentLength, ContentType, Cookie, HeaderMapExt};
use hyper::header::SET_COOKIE;
use hyper::{Body, Method, Response};
use ring::rand::{SecureRandom, SystemRandom};
use ring::{
    digest::{digest, SHA256},
    hmac,
};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{borrow, time::Duration};
use thiserror::Error;
use tokio::time::sleep;
use url::form_urlencoded;

use super::resp;

pub enum AuthResult<T> {
    Authenticated {
        credentials: T,
        request: RequestWrapper,
    },
    Rejected(Response<Body>),
    LoggedIn(Response<Body>),
}
type AuthFuture<T> = Pin<Box<dyn Future<Output = Result<AuthResult<T>>> + Send>>;

pub trait Authenticator: Send + Sync {
    type Credentials;
    fn authenticate(&self, req: RequestWrapper) -> AuthFuture<Self::Credentials>;
}

#[derive(Clone, Debug)]
struct Secrets {
    shared_secret: String,
    server_secret: Vec<u8>,
    token_validity_hours: u32,
}

#[derive(Clone)]
pub struct SharedSecretAuthenticator {
    secrets: Arc<Secrets>,
}

impl SharedSecretAuthenticator {
    pub fn new(shared_secret: String, server_secret: Vec<u8>, token_validity_hours: u32) -> Self {
        SharedSecretAuthenticator {
            secrets: Arc::new(Secrets {
                shared_secret,
                server_secret,
                token_validity_hours,
            }),
        }
    }
}

const COOKIE_NAME: &str = "audioserve_token";

impl Authenticator for SharedSecretAuthenticator {
    type Credentials = ();
    fn authenticate(&self, mut req: RequestWrapper) -> AuthFuture<()> {
        fn deny() -> AuthResult<()> {
            AuthResult::Rejected(resp::deny())
        }
        // this is part where client can authenticate itself and get token
        if req.method() == Method::POST && req.path() == "/authenticate" {
            debug!("Authentication request");
            let auth = self.secrets.clone();
            return Box::pin(async move {
                match req.body_bytes().await {
                    Err(e) => bail!(e),
                    Ok(b) => {
                        let params = form_urlencoded::parse(b.as_ref())
                            .into_owned()
                            .collect::<HashMap<String, String>>();
                        if let Some(secret) = params.get("secret") {
                            debug!("Authenticating user");
                            if auth.auth_token_ok(secret) {
                                debug!("Authentication success");
                                let token = auth.new_auth_token();
                                let resp = Response::builder()
                                    .typed_header(ContentType::text())
                                    .typed_header(ContentLength(token.len() as u64))
                                    .header(
                                        SET_COOKIE,
                                        format!(
                                            "{}={}; Max-Age={}; SameSite=Lax",
                                            COOKIE_NAME,
                                            token,
                                            10 * 365 * 24 * 3600
                                        )
                                        .as_str(),
                                    );

                                Ok(AuthResult::LoggedIn(resp.body(token.into()).unwrap()))
                            } else {
                                error!(
                                    "Invalid authentication: invalid shared secret, client: {:?}",
                                    req.remote_addr()
                                );
                                // Let's not return failure immediately, because somebody is using wrong shared secret
                                // Legitimate user can wait a bit, but for brute force attack it can be advantage not to reply quickly
                                sleep(Duration::from_millis(500)).await;
                                Ok(deny())
                            }
                        } else {
                            error!(
                                "Invalid authentication: missing shared secret, client: {:?}",
                                req.remote_addr()
                            );
                            Ok(deny())
                        }
                    }
                }
            });
        } else {
            // And in this part we check token
            let mut token = req
                .headers()
                .typed_get::<Authorization<Bearer>>()
                .map(|a| a.0.token().to_owned());
            if token.is_none() {
                token = req
                    .headers()
                    .typed_get::<Cookie>()
                    .and_then(|c| c.get(COOKIE_NAME).map(borrow::ToOwned::to_owned));
            }

            if token.is_none() {
                error!(
                    "Invalid access: missing token on path {}, client: {:?}",
                    req.path(),
                    req.remote_addr()
                );
                return Box::pin(future::ok(deny()));
            }
            if !self.secrets.token_ok(&token.unwrap()) {
                error!(
                    "Invalid access: invalid token on path {}, client: {:?}",
                    req.path(),
                    req.remote_addr()
                );
                return Box::pin(future::ok(deny()));
            }
        }
        // If everything is ok we return credentials (in this case they are just unit type) and we return back request
        Box::pin(future::ok(AuthResult::Authenticated {
            request: req,
            credentials: (),
        }))
    }
}

impl Secrets {
    fn auth_token_ok(&self, token: &str) -> bool {
        let parts = token
            .split('|')
            .filter_map(|s| match BASE64.decode(s.as_bytes()) {
                Ok(x) => Some(x),
                Err(e) => {
                    error!(
                        "Invalid base64 in authentication token {} in string {}",
                        e, s
                    );
                    None
                }
            })
            .collect::<Vec<_>>();
        if parts.len() == 2 {
            if parts[0].len() != 32 {
                error!("Random salt must be 32 bytes");
                return false;
            }
            let mut hash2 = self.shared_secret.clone().into_bytes();
            let hash = &parts[1];
            hash2.extend(&parts[0]);
            let hash2 = digest(&SHA256, &hash2);

            return hash2.as_ref() == &hash[..];
        } else {
            error!("Incorrectly formed login token - {} parts", parts.len())
        }
        false
    }
    fn new_auth_token(&self) -> String {
        Token::new(self.token_validity_hours, &self.server_secret).into()
    }

    fn token_ok(&self, token: &str) -> bool {
        match token.parse::<Token>() {
            Ok(token) => token.is_valid(&self.server_secret),
            Err(e) => {
                warn!("Invalid token: {}", e);
                false
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
struct Token {
    random: [u8; 32],
    validity: [u8; 8],
    signature: [u8; 32],
}

fn prepare_data(r: &[u8; 32], v: [u8; 8]) -> [u8; 40] {
    let mut to_sign = [0u8; 40];
    to_sign[0..32].copy_from_slice(&r[..]);
    to_sign[32..40].copy_from_slice(&v[..]);
    to_sign
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Invalid system time")
        .as_secs()
}

impl Token {
    fn new(token_validity_hours: u32, secret: &[u8]) -> Self {
        let mut random = [0u8; 32];
        let rng = SystemRandom::new();
        rng.fill(&mut random)
            .expect("Cannot generate random number");
        let validity: u64 = now() + u64::from(token_validity_hours) * 3600;
        let validity: [u8; 8] = unsafe { ::std::mem::transmute(validity.to_be()) };
        let to_sign = prepare_data(&random, validity);
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
        let sig = hmac::sign(&key, &to_sign);
        let slice = sig.as_ref();
        assert!(slice.len() == 32);
        let mut signature = [0u8; 32];
        signature.copy_from_slice(slice);

        Token {
            random,
            validity,
            signature,
        }
    }

    fn is_valid(&self, secret: &[u8]) -> bool {
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
        let data = prepare_data(&self.random, self.validity);
        if hmac::verify(&key, &data, &self.signature).is_err() {
            return false;
        };

        self.validity() > now()
    }

    fn validity(&self) -> u64 {
        let ts: u64 = unsafe { ::std::mem::transmute_copy(&self.validity) };
        u64::from_be(ts)
    }
}

impl Into<String> for Token {
    fn into(self) -> String {
        let data = [&self.random[..], &self.validity[..], &self.signature[..]].concat();
        BASE64.encode(&data)
    }
}

#[derive(Error, Debug, PartialEq)]
enum TokenError {
    #[error("Invalid token size")]
    InvalidSize,

    #[error("Invalid token encoding")]
    InvalidEncoding(#[from] ::data_encoding::DecodeError),
}

impl ::std::str::FromStr for Token {
    type Err = TokenError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = BASE64.decode(s.as_bytes())?;
        if bytes.len() != 72 {
            return Err(TokenError::InvalidSize);
        };
        let mut random = [0u8; 32];
        let mut validity = [0u8; 8];
        let mut signature = [0u8; 32];

        random.copy_from_slice(&bytes[0..32]);
        validity.copy_from_slice(&bytes[32..40]);
        signature.copy_from_slice(&bytes[40..72]);

        Ok(Token {
            random,
            validity,
            signature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::init::init_default_config;
    use borrow::Cow;
    use hyper::{Request, StatusCode};

    #[test]
    fn test_token() {
        let token = Token::new(24, b"my big secret");
        assert!(token.is_valid(b"my big secret"));
        let orig_token = token.clone();
        let serialized_token: String = token.into();
        assert!(serialized_token.len() >= 72);
        let new_token: Token = serialized_token.parse().unwrap();
        assert_eq!(orig_token, new_token);
        assert!(new_token.is_valid(b"my big secret"));
        assert!(!new_token.is_valid(b"wrong secret"));
        assert!(new_token.validity() - now() <= 24 * 3600);
    }

    fn build_request(body: impl Into<Body>) -> RequestWrapper {
        let b = body.into();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/authenticate")
            .body(b)
            .unwrap();

        RequestWrapper::new(req, None, None, false).unwrap()
    }

    fn build_authenticated_request(token: &str) -> RequestWrapper {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/neco")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::from("Hey"))
            .unwrap();

        RequestWrapper::new(req, None, None, false).unwrap()
    }

    fn shared_secret(sec: &str) -> String {
        let mut salt = [0u8; 32];
        let rng = SystemRandom::new();
        rng.fill(&mut salt).expect("cannot generate random number");
        let mut res = BASE64.encode(&salt);
        res.push('|');
        let mut hash: Vec<u8> = sec.into();
        hash.extend(&salt);
        let hash = digest(&SHA256, &hash);
        res.push_str(&BASE64.encode(hash.as_ref()));
        res
    }

    fn shared_secret_form(sec: &str) -> String {
        let ss = shared_secret(sec);
        let encoded_ss: Cow<str> =
            percent_encoding::percent_encode(ss.as_bytes(), percent_encoding::NON_ALPHANUMERIC)
                .into();
        "secret=".to_string() + encoded_ss.as_ref()
    }

    #[tokio::test]
    async fn test_authenticator_login() {
        env_logger::try_init().ok();
        let invalid_secret = "secret=aaaaa";
        let shared = "kulisak";
        init_default_config();

        let ss = shared_secret_form(shared);
        let aut = SharedSecretAuthenticator::new(shared.into(), (&b"123456"[..]).into(), 24);
        let req = build_request(ss);
        let res = aut
            .authenticate(req)
            .await
            .expect("authentication procedure internal error");

        if let AuthResult::LoggedIn(res) = res {
            assert_eq!(res.status(), StatusCode::OK);
            let token = res
                .into_body()
                .filter_map(|x| future::ready(x.ok()))
                .map(|x| x.to_vec())
                .concat()
                .await;
            let token = String::from_utf8(token).expect("token is string");
            assert!(token.len() > 64);
            let req = build_authenticated_request(&token);

            let res = aut
                .authenticate(req)
                .await
                .expect("authentication procedure internal error");

            if let AuthResult::Authenticated { request, .. } = res {
                info!("token {:?} is OK", request.headers().get("Authorization"))
            } else {
                panic!("Token authentication failed")
            }
        } else {
            panic!("Authentication should succeed")
        }

        let wrap = build_request(invalid_secret);

        let res = aut
            .authenticate(wrap)
            .await
            .expect("authentication procedure internal error");

        if let AuthResult::Rejected(res) = res {
            assert_eq!(res.status(), StatusCode::UNAUTHORIZED)
        } else {
            panic!("Authentication should fail");
        }
    }
}
