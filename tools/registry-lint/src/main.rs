use std::{
  collections::{HashMap, HashSet},
  path::{Path, PathBuf},
  process::ExitCode,
};

use anyhow::{Context, Result};

const TEMPLATE_SCHEMA: &str = include_str!("../schemas/anesis.template.schema.json");
const ADDON_SCHEMA: &str = include_str!("../schemas/anesis.addon.schema.json");
const STACK_SCHEMA: &str = include_str!("../schemas/anesis.stack.schema.json");

struct Manifest {
  path: PathBuf,
  value: serde_json::Value,
}

fn main() -> ExitCode {
  let args: Vec<String> = std::env::args().collect();
  if args.len() != 4 {
    eprintln!(
      "usage: {} <templates-dir> <addons-dir> <stacks-dir>",
      args.first().map(String::as_str).unwrap_or("registry-lint")
    );
    return ExitCode::FAILURE;
  }

  match run(
    Path::new(&args[1]),
    Path::new(&args[2]),
    Path::new(&args[3]),
  ) {
    Ok(true) => {
      println!("registry-lint: all checks passed");
      ExitCode::SUCCESS
    }
    Ok(false) => ExitCode::FAILURE,
    Err(err) => {
      eprintln!("registry-lint: {err:#}");
      ExitCode::FAILURE
    }
  }
}

fn run(templates_dir: &Path, addons_dir: &Path, stacks_dir: &Path) -> Result<bool> {
  let template_schema = compile_schema(TEMPLATE_SCHEMA)?;
  let addon_schema = compile_schema(ADDON_SCHEMA)?;
  let stack_schema = compile_schema(STACK_SCHEMA)?;

  let templates = load_manifests(templates_dir, "anesis.template.json")?;
  let addons = load_manifests(addons_dir, "anesis.addon.json")?;
  let stacks = load_manifests(stacks_dir, "anesis.stack.json")?;

  let mut errors: Vec<String> = Vec::new();

  for m in &templates {
    validate_schema(&template_schema, m, &mut errors);
  }
  for m in &addons {
    validate_schema(&addon_schema, m, &mut errors);
  }
  for m in &stacks {
    validate_schema(&stack_schema, m, &mut errors);
  }

  let template_names = index_by_field(&templates, "name", &mut errors, "template");
  let addon_ids = index_by_field(&addons, "id", &mut errors, "addon");

  for m in &addons {
    let Some(requires) = m.value.get("requires").and_then(|v| v.as_array()) else {
      continue;
    };
    for req in requires {
      let Some(req_id) = req.as_str() else { continue };
      if !addon_ids.contains_key(req_id) {
        errors.push(format!(
          "{}: requires unknown addon '{req_id}'",
          m.path.display()
        ));
      }
    }
  }

  for m in &addons {
    check_copy_sources(m, &mut errors);
    check_test_fixture(m, &mut errors);
    check_id_matches_directory(m, "addon", &mut errors);
  }
  for m in &stacks {
    check_id_matches_directory(m, "stack", &mut errors);
  }

  let addon_commands = index_addon_commands(&addons);

  for m in &stacks {
    if let Some(template) = m.value.get("template").and_then(|v| v.as_str())
      && !template_names.contains_key(template)
    {
      errors.push(format!(
        "{}: references unknown template '{template}'",
        m.path.display()
      ));
    }
    if let Some(stack_addons) = m.value.get("addons").and_then(|v| v.as_array()) {
      for addon_ref in stack_addons {
        let Some(addon_id) = addon_ref.get("id").and_then(|v| v.as_str()) else {
          continue;
        };
        if !addon_ids.contains_key(addon_id) {
          errors.push(format!(
            "{}: references unknown addon '{addon_id}'",
            m.path.display()
          ));
          continue;
        }

        let command = addon_ref
          .get("command")
          .and_then(|v| v.as_str())
          .unwrap_or("install");
        if let Some(available) = addon_commands.get(addon_id)
          && !available.contains(&command.to_string())
        {
          let mut sorted: Vec<&String> = available.iter().collect();
          sorted.sort();
          errors.push(format!(
            "{}: addon '{addon_id}' has no command '{command}' (available: {})",
            m.path.display(),
            sorted
              .iter()
              .map(|s| s.as_str())
              .collect::<Vec<_>>()
              .join(", ")
          ));
        }
      }
    }
  }

  println!(
    "registry-lint: checked {} template(s), {} addon(s), {} stack(s)",
    templates.len(),
    addons.len(),
    stacks.len()
  );

  if errors.is_empty() {
    Ok(true)
  } else {
    for e in &errors {
      eprintln!("✗ {e}");
    }
    eprintln!("registry-lint: {} error(s)", errors.len());
    Ok(false)
  }
}

