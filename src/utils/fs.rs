use std::{fs, path::Path};

use anyhow::Result;
use ignore::WalkBuilder;

use crate::templates::TemplateFile;

pub fn copy_dir_respecting_gitignore(source: &Path, dest: &Path) -> Result<()> {
  for entry in WalkBuilder::new(source)
    .hidden(false)
    .require_git(false)
    .filter_entry(|e| e.file_name() != ".git")
    .build()
  {
    let entry = entry?;
    if !entry.file_type().is_some_and(|t| t.is_file()) {
      continue;
    }
    let relative = entry.path().strip_prefix(source)?;
    let out = dest.join(relative);
    if let Some(parent) = out.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::copy(entry.path(), &out)?;
  }
  Ok(())
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
