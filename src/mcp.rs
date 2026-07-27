use std::io::{BufRead, Write};
use std::process::{Command, Stdio};

use anyhow::Result;
use serde_json::{Value, json};

pub fn run_mcp() -> Result<()> {
  let stdin = std::io::stdin();
  let stdout = std::io::stdout();
  let mut out = stdout.lock();

  for line in stdin.lock().lines() {
    let line = line?;
    if line.trim().is_empty() {
      continue;
    }
    let Ok(req) = serde_json::from_str::<Value>(&line) else {
      continue;
    };

    let Some(id) = req.get("id").cloned() else {
      continue;
    };
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");

    let reply = match dispatch(method, req.get("params")) {
      Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
      Err((code, message)) => {
        json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
      }
    };

    writeln!(out, "{reply}")?;
    out.flush()?;
  }

  Ok(())
}

fn dispatch(method: &str, params: Option<&Value>) -> Result<Value, (i64, String)> {
  match method {
    "initialize" => Ok(json!({
      "protocolVersion": "2024-11-05",
      "capabilities": { "tools": {} },
      "serverInfo": { "name": "anesis", "version": env!("CARGO_PKG_VERSION") }
    })),
    "ping" => Ok(json!({})),
    "tools/list" => Ok(json!({ "tools": tools_list() })),
    "tools/call" => call_tool(params),
    other => Err((-32601, format!("Method not found: {other}"))),
  }
}

fn call_tool(params: Option<&Value>) -> Result<Value, (i64, String)> {
  let params = params.ok_or((-32602, "Missing params".to_string()))?;
  let name = params
    .get("name")
    .and_then(Value::as_str)
    .ok_or((-32602, "Missing tool name".to_string()))?;
  let args = params
    .get("arguments")
    .cloned()
    .unwrap_or_else(|| json!({}));

  let (text, is_error) = run_tool(name, &args);
  Ok(json!({
    "content": [{ "type": "text", "text": text }],
    "isError": is_error
  }))
}

fn run_tool(name: &str, args: &Value) -> (String, bool) {
  let s = |k: &str| {
    args
      .get(k)
      .and_then(Value::as_str)
      .unwrap_or("")
      .to_string()
  };
  let cwd = args.get("path").and_then(Value::as_str).map(String::from);

  let mut cmd: Vec<String> = match name {
    "search_registry" => {
      let mut v = vec!["search".to_string()];
      let q = s("query");
      if !q.is_empty() {
        v.push(q);
      }
      v.push("--json".to_string());
      v
    }
    "get_manifest" => {
      let kind = s("kind");
      let id = s("id");
      if id.is_empty() {
        return ("'id' is required".to_string(), true);
      }
      match kind.as_str() {
        "template" => vec!["template".into(), "info".into(), id, "--json".into()],
        "addon" => vec!["addon".into(), "info".into(), id, "--json".into()],
        "stack" => vec!["stack".into(), "info".into(), id, "--json".into()],
        other => {
          return (
            format!("Unknown kind '{other}'; use template|addon|stack"),
            true,
          );
        }
      }
    }
    "scaffold_project" => {
      let project = s("name");
      if project.is_empty() {
        return ("'name' is required".to_string(), true);
      }
      let mut v = vec!["new".to_string(), project];
      let stack = s("stack");
      let template = s("template");
      if !stack.is_empty() {
        v.push("--stack".into());
        v.push(stack);
      } else if !template.is_empty() {
        v.push(template);
      } else {
        return ("Provide either 'template' or 'stack'".to_string(), true);
      }
      v.push("--yes".into());
      push_allow_run(&mut v, args);
      push_inputs(&mut v, args);
      v
    }
    "apply_addon" => {
      let id = s("addon_id");
      let command = s("command");
      if id.is_empty() || command.is_empty() {
        return ("'addon_id' and 'command' are required".to_string(), true);
      }
      let mut v = vec!["use".to_string(), id, command, "--yes".into()];
      push_allow_run(&mut v, args);
      push_inputs(&mut v, args);
      v
    }
    "apply_stack" => {
      let project = s("name");
      let stack = s("stack");
      if project.is_empty() || stack.is_empty() {
        return ("'name' and 'stack' are required".to_string(), true);
      }
      let mut v = vec![
        "new".to_string(),
        project,
        "--stack".into(),
        stack,
        "--yes".into(),
      ];
      push_allow_run(&mut v, args);
      push_inputs(&mut v, args);
      v
    }
    "project_status" => vec!["status".into(), "--json".into()],
    other => return (format!("Unknown tool '{other}'"), true),
  };

  run_self(&mut cmd, cwd.as_deref())
}