fn index_addon_commands(addons: &[Manifest]) -> HashMap<String, HashSet<String>> {
  let mut index: HashMap<String, HashSet<String>> = HashMap::new();

  for m in addons {
    let Some(id) = m.value.get("id").and_then(|v| v.as_str()) else {
      continue;
    };
    let entry = index.entry(id.to_string()).or_default();

    let variants = m.value.get("variants").and_then(|v| v.as_array());
    for variant in variants.into_iter().flatten() {
      let commands = variant.get("commands").and_then(|v| v.as_array());
      for command in commands.into_iter().flatten() {
        if let Some(name) = command.get("name").and_then(|v| v.as_str()) {
          entry.insert(name.to_string());
        }
      }
    }
  }

  index
}

fn check_id_matches_directory(m: &Manifest, kind: &str, errors: &mut Vec<String>) {
  let Some(id) = m.value.get("id").and_then(|v| v.as_str()) else {
    return;
  };
  let Some(directory) = m.path.parent().and_then(|p| p.file_name()) else {
    return;
  };
  if directory != id {
    errors.push(format!(
      "{}: {kind} id '{id}' does not match its directory '{}'",
      m.path.display(),
      directory.display()
    ));
  }
}

fn check_copy_sources(m: &Manifest, errors: &mut Vec<String>) {
  let Some(addon_dir) = m.path.parent() else {
    return;
  };

  let variants = m.value.get("variants").and_then(|v| v.as_array());
  for variant in variants.into_iter().flatten() {
    let commands = variant.get("commands").and_then(|v| v.as_array());
    for command in commands.into_iter().flatten() {
      let command_name = command
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("<unnamed>");
      let steps = command.get("steps").and_then(|v| v.as_array());

      for step in steps.into_iter().flatten() {
        if step.get("type").and_then(|v| v.as_str()) != Some("copy") {
          continue;
        }
        let Some(src) = step.get("src").and_then(|v| v.as_str()) else {
          continue;
        };
        if src.contains("{{") {
          continue;
        }
        if !addon_dir.join(src).exists() {
          errors.push(format!(
            "{}: command '{command_name}' copies '{src}', which does not exist",
            m.path.display()
          ));
        }
      }
    }
  }
}

fn check_test_fixture(m: &Manifest, errors: &mut Vec<String>) {
  let Some(addon_dir) = m.path.parent() else {
    return;
  };
  let fixture = addon_dir.join("test-fixture");
  if !fixture.is_dir() {
    return;
  }

  let detected = detect_variant(m, &fixture);
  let Some(variant) = select_variant(m, detected.as_deref()) else {
    return;
  };

  let commands = variant.get("commands").and_then(|v| v.as_array());
  for command in commands.into_iter().flatten() {
    let command_name = command
      .get("name")
      .and_then(|v| v.as_str())
      .unwrap_or("<unnamed>");

    let has_prerequisites = command
      .get("requires_commands")
      .and_then(|v| v.as_array())
      .is_some_and(|required| !required.is_empty());
    if has_prerequisites {
      continue;
    }

    let steps = command.get("steps").and_then(|v| v.as_array());

    for step in steps.into_iter().flatten() {
      if step.get("type").and_then(|v| v.as_str()) != Some("inject") {
        continue;
      }
      if step.get("if_not_found").and_then(|v| v.as_str()) != Some("error") {
        continue;
      }
      let Some(target) = step.get("target") else {
        continue;
      };

      for (label, contents) in resolve_fixture_target(target, &fixture, m, command_name, errors) {
        for key in ["after", "before"] {
          let Some(anchor) = step.get(key).and_then(|v| v.as_str()) else {
            continue;
          };
          if anchor.contains("{{") {
            continue;
          }
          if !contents.contains(anchor.trim()) {
            errors.push(format!(
              "{}: test-fixture '{label}' has no anchor {key:?} = {anchor:?} \
               (command '{command_name}' would fail against it)",
              m.path.display()
            ));
          }
        }
      }
    }
  }
}

