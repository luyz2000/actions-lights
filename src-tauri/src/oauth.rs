//! "Iniciar sesión con GitHub" via the OAuth device flow (no redirect server needed).

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const GITHUB_WEB: &str = "https://github.com";

/// OAuth App client id baked in at build time; without it the login button is hidden.
pub fn client_id() -> Option<&'static str> {
    option_env!("ACTIONS_LIGHTS_OAUTH_CLIENT_ID").filter(|s| !s.is_empty())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PollResult {
    Pending,
    SlowDown,
    Token(String),
    Denied,
    Expired,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

fn http() -> reqwest::Client {
    reqwest::Client::builder().timeout(Duration::from_secs(30)).build().expect("http client")
}

pub async fn request_code(base: &str, client_id: &str) -> Result<DeviceCode, String> {
    http()
        .post(format!("{base}/login/device/code"))
        .header("Accept", "application/json")
        .form(&[("client_id", client_id), ("scope", "repo")])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("Respuesta inválida de GitHub: {e}"))
}

pub async fn poll_once(base: &str, client_id: &str, device_code: &str) -> Result<PollResult, String> {
    let r: TokenResponse = http()
        .post(format!("{base}/login/oauth/access_token"))
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("Respuesta inválida de GitHub: {e}"))?;
    match (r.access_token, r.error.as_deref()) {
        (Some(t), _) => Ok(PollResult::Token(t)),
        (None, Some("authorization_pending")) => Ok(PollResult::Pending),
        (None, Some("slow_down")) => Ok(PollResult::SlowDown),
        (None, Some("access_denied")) => Ok(PollResult::Denied),
        (None, Some("expired_token")) => Ok(PollResult::Expired),
        (None, other) => Err(format!("Error de inicio de sesión: {}", other.unwrap_or("desconocido"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;

    #[tokio::test]
    async fn request_code_parses_response() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("POST", "/login/device/code")
            .match_header("accept", "application/json")
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded("client_id".into(), "cid".into()),
                Matcher::UrlEncoded("scope".into(), "repo".into()),
            ]))
            .with_body(r#"{"device_code":"dc","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#)
            .create_async()
            .await;
        let code = request_code(&server.url(), "cid").await.unwrap();
        assert_eq!(code.user_code, "ABCD-1234");
        assert_eq!(code.interval, 5);
        m.assert_async().await;
    }

    async fn poll_with(body: &str) -> Result<PollResult, String> {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("POST", "/login/oauth/access_token")
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded("device_code".into(), "dc".into()),
                Matcher::UrlEncoded("grant_type".into(), "urn:ietf:params:oauth:grant-type:device_code".into()),
            ]))
            .with_body(body)
            .create_async()
            .await;
        poll_once(&server.url(), "cid", "dc").await
    }

    #[tokio::test]
    async fn poll_maps_every_outcome() {
        assert_eq!(poll_with(r#"{"error":"authorization_pending"}"#).await, Ok(PollResult::Pending));
        assert_eq!(poll_with(r#"{"error":"slow_down","interval":10}"#).await, Ok(PollResult::SlowDown));
        assert_eq!(poll_with(r#"{"error":"access_denied"}"#).await, Ok(PollResult::Denied));
        assert_eq!(poll_with(r#"{"error":"expired_token"}"#).await, Ok(PollResult::Expired));
        assert_eq!(
            poll_with(r#"{"access_token":"gho_x","token_type":"bearer","scope":"repo"}"#).await,
            Ok(PollResult::Token("gho_x".into()))
        );
        assert!(poll_with(r#"{"error":"incorrect_client_credentials"}"#).await.is_err());
    }

    #[test]
    fn client_id_absent_in_tests() {
        // Feature is hidden unless the build provides a client id.
        assert_eq!(client_id().is_some(), option_env!("ACTIONS_LIGHTS_OAUTH_CLIENT_ID").is_some());
    }
}
