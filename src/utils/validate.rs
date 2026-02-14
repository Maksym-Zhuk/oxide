use std::path::Path;

use regex::Regex;

pub fn validate_project_name(name: &str) -> Result<(), String> {
  if Path::new(name).exists() {
    return Err(format!("Directory '{}' already exists!", name));
  }
  if name.is_empty() {
    return Err("Project name cannot be empty".to_string());
  }

  if name.len() > 255 {
    return Err("Project name is too long (max 255 characters)".to_string());
  }

  let valid_chars = Regex::new(r"^[a-zA-Z0-9_\-\.]+$").unwrap();
  if !valid_chars.is_match(name) {
    return Err(
      "Project name can only contain letters, numbers, hyphens, underscores, and dots".to_string(),
    );
  }

  if name.starts_with('.') {
    return Err("Project name cannot start with a dot".to_string());
  }

  if name.ends_with('.') || name.ends_with(' ') {
    return Err("Project name cannot end with a dot or space".to_string());
  }

  let reserved_windows = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
  ];

  let uppercase_name = name.to_uppercase();
  if reserved_windows.contains(&uppercase_name.as_str()) {
    return Err(format!("'{}' is a reserved name in Windows", name));
  }

  Ok(())
}
