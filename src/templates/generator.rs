use std::{fs, path::Path};

use include_dir::Dir;
use tera::{Context, Tera};

pub fn extract_template(
  template: &Dir,
  project_name: &str,
  tauri_user_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
  let output_path = Path::new(project_name);

  fs::create_dir_all(output_path)?;

  let mut context = Context::new();
  context.insert("project_name", project_name);
  context.insert("project_name_kebab", &to_kebab_case(project_name));
  context.insert("project_name_snake", &to_snake_case(project_name));
  let user_name = tauri_user_name.unwrap_or("tauri".to_string());
  context.insert("tauri_user_name", &user_name);

  let mut tera = Tera::default();

  extract_dir_contents(template, output_path, &mut tera, &context)?;

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
  dir: &Dir,
  base_path: &Path,
  tera: &mut Tera,
  context: &Context,
) -> Result<(), Box<dyn std::error::Error>> {
  for file in dir.files() {
    let file_name = file.path().file_name().unwrap();
    let file_name_str = file_name.to_string_lossy();

    if file_name_str.ends_with(".tera") {
      let output_name = file_name_str.trim_end_matches(".tera");
      let output_path = base_path.join(output_name);
      let template_content = std::str::from_utf8(file.contents())?;
      tera.add_raw_template(&file_name_str, template_content)?;
      let rendered = tera.render(&file_name_str, context)?;
      fs::write(&output_path, rendered)?;
      println!("  ✓ {}", output_path.display());
    } else {
      let output_path = base_path.join(file_name);
      fs::write(&output_path, file.contents())?;
      println!("  ✓ {}", output_path.display());
    }
  }

  for subdir in dir.dirs() {
    let subdir_name = subdir.path().file_name().unwrap();
    let subdir_path = base_path.join(subdir_name);

    fs::create_dir_all(&subdir_path)?;
    extract_dir_contents(subdir, &subdir_path, tera, context)?;
  }

  Ok(())
}
