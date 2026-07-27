use std::{
  collections::{HashMap, HashSet},
  fs,
  path::{Component, Path, PathBuf},
};

use anyhow::{Result, anyhow};
use tera::{Context, Tera};

use crate::{
  context::{AppContext, CleanupTask},
  manifest::AnesisManifest,
  templates::{AnesisTemplate, ExcludeBlock, TemplateFile},
};

use super::cache::get_cached_template;

pub fn extract_template(
  files: &[TemplateFile],
  project_name: &str,
  ctx: &AppContext,
  inputs: &HashMap<String, String>,
  excluded: &HashSet<PathBuf>,
) -> Result<()> {
  let output_path = PathBuf::from(project_name);
  let existed_before = output_path.exists();
  fs::create_dir_all(&output_path)?;

  if !existed_before {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(CleanupTask::PartialProject {
      path: output_path.clone(),
    });
  }

  let mut context = Context::new();
  context.insert("project_name", project_name);
  context.insert("project_name_kebab", &to_kebab_case(project_name));
  context.insert("project_name_snake", &to_snake_case(project_name));
  insert_inputs(&mut context, inputs);

  let mut tera = Tera::default();

  let result = extract_dir_contents(files, &output_path, &mut tera, &context, ctx, excluded);

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }

  result
}

pub fn parse_template_manifest(files: &[TemplateFile]) -> Option<AnesisTemplate> {
  files
    .iter()
    .find(|f| {
      f.path
        .file_name()
        .map(|n| n == "anesis.template.json")
        .unwrap_or(false)
    })
    .and_then(|f| serde_json::from_slice(&f.contents).ok())
}

pub fn eval_when(expr: &str, inputs: &HashMap<String, String>) -> bool {
  let expr = expr.trim();
  let (negate, name) = match expr.strip_prefix('!') {
    Some(rest) => (true, rest.trim()),
    None => (false, expr),
  };
  let truthy = inputs.get(name).map(|v| v == "true").unwrap_or(false);
  truthy ^ negate
}

pub fn excluded_paths(
  blocks: &[ExcludeBlock],
  inputs: &HashMap<String, String>,
) -> HashSet<PathBuf> {
  let mut set = HashSet::new();
  for block in blocks {
    if eval_when(&block.when, inputs) {
      set.extend(block.paths.iter().map(PathBuf::from));
    }
  }
  set
}

fn insert_inputs(context: &mut Context, inputs: &HashMap<String, String>) {
  for (k, v) in inputs {
    context.insert(k, v);
    context.insert(format!("{k}_pascal"), &to_pascal_case(v));
    context.insert(format!("{k}_camel"), &to_camel_case(v));
    context.insert(format!("{k}_kebab"), &to_kebab_case(v));
    context.insert(format!("{k}_snake"), &to_snake_case(v));
  }
}

fn relative_output(file: &TemplateFile) -> Option<PathBuf> {
  let name = file.path.file_name()?.to_string_lossy().to_string();
  if name == "anesis.template.json" {
    return None;
  }
  match name.strip_suffix(".tera").filter(|s| !s.is_empty()) {
    Some(stripped) => Some(file.path.with_file_name(stripped)),
    None => Some(file.path.clone()),
  }
}

fn is_excluded(file: &TemplateFile, excluded: &HashSet<PathBuf>) -> bool {
  relative_output(file)
    .map(|p| excluded.contains(&p))
    .unwrap_or(false)
}

pub fn to_kebab_case(s: &str) -> String {
  s.chars()
    .map(|c| match c {
      '_' | ' ' => '-',
      _ => c,
    })
    .collect::<String>()
    .to_lowercase()
}

pub fn to_snake_case(s: &str) -> String {
  s.chars()
    .map(|c| match c {
      '-' | ' ' => '_',
      _ => c,
    })
    .collect::<String>()
    .to_lowercase()
}

