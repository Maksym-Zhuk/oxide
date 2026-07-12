use std::{fs, path::Path};

use anyhow::Result;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;

use crate::{auth::server::User, utils::errors::AnesisError};

#[derive(Deserialize)]
struct ExpClaim {
  exp: i64,
}

pub fn get_auth_user(auth_path: &Path) -> Result<User> {
  // A PAT in the environment wins over the stored browser session — this is how
  // CI and AI agents authenticate without an interactive login. PATs aren't
  // JWTs, so the local expiry check is skipped (the server validates them).
  if let Ok(token) = std::env::var("ANESIS_TOKEN") {
    let token = token.trim();
    if !token.is_empty() {
      return Ok(User {
        token: token.to_string(),
        name: "token".to_string(),
      });
    }
  }

  let auth_json_str = fs::read_to_string(auth_path).map_err(|_| AnesisError::NotLoggedIn)?;
  let user: User = serde_json::from_str(&auth_json_str)?;

  if is_token_expired(&user.token) {
    return Err(AnesisError::SessionExpired.into());
  }

  Ok(user)
}

/// Best-effort local check of the JWT `exp` claim so authenticated commands
/// fail fast with a clear "re-login" message instead of a doomed request.
/// Returns false when the token can't be parsed — the server stays the source
/// of truth for signature/token_version validation.
fn is_token_expired(token: &str) -> bool {
  let Some(payload_b64) = token.split('.').nth(1) else {
    return false;
  };
  let Ok(bytes) = URL_SAFE_NO_PAD.decode(payload_b64) else {
    return false;
  };
  match serde_json::from_slice::<ExpClaim>(&bytes) {
    Ok(claim) => claim.exp <= chrono::Utc::now().timestamp(),
    Err(_) => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn token_with_exp(exp: i64) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256"}"#);
    let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{exp}}}"#));
    format!("{header}.{payload}.sig")
  }

  #[test]
  fn detects_expired_and_valid_tokens() {
    let now = chrono::Utc::now().timestamp();
    assert!(is_token_expired(&token_with_exp(now - 60)));
    assert!(!is_token_expired(&token_with_exp(now + 3600)));
    // Unparseable tokens defer to the server rather than blocking locally.
    assert!(!is_token_expired("not-a-jwt"));
  }
}
