use assert_fs::TempDir;
use assert_fs::prelude::*;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

struct McpSession {
  child: std::process::Child,
  stdin: std::process::ChildStdin,
  stdout: BufReader<std::process::ChildStdout>,
}

impl McpSession {
  fn start(home: &TempDir, workdir: &TempDir) -> Self {
    let mut child = Command::new(env!("CARGO_BIN_EXE_anesis"))
      .arg("mcp")
      .current_dir(workdir.path())
      .env("HOME", home.path())
      .env("USERPROFILE", home.path())
      .env("ANESIS_HOME", home.path())
      .env("ANESIS_NO_TELEMETRY", "1")
      .env("ANESIS_BACKEND_URL", "http://127.0.0.1:1")
      .env("ANESIS_RELEASES_API_URL", "http://127.0.0.1:1")
      .env_remove("ANESIS_TOKEN")
      .env_remove("ANESIS_DEBUG")
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    Self {
      child,
      stdin,
      stdout,
    }
  }

  fn call(&mut self, request: serde_json::Value) -> serde_json::Value {
    let line = serde_json::to_string(&request).unwrap();
    writeln!(self.stdin, "{line}").unwrap();
    self.stdin.flush().unwrap();

    let mut reply_line = String::new();
    self.stdout.read_line(&mut reply_line).unwrap();
    assert!(
      !reply_line.is_empty(),
      "anesis mcp closed stdout without replying to {request}"
    );
    serde_json::from_str(&reply_line)
      .unwrap_or_else(|e| panic!("reply was not valid JSON ({e}): {reply_line}"))
  }

  fn shutdown(mut self) {
    drop(self.stdin);
    let status = self.child.wait().unwrap();
    assert!(status.success(), "anesis mcp exited with {status}");
  }
}

fn write_template(dir: &TempDir, name: &str) {
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
  "metadata": {{ "displayName": "{name}", "description": "MCP e2e fixture", "tags": [] }},
  "inputs": []
}}"#
    ))
    .unwrap();
  dir.child("README.md").write_str("# hello\n").unwrap();
}

#[test]
fn initialize_then_tools_list_over_real_stdio() {
  let home = TempDir::new().unwrap();
  let workdir = TempDir::new().unwrap();
  let mut session = McpSession::start(&home, &workdir);

  let init = session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
  }));
  assert_eq!(init["result"]["serverInfo"]["name"], "anesis");

  let list = session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 2, "method": "tools/list"
  }));
  let tools = list["result"]["tools"].as_array().unwrap();
  assert!(tools.iter().any(|t| t["name"] == "scaffold_project"));

  session.shutdown();
}

#[test]
fn scaffold_project_over_real_stdio_actually_scaffolds_via_run_self() {
  let home = TempDir::new().unwrap();
  let workdir = TempDir::new().unwrap();

  let template_dir = TempDir::new().unwrap();
  write_template(&template_dir, "mcp-e2e-template");
  let status = Command::new(env!("CARGO_BIN_EXE_anesis"))
    .args([
      "template",
      "link",
      &template_dir.path().display().to_string(),
      "--force",
    ])
    .env("HOME", home.path())
    .env("USERPROFILE", home.path())
    .env("ANESIS_HOME", home.path())
    .env("ANESIS_NO_TELEMETRY", "1")
    .env_remove("ANESIS_TOKEN")
    .status()
    .unwrap();
  assert!(status.success());

  let mut session = McpSession::start(&home, &workdir);
  session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
  }));

  let result = session.call(serde_json::json!({
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "scaffold_project",
      "arguments": {
        "name": "mcp-proj",
        "template": "mcp-e2e-template",
        "path": workdir.path().display().to_string()
      }
    }
  }));

  assert_eq!(
    result["result"]["isError"], false,
    "scaffold_project call failed: {result}"
  );
  assert!(
    workdir.path().join("mcp-proj/README.md").exists(),
    "run_self must have actually invoked `anesis new` as a real subprocess"
  );

  session.shutdown();
}