fn push_allow_run(cmd: &mut Vec<String>, args: &Value) {
  if args.get("allow_run").and_then(Value::as_bool) == Some(true) {
    cmd.push("--allow-run".to_string());
  }
}

fn push_inputs(cmd: &mut Vec<String>, args: &Value) {
  if let Some(obj) = args.get("inputs").and_then(Value::as_object) {
    for (k, v) in obj {
      let value = match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
      };
      cmd.push("--input".to_string());
      cmd.push(format!("{k}={value}"));
    }
  }
}

fn run_self(args: &mut [String], cwd: Option<&str>) -> (String, bool) {
  let exe = match std::env::current_exe() {
    Ok(exe) => exe,
    Err(e) => return (format!("cannot locate anesis binary: {e}"), true),
  };

  let mut cmd = Command::new(exe);
  cmd.args(args.iter()).stdin(Stdio::null());
  if let Some(dir) = cwd {
    cmd.current_dir(dir);
  }

  match cmd.output() {
    Ok(o) => {
      let mut text = String::from_utf8_lossy(&o.stdout).into_owned();
      let err = String::from_utf8_lossy(&o.stderr);
      if !err.trim().is_empty() {
        if !text.is_empty() {
          text.push('\n');
        }
        text.push_str(&err);
      }
      (text.trim().to_string(), !o.status.success())
    }
    Err(e) => (format!("failed to run anesis: {e}"), true),
  }
}

pub fn push_inputs_for_tests(cmd: &mut Vec<String>, args: &Value) {
  push_inputs(cmd, args)
}

#[doc(hidden)]
pub fn push_allow_run_for_tests(cmd: &mut Vec<String>, args: &Value) {
  push_allow_run(cmd, args)
}

#[doc(hidden)]
pub fn tools_list_for_tests() -> Value {
  tools_list()
}

fn tools_list() -> Value {
  json!([
    {
      "name": "search_registry",
      "description": "Search the Anesis registry for templates, addons and stacks. Returns matching entries as JSON.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": { "type": "string", "description": "Filter text (optional; empty lists everything)" }
        }
      }
    },
    {
      "name": "get_manifest",
      "description": "Show the full manifest of a template, addon or stack (description, version, variants, commands, inputs, steps).",
      "inputSchema": {
        "type": "object",
        "properties": {
          "kind": { "type": "string", "enum": ["template", "addon", "stack"] },
          "id": { "type": "string", "description": "Template name / addon id / stack id" }
        },
        "required": ["kind", "id"]
      }
    },
    {
      "name": "scaffold_project",
      "description": "Create a new project from a template (or a stack). Non-interactive. Addon 'run' steps are refused unless allow_run is true.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": { "type": "string", "description": "Project directory to create (use '.' for the current dir)" },
          "template": { "type": "string", "description": "Template name (omit if using a stack)" },
          "stack": { "type": "string", "description": "Stack id (alternative to template)" },
          "inputs": { "type": "object", "description": "Template input values by name", "additionalProperties": true },
          "allow_run": { "type": "boolean", "description": "Permit addon 'run' steps to execute arbitrary shell commands from the registry. Defaults to false; ask the user before setting it." },
          "path": { "type": "string", "description": "Working directory to run in (defaults to the server's cwd)" }
        },
        "required": ["name"]
      }
    },
    {
      "name": "apply_addon",
      "description": "Run an addon command in an existing project. Non-interactive. If the command has a 'run' step it will fail unless allow_run is true.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "addon_id": { "type": "string" },
          "command": { "type": "string", "description": "Addon command to run" },
          "inputs": { "type": "object", "description": "Addon input values by name", "additionalProperties": true },
          "allow_run": { "type": "boolean", "description": "Permit addon 'run' steps to execute arbitrary shell commands from the registry. Defaults to false; ask the user before setting it." },
          "path": { "type": "string", "description": "Project directory to run in" }
        },
        "required": ["addon_id", "command"]
      }
    },
    {
      "name": "apply_stack",
      "description": "Scaffold a new project from a stack (template + ordered addons). Non-interactive. Addon 'run' steps are refused unless allow_run is true.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": { "type": "string", "description": "Project directory to create" },
          "stack": { "type": "string", "description": "Stack id" },
          "inputs": { "type": "object", "additionalProperties": true },
          "allow_run": { "type": "boolean", "description": "Permit addon 'run' steps to execute arbitrary shell commands from the registry. Defaults to false; ask the user before setting it." },
          "path": { "type": "string", "description": "Working directory to run in" }
        },
        "required": ["name", "stack"]
      }
    },
    {
      "name": "project_status",
      "description": "Report the current project's template and applied addons as JSON.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": { "type": "string", "description": "Project directory to inspect" }
        }
      }
    }
  ])
}
