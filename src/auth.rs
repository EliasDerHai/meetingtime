use std::path::PathBuf;
use std::time::Duration as StdDuration;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    RefreshToken, Scope, TokenResponse, TokenUrl, basic::BasicClient, reqwest::async_http_client,
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::timeout;

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

pub fn token_path() -> Result<PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| anyhow!("could not determine data directory"))?;
    Ok(base.join("meetingtime").join("token.json"))
}

fn build_client() -> Result<BasicClient> {
    let client_id = std::env::var("MEETINGTIME_CLIENT_ID")
        .context("MEETINGTIME_CLIENT_ID env var not set — create a Desktop app OAuth client in Google Cloud Console")?;
    let client_secret = std::env::var("MEETINGTIME_CLIENT_SECRET")
        .context("MEETINGTIME_CLIENT_SECRET env var not set")?;

    let client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .context("invalid auth URL")?,
        Some(
            TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                .context("invalid token URL")?,
        ),
    )
    .set_redirect_uri(
        RedirectUrl::new("http://localhost:8080".to_string()).context("invalid redirect URL")?,
    );

    Ok(client)
}

pub async fn run() -> Result<()> {
    let client = build_client()?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/calendar.readonly".to_string(),
        ))
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .set_pkce_challenge(pkce_challenge)
        .url();

    println!("Opening browser for authorization...");
    if open::that(auth_url.as_str()).is_err() {
        println!("Could not open browser automatically. Open this URL manually:\n\n  {auth_url}\n");
    }

    let code = get_auth_code().await?;

    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .context("token exchange failed")?;

    let expires_in = token_response
        .expires_in()
        .unwrap_or(std::time::Duration::from_secs(3600));
    let expires_at = Utc::now() + Duration::from_std(expires_in)?;

    let refresh_token = token_response.refresh_token().ok_or_else(|| {
        anyhow!(
            "no refresh_token in response — ensure access_type=offline and prompt=consent were set"
        )
    })?;

    let token = Token {
        access_token: token_response.access_token().secret().clone(),
        refresh_token: refresh_token.secret().clone(),
        expires_at,
    };

    persist_token(&token)?;
    println!("Token saved to {}", token_path()?.display());
    Ok(())
}

fn parse_code(text: &str) -> Result<String> {
    let pos = text
        .find("code=")
        .ok_or_else(|| anyhow!("no code= found — did you paste the right URL?"))?;
    let after = &text[pos + 5..];
    let code = after
        .split(|c| c == '&' || c == ' ')
        .next()
        .unwrap_or(after)
        .trim();
    Ok(code.to_string())
}

async fn wait_for_redirect(listener: TcpListener) -> Result<String> {
    let (mut stream, _) = listener.accept().await?;

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = std::str::from_utf8(&buf[..n]).context("redirect request was not valid UTF-8")?;

    let code = parse_code(request)?;

    let body = "<html><body><h2>Authorization successful!</h2>\
                <p>You can close this tab and return to the terminal.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await?;

    Ok(code)
}

async fn get_auth_code() -> Result<String> {
    match TcpListener::bind("127.0.0.1:8080").await {
        Ok(listener) => {
            println!("Waiting for browser redirect on http://localhost:8080 ...");
            timeout(StdDuration::from_secs(300), wait_for_redirect(listener))
                .await
                .context("timed out after 5 minutes waiting for OAuth redirect")?
        }
        Err(_) => {
            println!("Port 8080 is in use. After authorizing, paste the redirect URL here:");
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            parse_code(line.trim())
        }
    }
}

pub fn load_token() -> Result<Token> {
    let path = token_path()?;
    let json = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "token not found at {} — run `meetingtime auth` first",
            path.display()
        )
    })?;
    serde_json::from_str(&json).context("token file is malformed")
}

pub async fn refresh_if_needed(token: Token) -> Result<Token> {
    if token.expires_at - Utc::now() > Duration::minutes(5) {
        return Ok(token);
    }

    let client = build_client()?;
    let token_response = client
        .exchange_refresh_token(&RefreshToken::new(token.refresh_token.clone()))
        .request_async(async_http_client)
        .await
        .context("token refresh failed")?;

    let expires_in = token_response
        .expires_in()
        .unwrap_or(std::time::Duration::from_secs(3600));
    let expires_at = Utc::now() + Duration::from_std(expires_in)?;

    let new_token = Token {
        access_token: token_response.access_token().secret().clone(),
        // Google may omit refresh_token on refresh; keep the existing one
        refresh_token: token_response
            .refresh_token()
            .map(|t| t.secret().clone())
            .unwrap_or(token.refresh_token),
        expires_at,
    };

    persist_token(&new_token)?;
    Ok(new_token)
}

fn persist_token(token: &Token) -> Result<()> {
    let path = token_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("could not create directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(token)?;
    std::fs::write(&path, json)
        .with_context(|| format!("could not write token to {}", path.display()))?;
    Ok(())
}
