use std::{fs, path::Path};

use anyhow::{Context, Result};
use clap::Command;

pub fn generate(dir: &Path) -> Result<Vec<String>> {
  fs::create_dir_all(dir)
    .with_context(|| format!("Failed to create man directory {}", dir.display()))?;

  let command = crate::cli::command();
  let mut written = Vec::new();
  write_page(dir, &command, "anesis", &mut written)?;
  Ok(written)
}

fn write_page(
  dir: &Path,
  command: &Command,
  page_name: &str,
  written: &mut Vec<String>,
) -> Result<()> {
  let file_name = format!("{page_name}.1");
  let path = dir.join(&file_name);

  let mut buffer = Vec::new();
  clap_mangen::Man::new(command.clone())
    .title(page_name.to_uppercase())
    .render(&mut buffer)
    .with_context(|| format!("Failed to render {file_name}"))?;
  fs::write(&path, buffer).with_context(|| format!("Failed to write {}", path.display()))?;
  written.push(file_name);

  for sub in command.get_subcommands() {
    if sub.is_hide_set() {
      continue;
    }
    let nested = format!("{page_name}-{}", sub.get_name());
    write_page(dir, sub, &nested, written)?;
  }

  Ok(())
}
