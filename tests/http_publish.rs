use std::sync::{Arc, Mutex};
use std::time::Duration;

use anesis::{
  addons::{publish::publish_addon, update::update_addon},
  context::AppContext,
  paths::AnesisPaths,
  templates::{publish::publish, update::update},
  utils::errors::AnesisError,
};
use assert_fs::TempDir;
use reqwest::Client;

fn make_ctx_without_auth(tmp: &TempDir) -> AppContext {
  let paths = AnesisPaths {
    home: tmp.path().to_path_buf(),
    version_check: tmp.path().join("version_check.json"),
    cache: tmp.path().join("cache"),
    templates: tmp.path().join("cache/templates"),
    auth: tmp.path().join("auth.json"),
    addons: tmp.path().join("cache/addons"),
    addons_index: tmp.path().join("cache/addons/anesis-addons.json"),
    stacks: tmp.path().join("cache/stacks"),
  };
  let client = Client::builder()
    .timeout(Duration::from_secs(5))
    .build()
    .unwrap();
  let cleanup_state = Arc::new(Mutex::new(None));
  AppContext::new(paths, client, cleanup_state)
}

#[tokio::test]
async fn template_publish_fails_when_not_logged_in() {
  let tmp = TempDir::new().unwrap();
  let ctx = make_ctx_without_auth(&tmp);

  let err = publish(&ctx, "https://github.com/owner/repo", None, None, None)
    .await
    .unwrap_err();
  assert!(
    err
      .downcast_ref::<AnesisError>()
      .is_some_and(|e| matches!(e, AnesisError::NotLoggedIn)),
    "expected NotLoggedIn, got: {err}"
  );
}

#[tokio::test]
async fn template_update_fails_when_not_logged_in() {
  let tmp = TempDir::new().unwrap();
  let ctx = make_ctx_without_auth(&tmp);

  let err = update(&ctx, "https://github.com/owner/repo", None, None, None)
    .await
    .unwrap_err();
  assert!(
    err
      .downcast_ref::<AnesisError>()
      .is_some_and(|e| matches!(e, AnesisError::NotLoggedIn)),
    "expected NotLoggedIn, got: {err}"
  );
}

#[tokio::test]
async fn addon_publish_fails_when_not_logged_in() {
  let tmp = TempDir::new().unwrap();
  let ctx = make_ctx_without_auth(&tmp);

  let err = publish_addon(&ctx, "https://github.com/owner/addon", None, None, None)
    .await
    .unwrap_err();
  assert!(
    err
      .downcast_ref::<AnesisError>()
      .is_some_and(|e| matches!(e, AnesisError::NotLoggedIn)),
    "expected NotLoggedIn, got: {err}"
  );
}

#[tokio::test]
async fn addon_update_fails_when_not_logged_in() {
  let tmp = TempDir::new().unwrap();
  let ctx = make_ctx_without_auth(&tmp);

  let err = update_addon(&ctx, "https://github.com/owner/addon", None, None, None)
    .await
    .unwrap_err();
  assert!(
    err
      .downcast_ref::<AnesisError>()
      .is_some_and(|e| matches!(e, AnesisError::NotLoggedIn)),
    "expected NotLoggedIn, got: {err}"
  );
}

#[test]
fn publish_template_dto_serializes_url() {
  use anesis::templates::publish::PublishTemplateDto;
  let dto = PublishTemplateDto {
    url: "https://github.com/owner/repo".to_string(),
    visibility: None,
    repo_credential_id: None,
    organization_id: None,
  };
  let json = serde_json::to_string(&dto).unwrap();
  assert!(json.contains(r#""url":"https://github.com/owner/repo""#));
}

#[test]
fn update_template_dto_serializes_url() {
  use anesis::templates::update::UpdateTemplateDto;
  let dto = UpdateTemplateDto {
    url: "https://github.com/owner/repo".to_string(),
    visibility: None,
    repo_credential_id: None,
    organization_id: None,
  };
  let json = serde_json::to_string(&dto).unwrap();
  assert!(json.contains(r#""url":"https://github.com/owner/repo""#));
}

#[test]
fn publish_template_dto_omits_none_fields() {
  use anesis::templates::publish::PublishTemplateDto;
  let dto = PublishTemplateDto {
    url: "https://github.com/owner/repo".to_string(),
    visibility: None,
    repo_credential_id: None,
    organization_id: None,
  };
  let json = serde_json::to_string(&dto).unwrap();
  assert!(!json.contains("visibility"));
  assert!(!json.contains("repo_credential_id"));
  assert!(!json.contains("organization_id"));
}

#[test]
fn publish_template_dto_includes_visibility_when_set() {
  use anesis::templates::publish::PublishTemplateDto;
  let dto = PublishTemplateDto {
    url: "https://github.com/owner/repo".to_string(),
    visibility: Some("private".to_string()),
    repo_credential_id: None,
    organization_id: None,
  };
  let json = serde_json::to_string(&dto).unwrap();
  assert!(json.contains(r#""visibility":"private""#));
}

#[test]
fn update_template_dto_omits_none_fields() {
  use anesis::templates::update::UpdateTemplateDto;
  let dto = UpdateTemplateDto {
    url: "https://github.com/owner/repo".to_string(),
    visibility: None,
    repo_credential_id: None,
    organization_id: None,
  };
  let json = serde_json::to_string(&dto).unwrap();
  assert!(!json.contains("visibility"));
  assert!(!json.contains("repo_credential_id"));
  assert!(!json.contains("organization_id"));
}
