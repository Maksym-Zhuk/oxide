use std::sync::{Arc, Mutex};

use anesis::context::{AppContext, CleanupState};
use anesis::paths::AnesisPaths;
use assert_fs::TempDir;
use reqwest::Client;

fn make_paths(tmp: &TempDir) -> AnesisPaths {
  AnesisPaths {
    home: tmp.path().to_path_buf(),
    version_check: tmp.path().join("version_check.json"),
    cache: tmp.path().join("cache"),
    templates: tmp.path().join("cache/templates"),
    auth: tmp.path().join("auth.json"),
    addons: tmp.path().join("cache/addons"),
    addons_index: tmp.path().join("cache/addons/anesis-addons.json"),
    stacks: tmp.path().join("cache/stacks"),
  }
}

#[test]
fn new_sets_default_backend_and_frontend_urls() {
  let tmp = TempDir::new().unwrap();
  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));
  let ctx = AppContext::new(make_paths(&tmp), Client::new(), cleanup_state);

  assert_eq!(ctx.backend_url, "https://anesis-server.onrender.com");
  assert_eq!(ctx.frontend_url, "https://anesis-dev.vercel.app");
}

#[test]
fn new_preserves_supplied_paths() {
  let tmp = TempDir::new().unwrap();
  let paths = make_paths(&tmp);
  let expected_auth = paths.auth.clone();
  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));

  let ctx = AppContext::new(paths, Client::new(), cleanup_state);

  assert_eq!(ctx.paths.home, tmp.path());
  assert_eq!(ctx.paths.auth, expected_auth);
}

#[test]
fn new_starts_with_empty_cleanup_state() {
  let tmp = TempDir::new().unwrap();
  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));
  let ctx = AppContext::new(make_paths(&tmp), Client::new(), cleanup_state);

  assert!(ctx.cleanup_state.lock().unwrap().is_none());
}

#[test]
fn cleanup_state_is_shared_via_arc() {
  let tmp = TempDir::new().unwrap();
  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));
  let ctx = AppContext::new(make_paths(&tmp), Client::new(), cleanup_state.clone());

  *cleanup_state.lock().unwrap() = Some(tmp.path().join("in-progress"));

  assert_eq!(
    ctx.cleanup_state.lock().unwrap().as_deref(),
    Some(tmp.path().join("in-progress").as_path())
  );
}
