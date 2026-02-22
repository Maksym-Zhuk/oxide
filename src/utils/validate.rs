use std::path::Path;

use anyhow::{Result, anyhow};
use regex::Regex;

pub fn validate_project_name(name: &str) -> Result<()> {
  if name == "." {
    return Ok(());
  }

  if Path::new(name).exists() {
    return Err(anyhow!("Directory '{}' already exists!", name));
  }
  if name.is_empty() {
    return Err(anyhow!("Project name cannot be empty"));
  }

  if name.len() > 255 {
    return Err(anyhow!("Project name is too long (max 255 characters)"));
  }

  let valid_chars = Regex::new(r"^[a-zA-Z0-9_\-\.]+$").unwrap();
  if !valid_chars.is_match(name) {
    return Err(anyhow!(
      "Project name can only contain letters, numbers, hyphens, underscores, and dots"
    ));
  }

  if name.starts_with('.') {
    return Err(anyhow!("Project name cannot start with a dot"));
  }

  if name.ends_with('.') || name.ends_with(' ') {
    return Err(anyhow!("Project name cannot end with a dot or space"));
  }

  let reserved_windows = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
  ];

  let uppercase_name = name.to_uppercase();
  if reserved_windows.contains(&uppercase_name.as_str()) {
    return Err(anyhow!("'{}' is a reserved name in Windows", name));
  }

  Ok(())
}
