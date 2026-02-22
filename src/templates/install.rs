use std::{
  fs,
  path::{Path, PathBuf},
  time::Duration,
};

use anyhow::Result;
use reqwest::Client;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};

use crate::{cache::update_templates_cache, utils::git::download_dir};

pub async fn install_template(template_path: &Path, path: &PathBuf) -> Result<()> {
  let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

  let api_url = format!(
    "https://api.github.com/repos/oxide-cli/templates/contents/{}",
    path.to_str().unwrap_or_default()
  );

  let cleanup_path = template_path.join(path);
  let template_path_clone = template_path.to_path_buf();

  ctrlc::set_handler(move || {
    println!("\n⚠ Interrupted! Cleaning up...");
    if cleanup_path.exists() {
      if let Err(e) = fs::remove_dir_all(&cleanup_path) {
        println!("Failed to remove: {}", e);
      }

      let mut current = cleanup_path.parent();
      while let Some(parent) = current {
        if parent == template_path_clone {
          break;
        }
        if fs::remove_dir(parent).is_err() {
          break;
        }
        current = parent.parent();
      }
      println!("✓ Removed incomplete template");
    }
    std::process::exit(1);
  })?;

  println!("Download template from github repo...");
  download_dir(&client, &api_url, &template_path.join(path)).await?;

  update_templates_cache(template_path, path)?;
  println!("Template successfully downloaded");

  Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct RegistryTemplate {
  pub name: String,
  pub path: String,
}

pub async fn install_template_by_name(template_path: &Path, template_name: String) -> Result<()> {
  let oxide_registry_file = template_path.join("oxide-registry.json");

  let registry: Vec<RegistryTemplate> = if !oxide_registry_file.exists() {
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let raw_url = "https://raw.githubusercontent.com/oxide-cli/templates/main/oxide-registry.json";
    let content = client
      .get(raw_url)
      .header(USER_AGENT, "oxide")
      .send()
      .await?
      .text()
      .await?;
    let registry: Vec<RegistryTemplate> = serde_json::from_str(&content)?;
    fs::write(&oxide_registry_file, &content)?;
    registry
  } else {
    let content = fs::read_to_string(&oxide_registry_file)?;
    serde_json::from_str(&content)?
  };

  let entry = registry.iter().find(|t| t.name == template_name);

  if let Some(p) = entry {
    install_template(template_path, &PathBuf::from(&p.path)).await?;
  } else {
    anyhow::bail!("Template '{}' not found in registry", template_name);
  }

  Ok(())
}
