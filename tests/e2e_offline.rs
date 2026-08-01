use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::prelude::*;

struct Cli {
  home: TempDir,
  workdir: TempDir,
}

impl Cli {
  fn new() -> Self {
    Self {
      home: TempDir::new().unwrap(),
      workdir: TempDir::new().unwrap(),
    }
  }

  fn command(&self) -> Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("anesis");
    cmd
      .current_dir(self.workdir.path())
      .env("HOME", self.home.path())
      .env("USERPROFILE", self.home.path())
      .env("ANESIS_HOME", self.home.path())
      .env("ANESIS_NO_TELEMETRY", "1")
      .env("ANESIS_BACKEND_URL", "http://127.0.0.1:1")
      .env("ANESIS_RELEASES_API_URL", "http://127.0.0.1:1")
      .env_remove("ANESIS_TOKEN")
      .env_remove("ANESIS_DEBUG");
    cmd
  }

  fn write_template(&self, name: &str) -> assert_fs::TempDir {
    let dir = TempDir::new().unwrap();
    dir
      .child("anesis.template.json")
      .write_str(&format!(
        r#"{{
  "name": "{name}",
  "version": "1.0.0",
  "anesisVersion": ">=0.5.0",
  "author": {{ "name": "anesis", "github": "anesis-dev" }},
  "repository": {{ "url": "https://github.com/anesis-dev/templates" }},
  "specialization": "backend",
  "scope": "cli",
  "technologies": [],
  "languages": [],
  "type": "base",
  "metadata": {{
    "displayName": "{name}",
    "description": "Offline e2e fixture template",
    "tags": []
  }},
  "inputs": []
}}"#
      ))
      .unwrap();
    dir
      .child("README.md")
      .write_str("# hello\n<!-- generated -->\n")
      .unwrap();
    dir
      .child("config.txt.tera")
      .write_str("name={{ project_name }}\nkebab={{ project_name_kebab }}\n")
      .unwrap();
    dir
  }

  fn write_addon(&self, id: &str, steps: &str) -> assert_fs::TempDir {
    let dir = TempDir::new().unwrap();
    dir
      .child("anesis.addon.json")
      .write_str(&format!(
        r#"{{
  "schema_version": "1",
  "id": "{id}",
  "name": "{id}",
  "version": "1.0.0",
  "description": "Offline e2e fixture addon",
  "author": "anesis",
  "requires": [],
  "inputs": [],
  "detect": [],
  "variants": [{{
    "when": null,
    "commands": [{{
      "name": "install",
      "description": "",
      "once": true,
      "requires_commands": [],
      "inputs": [],
      "steps": {steps}
    }}]
  }}]
}}"#
      ))
      .unwrap();
    dir
  }
}

#[test]
fn link_new_use_undo_round_trips_through_the_real_binary_without_any_network_call() {
  let cli = Cli::new();

  let template_dir = cli.write_template("e2e-template");
  cli
    .command()
    .args([
      "template",
      "link",
      &template_dir.path().display().to_string(),
      "--force",
    ])
    .assert()
    .success();

  cli
    .command()
    .args(["new", "proj", "e2e-template", "--yes"])
    .assert()
    .success();

  let project_root = cli.workdir.path().join("proj");
  assert_eq!(
    std::fs::read_to_string(project_root.join("README.md")).unwrap(),
    "# hello\n<!-- generated -->\n"
  );
  assert_eq!(
    std::fs::read_to_string(project_root.join("config.txt")).unwrap(),
    "name=proj\nkebab=proj\n"
  );
  assert!(
    project_root.join("anesis.json").exists(),
    "extract_dir_contents must record the template use in anesis.json"
  );

  let addon_dir = cli.write_addon(
    "e2e-addon",
    r#"[
      { "type": "create", "path": "addon-file.txt", "content": "from addon\n", "if_exists": "overwrite" },
      {
        "type": "inject",
        "target": { "type": "file", "file": "README.md" },
        "content": "<!-- injected -->",
        "after": "generated",
        "if_not_found": "error"
      }
    ]"#,
  );
  cli
    .command()
    .args([
      "addon",
      "link",
      &addon_dir.path().display().to_string(),
      "--force",
    ])
    .assert()
    .success();

  cli
    .command()
    .current_dir(&project_root)
    .args(["use", "e2e-addon", "install", "--yes"])
    .assert()
    .success();

  assert_eq!(
    std::fs::read_to_string(project_root.join("addon-file.txt")).unwrap(),
    "from addon\n"
  );
  let readme_after_addon = std::fs::read_to_string(project_root.join("README.md")).unwrap();
  assert!(readme_after_addon.contains("<!-- injected -->"));

  let lock_before_undo = std::fs::read_to_string(project_root.join("anesis.lock")).unwrap();
  assert!(
    lock_before_undo.contains("e2e-addon"),
    "the applied addon must be recorded in anesis.lock: {lock_before_undo}"
  );

  cli
    .command()
    .current_dir(&project_root)
    .args(["undo", "e2e-addon", "--yes"])
    .assert()
    .success();

  assert!(
    !project_root.join("addon-file.txt").exists(),
    "undo must remove the file the addon created"
  );
  assert_eq!(
    std::fs::read_to_string(project_root.join("README.md")).unwrap(),
    "# hello\n<!-- generated -->\n",
    "undo must byte-for-byte restore the file the addon injected into"
  );
}

#[test]
fn a_failing_addon_command_is_fully_rolled_back_through_the_real_binary() {
  let cli = Cli::new();

  let template_dir = cli.write_template("e2e-template-2");
  cli
    .command()
    .args([
      "template",
      "link",
      &template_dir.path().display().to_string(),
      "--force",
    ])
    .assert()
    .success();
  cli
    .command()
    .args(["new", "proj", "e2e-template-2", "--yes"])
    .assert()
    .success();
  let project_root = cli.workdir.path().join("proj");

  let addon_dir = cli.write_addon(
    "e2e-failing-addon",
    r#"[
      { "type": "create", "path": "should-not-survive.txt", "content": "x\n", "if_exists": "overwrite" },
      {
        "type": "inject",
        "target": { "type": "file", "file": "README.md" },
        "content": "unreachable",
        "after": "this-marker-does-not-exist",
        "if_not_found": "error"
      }
    ]"#,
  );
  cli
    .command()
    .args([
      "addon",
      "link",
      &addon_dir.path().display().to_string(),
      "--force",
    ])
    .assert()
    .success();

  cli
    .command()
    .current_dir(&project_root)
    .args(["use", "e2e-failing-addon", "install", "--yes"])
    .assert()
    .failure();

  assert!(
    !project_root.join("should-not-survive.txt").exists(),
    "step 1's create must have been rolled back when step 2 failed"
  );
  assert!(
    !project_root.join("anesis.lock").exists()
      || !std::fs::read_to_string(project_root.join("anesis.lock"))
        .unwrap()
        .contains("e2e-failing-addon"),
    "a fully rolled-back command must not leave a lock entry"
  );
}
