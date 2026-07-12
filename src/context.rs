use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
};

use reqwest::Client;

use crate::paths::AnesisPaths;

pub type CleanupState = Arc<Mutex<Option<PathBuf>>>;

pub struct AppContext {
  pub paths: AnesisPaths,
  pub client: Client,
  pub cleanup_state: CleanupState,
  pub backend_url: String,
  pub frontend_url: String,
}

impl AppContext {
  pub fn new(paths: AnesisPaths, client: Client, cleanup_state: CleanupState) -> Self {
    // Default to production; override with env vars for local development
    // (e.g. ANESIS_BACKEND_URL=http://localhost:4000).
    let backend_url = std::env::var("ANESIS_BACKEND_URL")
      .unwrap_or_else(|_| "https://anesis-server.onrender.com".to_string());
    let frontend_url = std::env::var("ANESIS_FRONTEND_URL")
      .unwrap_or_else(|_| "https://anesis-dev.vercel.app".to_string());
    Self {
      paths,
      client,
      cleanup_state,
      backend_url,
      frontend_url,
    }
  }
}