fn resolve_fixture_target(
  target: &serde_json::Value,
  fixture: &Path,
  m: &Manifest,
  command_name: &str,
  errors: &mut Vec<String>,
) -> Vec<(String, String)> {
  if let Some(file) = target.get("file").and_then(|v| v.as_str()) {
    if file.contains("{{") {
      return Vec::new();
    }
    return match std::fs::read_to_string(fixture.join(file)) {
      Ok(contents) => vec![(file.to_string(), contents)],
      Err(_) => {
        errors.push(format!(
          "{}: test-fixture is missing '{file}', which command '{command_name}' injects into",
          m.path.display()
        ));
        Vec::new()
      }
    };
  }

  let Some(pattern) = target.get("glob").and_then(|v| v.as_str()) else {
    return Vec::new();
  };
  if pattern.contains("{{") {
    return Vec::new();
  }

  let joined = fixture.join(pattern);
  let matched: Vec<(String, String)> = glob::glob(&joined.to_string_lossy())
    .into_iter()
    .flatten()
    .filter_map(|entry| entry.ok())
    .filter_map(|path| {
      let contents = std::fs::read_to_string(&path).ok()?;
      let label = path
        .strip_prefix(fixture)
        .unwrap_or(&path)
        .display()
        .to_string();
      Some((label, contents))
    })
    .collect();

  if matched.is_empty() {
    errors.push(format!(
      "{}: test-fixture has no file matching '{pattern}', which command '{command_name}' injects into",
      m.path.display()
    ));
  }
  matched
}

fn select_variant<'a>(m: &'a Manifest, detected: Option<&str>) -> Option<&'a serde_json::Value> {
  let variants = m.value.get("variants")?.as_array()?;
  variants
    .iter()
    .find(|v| v.get("when").and_then(|w| w.as_str()) == detected)
    .or_else(|| {
      variants
        .iter()
        .find(|v| v.get("when").map(|w| w.is_null()).unwrap_or(true))
    })
}

fn detect_variant(m: &Manifest, dir: &Path) -> Option<String> {
  let blocks = m.value.get("detect")?.as_array()?;
  for block in blocks {
    let rules = block.get("rules").and_then(|v| v.as_array())?;
    let any = block.get("match").and_then(|v| v.as_str()) == Some("any");
    let mut results = rules.iter().map(|rule| eval_detect_rule(rule, dir));
    let matched = if any {
      results.any(|r| r)
    } else {
      results.all(|r| r)
    };
    if matched {
      return block.get("id").and_then(|v| v.as_str()).map(str::to_string);
    }
  }
  None
}

