use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod catalog;
pub mod generator;
pub mod info;
pub mod install;
pub mod link;
pub mod loader;
pub mod publish;
pub mod update;

pub struct TemplateFile {
  pub path: PathBuf,
  pub contents: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplate {
  pub name: String,
  pub version: String,
  #[serde(rename = "anesisVersion")]
  pub anesis_version: String,
  pub repository: AnesisTemplateRepository,
  pub metadata: AnesisTemplateMetadata,
  // Prompted once when scaffolding; same schema as addon inputs. Values (and their
  // derived case variants) are exposed to `.tera` files and to `exclude` conditions.
  #[serde(default)]
  pub inputs: Vec<crate::addons::manifest::InputDef>,
  // Files to omit when a condition holds, e.g. `{ "when": "!use_docker", "paths":
  // ["Dockerfile"] }`. `when` is `<input>` or `!<input>` against a boolean input.
  #[serde(default)]
  pub exclude: Vec<ExcludeBlock>,
}

#[derive(Serialize, Deserialize)]
pub struct ExcludeBlock {
  pub when: String,
  pub paths: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplateRepository {
  pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplateMetadata {
  #[serde(rename = "displayName")]
  pub display_name: String,
  pub description: String,
}
