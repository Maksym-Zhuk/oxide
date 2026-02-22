use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::Result;

use crate::{
  prompts::{BuildTool, Language, Platform},
  templates::TemplateFile,
};

pub fn generate_path(
  language: &Option<Language>,
  build_tool: &Option<BuildTool>,
  framework_name: &str,
  platform: &Option<Platform>,
) -> PathBuf {
  let mut path = PathBuf::new();

  if let Some(lg) = language {
    match lg {
      Language::JavaScript => path.push("js"),
      Language::TypeScript => path.push("ts"),
    }
  };

  if let Some(bt) = build_tool {
    path.push(bt.to_string().to_lowercase());
  };

  if framework_name == "Qwik" {
    path.push("vite");
  }

  path.push(framework_name.to_string().to_lowercase());

  if let Some(pl) = platform {
    path.push(pl.to_string().to_lowercase());
  };

  path
}

pub fn read_dir_to_files(path: &Path) -> Result<Vec<TemplateFile>> {
  let mut files = Vec::new();
  read_dir_recursive(path, path, &mut files)?;
  Ok(files)
}

pub fn read_dir_recursive(
  base: &Path,
  current: &Path,
  files: &mut Vec<TemplateFile>,
) -> Result<()> {
  for entry in fs::read_dir(current)? {
    let entry = entry?;
    let path = entry.path();
    let file_type = entry.file_type()?;

    if file_type.is_file() {
      let contents = fs::read(&path)?;
      let relative_path = path.strip_prefix(base)?.to_path_buf();
      files.push(TemplateFile {
        path: relative_path,
        contents,
      });
    } else if file_type.is_dir() {
      read_dir_recursive(base, &path, files)?;
    }
  }

  Ok(())
}