fn eval_detect_rule(rule: &serde_json::Value, dir: &Path) -> bool {
  let negate = rule
    .get("negate")
    .and_then(|v| v.as_bool())
    .unwrap_or(false);
  let file = rule
    .get("file")
    .and_then(|v| v.as_str())
    .unwrap_or_default();
  let path = dir.join(file);

  let result = match rule.get("type").and_then(|v| v.as_str()) {
    Some("file_exists") => path.exists(),
    Some("file_contains") => match (std::fs::read_to_string(&path), rule.get("contains")) {
      (Ok(contents), Some(needle)) => needle
        .as_str()
        .is_some_and(|needle| contents.contains(needle)),
      _ => false,
    },
    Some("json_contains") => {
      let Some(key_path) = rule.get("key_path").and_then(|v| v.as_str()) else {
        return negate;
      };
      let found = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|json| {
          let mut cursor = &json;
          for segment in key_path.split('.') {
            cursor = cursor.get(segment)?;
          }
          Some(cursor.clone())
        });
      match (found, rule.get("value").and_then(|v| v.as_str())) {
        (Some(actual), Some(expected)) => actual.as_str() == Some(expected),
        (Some(_), None) => true,
        (None, _) => false,
      }
    }
    Some("toml_contains") => {
      let Some(key_path) = rule.get("key_path").and_then(|v| v.as_str()) else {
        return negate;
      };
      let found = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| toml::from_str::<toml::Value>(&raw).ok())
        .and_then(|table| {
          let mut cursor = &table;
          for segment in key_path.split('.') {
            cursor = cursor.get(segment)?;
          }
          Some(cursor.clone())
        });
      match (found, rule.get("value").and_then(|v| v.as_str())) {
        (Some(actual), Some(expected)) => actual.as_str() == Some(expected),
        (Some(_), None) => true,
        (None, _) => false,
      }
    }
    _ => false,
  };

  if negate { !result } else { result }
}

fn compile_schema(raw: &str) -> Result<jsonschema::Validator> {
  let schema = serde_json::from_str(raw).context("embedded schema is invalid JSON")?;
  jsonschema::validator_for(&schema).context("embedded schema is not a valid JSON Schema")
}

fn validate_schema(validator: &jsonschema::Validator, m: &Manifest, errors: &mut Vec<String>) {
  for err in validator.iter_errors(&m.value) {
    errors.push(format!(
      "{}: {} (at {})",
      m.path.display(),
      err,
      err.instance_path
    ));
  }
}

fn index_by_field(
  manifests: &[Manifest],
  field: &str,
  errors: &mut Vec<String>,
  kind: &str,
) -> HashMap<String, PathBuf> {
  let mut index = HashMap::new();
  for m in manifests {
    let Some(value) = m.value.get(field).and_then(|v| v.as_str()) else {
      continue;
    };
    if let Some(existing) = index.insert(value.to_string(), m.path.clone()) {
      errors.push(format!(
        "{}: duplicate {kind} '{value}' (also defined in {})",
        m.path.display(),
        existing.display()
      ));
    }
  }
  index
}