pub fn to_pascal_case(s: &str) -> String {
  s.split(['_', '-', ' '])
    .filter(|p| !p.is_empty())
    .map(|word| {
      let mut chars = word.chars();
      match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
      }
    })
    .collect()
}

pub fn to_camel_case(s: &str) -> String {
  let pascal = to_pascal_case(s);
  let mut chars = pascal.chars();
  match chars.next() {
    None => String::new(),
    Some(first) => first.to_lowercase().to_string() + chars.as_str(),
  }
}

fn safe_template_path(base: &Path, relative: &Path) -> Result<PathBuf> {
  let joined = base.join(relative);
  let mut out = PathBuf::new();
  for component in joined.components() {
    match component {
      Component::ParentDir => {
        out.pop();
      }
      Component::CurDir => {}
      c => out.push(c),
    }
  }
  let mut norm_base = PathBuf::new();
  for component in base.components() {
    match component {
      Component::ParentDir => {
        norm_base.pop();
      }
      Component::CurDir => {}
      c => norm_base.push(c),
    }
  }
  if !out.starts_with(&norm_base) {
    return Err(anyhow!(
      "Path traversal blocked: template file '{}' would escape the output directory",
      relative.display()
    ));
  }
  Ok(out)
}

fn resolved_output_path(file: &TemplateFile, base_path: &Path) -> Result<Option<PathBuf>> {
  let file_name = file
    .path
    .file_name()
    .ok_or_else(|| anyhow!("Invalid file path: {}", file.path.display()))?;
  let file_name_str = file_name.to_string_lossy();
  if file_name_str == "anesis.template.json" {
    return Ok(None);
  }
  let output_path = safe_template_path(base_path, &file.path)?;
  match file_name_str
    .strip_suffix(".tera")
    .filter(|s| !s.is_empty())
  {
    Some(output_name) => Ok(Some(output_path.with_file_name(output_name))),
    None => Ok(Some(output_path)),
  }
}

pub fn overwritten_paths(
  files: &[TemplateFile],
  project_name: &str,
  excluded: &HashSet<PathBuf>,
) -> Result<Vec<PathBuf>> {
  let base = PathBuf::from(project_name);
  let mut hits = Vec::new();
  for file in files {
    if is_excluded(file, excluded) {
      continue;
    }
    if let Some(path) = resolved_output_path(file, &base)?
      && path.exists()
    {
      hits.push(path);
    }
  }
  Ok(hits)
}

pub fn extract_dir_contents(
  files: &[TemplateFile],
  base_path: &Path,
  tera: &mut Tera,
  context: &Context,
  ctx: &AppContext,
  excluded: &HashSet<PathBuf>,
) -> Result<()> {
  for file in files {
    if is_excluded(file, excluded) {
      continue;
    }
    let file_name = file
      .path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Invalid file path: {}", file.path.display()))?;
    let file_name_str = file_name.to_string_lossy();
    if file_name_str == "anesis.template.json" {
      let template: AnesisTemplate = serde_json::from_slice(&file.contents)?;
      let template_name = template.name;
      let cached_template = get_cached_template(ctx, &template_name)?.ok_or_else(|| {
        anyhow!("template '{template_name}' not found in cache; cannot write anesis.json")
      })?;
      AnesisManifest::new(&template_name, &cached_template.commit_sha, Vec::new())
        .write(base_path)?;
      continue;
    }
    let template_key = file.path.to_string_lossy();

    let output_path = safe_template_path(base_path, &file.path)?;
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)?;
    }

    if let Some(output_name) = file_name_str.strip_suffix(".tera")
      && !output_name.is_empty()
    {
      let output_path = output_path.with_file_name(output_name);

      let template_content = std::str::from_utf8(&file.contents)?;
      tera.add_raw_template(&template_key, template_content)?;
      let rendered = tera.render(&template_key, context)?;

      fs::write(&output_path, rendered)?;
      println!("  ✓ {}", output_path.display());
    } else {
      fs::write(&output_path, &file.contents)?;
      println!("  ✓ {}", output_path.display());
    }
  }
  Ok(())
}
