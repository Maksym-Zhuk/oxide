use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::Result;
use tera::{Context, Tera};

use crate::templates::TemplateFile;

pub fn extract_template(
  files: &[TemplateFile],
  project_name: &str,
  tauri_user_name: Option<String>,
) -> Result<()> {
  let output_path = PathBuf::from(project_name);
  fs::create_dir_all(&output_path)?;

  let mut context = Context::new();
  context.insert("project_name", project_name);
  context.insert("project_name_kebab", &to_kebab_case(project_name));
  context.insert("project_name_snake", &to_snake_case(project_name));
  let user_name = tauri_user_name.unwrap_or("tauri".to_string());
  context.insert("tauri_user_name", &user_name);

  let mut tera = Tera::default();

  extract_dir_contents(files, &output_path, &mut tera, &context)?;

  Ok(())
}

fn to_kebab_case(s: &str) -> String {
  s.chars()
    .map(|c| match c {
      '_' | ' ' => '-',
      _ => c,
    })
    .collect::<String>()
    .to_lowercase()
}

fn to_snake_case(s: &str) -> String {
  s.chars()
    .map(|c| match c {
      '-' | ' ' => '_',
      _ => c,
    })
    .collect::<String>()
    .to_lowercase()
}

pub fn extract_dir_contents(
  files: &[TemplateFile],
  base_path: &Path,
  tera: &mut Tera,
  context: &Context,
) -> Result<()> {
  for file in files {
    let file_name = file.path.file_name().unwrap();
    let file_name_str = file_name.to_string_lossy();

    let output_path = base_path.join(&file.path);
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)?;
    }

    if file_name_str.ends_with(".tera") {
      let output_name = file_name_str.trim_end_matches(".tera");
      let output_path = output_path.with_file_name(output_name);

      let template_content = std::str::from_utf8(&file.contents)?;
      tera.add_raw_template(&file_name_str, template_content)?;
      let rendered = tera.render(&file_name_str, context)?;

      fs::write(&output_path, rendered)?;
      println!("  ✓ {}", output_path.display());
    } else {
      fs::write(&output_path, &file.contents)?;
      println!("  ✓ {}", output_path.display());
    }
  }
  Ok(())
}