fn load_manifests(dir: &Path, file_name: &str) -> Result<Vec<Manifest>> {
  let mut manifests = Vec::new();
  if !dir.exists() {
    return Ok(manifests);
  }
  for entry in walkdir::WalkDir::new(dir)
    .into_iter()
    .filter_map(|e| e.ok())
  {
    if entry.file_type().is_file() && entry.file_name() == file_name {
      let path = entry.path().to_path_buf();
      let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("could not read {}", path.display()))?;
      let value: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;
      manifests.push(Manifest { path, value });
    }
  }
  Ok(manifests)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  struct Registry {
    root: tempfile::TempDir,
  }

  impl Registry {
    fn new() -> Self {
      let root = tempfile::TempDir::new().unwrap();
      for dir in ["templates", "addons", "stacks"] {
        fs::create_dir_all(root.path().join(dir)).unwrap();
      }
      Self { root }
    }

    fn write(&self, relative: &str, contents: &str) {
      let path = self.root.path().join(relative);
      fs::create_dir_all(path.parent().unwrap()).unwrap();
      fs::write(path, contents).unwrap();
    }

    fn lint(&self) -> Result<bool> {
      run(
        &self.root.path().join("templates"),
        &self.root.path().join("addons"),
        &self.root.path().join("stacks"),
      )
    }

    fn add_template(&self, name: &str) {
      self.write(
        &format!("templates/{name}/anesis.template.json"),
        &format!(
          r#"{{
  "name": "{name}",
  "version": "1.0.0",
  "anesisVersion": ">=1.0.0",
  "author": {{ "name": "Test", "github": "test" }},
  "repository": {{ "type": "github", "url": "https://github.com/test/templates", "release": "v1.0.0" }},
  "specialization": "frontend",
  "scope": "web",
  "technologies": ["react"],
  "languages": ["typescript"],
  "type": "base",
  "metadata": {{ "displayName": "{name}", "description": "A test template.", "tags": ["test", "fixture"] }}
}}"#
        ),
      );
    }

    fn add_addon(&self, id: &str, steps: &str) {
      self.write(
        &format!("addons/{id}/anesis.addon.json"),
        &format!(
          r#"{{
  "schema_version": "1",
  "id": "{id}",
  "name": "{id}",
  "version": "1.0.0",
  "description": "A test addon.",
  "author": {{ "name": "Test", "github": "test" }},
  "variants": [
    {{
      "when": null,
      "commands": [
        {{ "name": "install", "steps": [{steps}] }}
      ]
    }}
  ]
}}"#
        ),
      );
    }

    fn add_stack(&self, id: &str, template: &str, addons: &str) {
      self.write(
        &format!("stacks/{id}/anesis.stack.json"),
        &format!(
          r#"{{
  "schema_version": "1",
  "id": "{id}",
  "name": "{id}",
  "version": "1.0.0",
  "author": {{ "name": "Test", "github": "test" }},
  "template": "{template}",
  "addons": [{addons}]
}}"#
        ),
      );
    }
  }

  #[test]
  fn a_clean_registry_passes() {
    let registry = Registry::new();
    registry.add_template("react-vite-ts");
    registry.add_addon(
      "docker",
      r#"{ "type": "create", "path": "a.txt", "content": "x" }"#,
    );
    registry.add_stack(
      "my-stack",
      "react-vite-ts",
      r#"{ "id": "docker", "command": "install" }"#,
    );

    assert!(registry.lint().unwrap());
  }

  #[test]
  fn an_addon_may_use_either_author_shape() {
    let registry = Registry::new();
    registry.write(
      "addons/legacy/anesis.addon.json",
      r#"{
  "schema_version": "1",
  "id": "legacy",
  "name": "legacy",
  "version": "1.0.0",
  "description": "Published before the shapes were aligned.",
  "author": "anesis-dev",
  "variants": []
}"#,
    );

    assert!(
      registry.lint().unwrap(),
      "a string author must stay valid so already-published addons keep working"
    );
  }

  #[test]
  fn a_copy_step_pointing_at_a_missing_file_is_rejected() {
    let registry = Registry::new();
    registry.add_addon(
      "docker",
      r#"{ "type": "copy", "src": "files/compose.yml", "dest": "docker-compose.yml" }"#,
    );

    assert!(
      !registry.lint().unwrap(),
      "a copy step whose source is absent must not pass lint"
    );
  }

  #[test]
  fn a_copy_step_pointing_at_a_real_file_is_accepted() {
    let registry = Registry::new();
    registry.add_addon(
      "docker",
      r#"{ "type": "copy", "src": "files/compose.yml", "dest": "docker-compose.yml" }"#,
    );
    registry.write("addons/docker/files/compose.yml", "services: {}\n");

    assert!(registry.lint().unwrap());
  }

  #[test]
  fn a_templated_copy_source_is_skipped() {
    let registry = Registry::new();
    registry.add_addon(
      "docker",
      r#"{ "type": "copy", "src": "files/{{ flavour }}.yml", "dest": "out.yml" }"#,
    );

    assert!(registry.lint().unwrap());
  }

  #[test]
  fn a_stack_naming_a_command_the_addon_lacks_is_rejected() {
    let registry = Registry::new();
    registry.add_template("react-vite-ts");
    registry.add_addon(
      "docker",
      r#"{ "type": "create", "path": "a.txt", "content": "x" }"#,
    );
    registry.add_stack(
      "my-stack",
      "react-vite-ts",
      r#"{ "id": "docker", "command": "configure" }"#,
    );

    assert!(
      !registry.lint().unwrap(),
      "'configure' does not exist on the addon, so the stack would fail at apply time"
    );
  }

  #[test]
  fn an_omitted_stack_command_is_checked_against_install() {
    let registry = Registry::new();
    registry.add_template("react-vite-ts");
    registry.write(
      "addons/docker/anesis.addon.json",
      r#"{
  "schema_version": "1",
  "id": "docker",
  "name": "docker",
  "version": "1.0.0",
  "description": "Has no install command.",
  "author": { "name": "Test", "github": "test" },
  "variants": [
    { "when": null, "commands": [{ "name": "configure", "steps": [] }] }
  ]
}"#,
    );
    registry.add_stack("my-stack", "react-vite-ts", r#"{ "id": "docker" }"#);

    assert!(!registry.lint().unwrap());
  }

  #[test]
  fn a_fixture_missing_an_injection_target_is_rejected() {
    let registry = Registry::new();
    registry.add_addon(
      "nest-health",
      r#"{
        "type": "inject",
        "target": { "type": "file", "file": "src/app.module.ts" },
        "content": "HealthModule,",
        "after": "// anesis:top-imports",
        "if_not_found": "error"
      }"#,
    );
    registry.write("addons/nest-health/test-fixture/package.json", "{}");

    assert!(
      !registry.lint().unwrap(),
      "the fixture has no src/app.module.ts to inject into"
    );
  }

  #[test]
  fn a_fixture_missing_the_anchor_text_is_rejected() {
    let registry = Registry::new();
    registry.add_addon(
      "nest-health",
      r#"{
        "type": "inject",
        "target": { "type": "file", "file": "src/app.module.ts" },
        "content": "HealthModule,",
        "after": "// anesis:top-imports",
        "if_not_found": "error"
      }"#,
    );
    registry.write(
      "addons/nest-health/test-fixture/src/app.module.ts",
      "export class AppModule {}\n",
    );

    assert!(
      !registry.lint().unwrap(),
      "the file exists but has no `// anesis:top-imports` marker"
    );
  }

  #[test]
  fn a_fixture_with_the_target_and_anchor_is_accepted() {
    let registry = Registry::new();
    registry.add_addon(
      "nest-health",
      r#"{
        "type": "inject",
        "target": { "type": "file", "file": "src/app.module.ts" },
        "content": "HealthModule,",
        "after": "// anesis:top-imports",
        "if_not_found": "error"
      }"#,
    );
    registry.write(
      "addons/nest-health/test-fixture/src/app.module.ts",
      "// anesis:top-imports\nexport class AppModule {}\n",
    );

    assert!(registry.lint().unwrap());
  }

  #[test]
  fn only_the_variant_the_fixture_selects_is_checked() {
    let registry = Registry::new();
    registry.write(
      "addons/tailwind/anesis.addon.json",
      r#"{
  "schema_version": "1",
  "id": "tailwind",
  "name": "tailwind",
  "version": "1.0.0",
  "description": "Two bundlers, two very different files.",
  "author": { "name": "Test", "github": "test" },
  "detect": [
    { "id": "vite", "rules": [{ "type": "json_contains", "file": "package.json", "key_path": "devDependencies.vite" }] },
    { "id": "farm", "rules": [{ "type": "json_contains", "file": "package.json", "key_path": "devDependencies.@farmfe/core" }] }
  ],
  "variants": [
    {
      "when": "vite",
      "commands": [{ "name": "install", "steps": [{
        "type": "inject",
        "target": { "type": "file", "file": "vite.config.ts" },
        "content": "tailwindcss(),",
        "before": "// anesis:build-plugins",
        "if_not_found": "error"
      }] }]
    },
    {
      "when": "farm",
      "commands": [{ "name": "install", "steps": [{
        "type": "inject",
        "target": { "type": "file", "file": "farm.config.ts" },
        "content": "postcss(),",
        "before": "// anesis:build-plugins",
        "if_not_found": "error"
      }] }]
    }
  ]
}"#,
    );
    registry.write(
      "addons/tailwind/test-fixture/package.json",
      r#"{ "devDependencies": { "vite": "^7" } }"#,
    );
    registry.write(
      "addons/tailwind/test-fixture/vite.config.ts",
      "plugins: [\n  // anesis:build-plugins\n]\n",
    );

    assert!(
      registry.lint().unwrap(),
      "the fixture is a Vite project, so the Farm variant must not be checked against it"
    );
  }

  #[test]
  fn a_toml_detect_rule_selects_a_variant() {
    let registry = Registry::new();
    registry.write(
      "addons/axum-tracing/anesis.addon.json",
      r#"{
  "schema_version": "1",
  "id": "axum-tracing",
  "name": "axum-tracing",
  "version": "1.0.0",
  "description": "Selected by a toml_contains rule.",
  "author": { "name": "Test", "github": "test" },
  "detect": [
    { "id": "axum", "rules": [{ "type": "toml_contains", "file": "Cargo.toml", "key_path": "dependencies.axum" }] }
  ],
  "variants": [
    {
      "when": "axum",
      "commands": [{ "name": "install", "steps": [{
        "type": "inject",
        "target": { "type": "file", "file": "src/main.rs" },
        "content": "// logging",
        "before": "// anesis:startup",
        "if_not_found": "error"
      }] }]
    }
  ]
}"#,
    );
    registry.write(
      "addons/axum-tracing/test-fixture/Cargo.toml",
      "[dependencies]\naxum = \"0.8\"\n",
    );
    registry.write(
      "addons/axum-tracing/test-fixture/src/main.rs",
      "fn main() {}\n",
    );

    assert!(
      !registry.lint().unwrap(),
      "the variant was selected, so the fixture's missing `// anesis:startup` must be reported"
    );
  }

  #[test]
  fn a_glob_target_matching_nothing_in_the_fixture_is_rejected() {
    let registry = Registry::new();
    registry.add_addon(
      "react-router",
      r#"{
        "type": "inject",
        "target": { "type": "glob", "glob": "src/*.tsx" },
        "content": "<BrowserRouter>",
        "after": "{/* anesis:providers-start */}",
        "if_not_found": "error"
      }"#,
    );
    registry.write("addons/react-router/test-fixture/package.json", "{}");

    assert!(!registry.lint().unwrap());
  }

  #[test]
  fn a_glob_target_whose_match_has_the_anchor_is_accepted() {
    let registry = Registry::new();
    registry.add_addon(
      "react-router",
      r#"{
        "type": "inject",
        "target": { "type": "glob", "glob": "src/*.tsx" },
        "content": "<BrowserRouter>",
        "after": "{/* anesis:providers-start */}",
        "if_not_found": "error"
      }"#,
    );
    registry.write(
      "addons/react-router/test-fixture/src/main.tsx",
      "{/* anesis:providers-start */}\n<App />\n",
    );

    assert!(registry.lint().unwrap());
  }

  #[test]
  fn an_id_that_does_not_match_its_directory_is_rejected() {
    let registry = Registry::new();
    registry.write(
      "addons/tailwindcss/anesis.addon.json",
      r#"{
  "schema_version": "1",
  "id": "tailwind",
  "name": "tailwind",
  "version": "1.0.0",
  "description": "Directory and id disagree.",
  "author": { "name": "Test", "github": "test" },
  "variants": []
}"#,
    );

    assert!(!registry.lint().unwrap());
  }

  #[test]
  fn a_stack_id_that_does_not_match_its_directory_is_rejected() {
    let registry = Registry::new();
    registry.add_template("react-vite-ts");
    registry.write(
      "stacks/spa/anesis.stack.json",
      r#"{
  "schema_version": "1",
  "id": "react-spa",
  "name": "React SPA",
  "version": "1.0.0",
  "author": { "name": "Test", "github": "test" },
  "template": "react-vite-ts",
  "addons": []
}"#,
    );

    assert!(!registry.lint().unwrap());
  }

  #[test]
  fn an_addon_without_a_fixture_is_not_flagged() {
    let registry = Registry::new();
    registry.add_addon(
      "nest-health",
      r#"{
        "type": "inject",
        "target": { "type": "file", "file": "src/app.module.ts" },
        "content": "HealthModule,",
        "after": "// anesis:top-imports",
        "if_not_found": "error"
      }"#,
    );

    assert!(registry.lint().unwrap());
  }

  #[test]
  fn a_command_with_prerequisites_is_not_checked_against_the_bare_fixture() {
    let registry = Registry::new();
    registry.write(
      "addons/docker/anesis.addon.json",
      r#"{
  "schema_version": "1",
  "id": "docker",
  "name": "docker",
  "version": "1.0.0",
  "description": "A test addon.",
  "author": { "name": "Test", "github": "test" },
  "variants": [
    {
      "when": null,
      "commands": [
        { "name": "install", "steps": [] },
        {
          "name": "add-postgres",
          "requires_commands": ["install"],
          "steps": [
            {
              "type": "inject",
              "target": { "type": "file", "file": "docker-compose.yml" },
              "content": "  postgres:",
              "after": "services:",
              "if_not_found": "error"
            }
          ]
        }
      ]
    }
  ]
}"#,
    );
    registry.write("addons/docker/test-fixture/package.json", "{}");

    assert!(registry.lint().unwrap());
  }

  #[test]
  fn a_stack_referencing_an_unknown_template_is_rejected() {
    let registry = Registry::new();
    registry.add_stack("my-stack", "does-not-exist", "");

    assert!(!registry.lint().unwrap());
  }

  #[test]
  fn an_addon_requiring_an_unknown_addon_is_rejected() {
    let registry = Registry::new();
    registry.write(
      "addons/dependent/anesis.addon.json",
      r#"{
  "schema_version": "1",
  "id": "dependent",
  "name": "dependent",
  "version": "1.0.0",
  "description": "Depends on something that is not published.",
  "author": { "name": "Test", "github": "test" },
  "requires": ["missing-addon"],
  "variants": []
}"#,
    );

    assert!(!registry.lint().unwrap());
  }

  #[test]
  fn duplicate_ids_are_rejected() {
    let registry = Registry::new();
    registry.add_addon("docker", "");
    registry.write(
      "addons/docker-copy/anesis.addon.json",
      r#"{
  "schema_version": "1",
  "id": "docker",
  "name": "docker again",
  "version": "1.0.0",
  "description": "Same id, different directory.",
  "author": { "name": "Test", "github": "test" },
  "variants": []
}"#,
    );

    assert!(!registry.lint().unwrap());
  }

  #[test]
  fn a_manifest_that_fails_the_schema_is_rejected() {
    let registry = Registry::new();
    registry.write(
      "templates/broken/anesis.template.json",
      r#"{
  "name": "broken",
  "version": "1.0.0",
  "anesisVersion": ">=1.0.0",
  "author": { "name": "Test", "github": "test" },
  "repository": { "type": "github", "url": "https://github.com/test/templates", "release": "v1.0.0" },
  "specialization": "frontend",
  "scope": "spaceship",
  "technologies": ["react"],
  "languages": ["typescript"],
  "type": "base",
  "metadata": { "displayName": "broken", "description": "A test template.", "tags": ["test", "fixture"] }
}"#,
    );

    assert!(!registry.lint().unwrap());
  }

  #[test]
  fn an_empty_registry_passes() {
    assert!(Registry::new().lint().unwrap());
  }
}
