use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{
  templates::{TemplateFile, install::install_template},
  utils::fs::read_dir_to_files,
};

pub async fn get_files(path: PathBuf, template_path: &Path) -> Result<Vec<TemplateFile>> {
  if !template_path.join(&path).exists() {
    install_template(template_path, &path).await?;
  }

  let files = read_dir_to_files(&template_path.join(&path))?;

  Ok(files)
}
