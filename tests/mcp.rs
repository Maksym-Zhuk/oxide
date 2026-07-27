mod common;

use common::push_inputs_for_tests;
use serde_json::json;

#[test]
fn push_inputs_expands_object_into_flags() {
  let mut cmd = vec!["use".to_string()];
  push_inputs_for_tests(
    &mut cmd,
    &json!({ "inputs": { "db": "postgres", "port": 5432 } }),
  );
  assert!(
    cmd
      .windows(2)
      .any(|w| w[0] == "--input" && w[1] == "db=postgres")
  );
  assert!(
    cmd
      .windows(2)
      .any(|w| w[0] == "--input" && w[1] == "port=5432")
  );
}

#[test]
fn push_inputs_noop_without_inputs() {
  let mut cmd = vec!["status".to_string()];
  push_inputs_for_tests(&mut cmd, &json!({ "path": "/tmp" }));
  assert_eq!(cmd, vec!["status".to_string()]);
}

#[test]
fn allow_run_is_only_forwarded_when_the_caller_asks_for_it() {
  let mut cmd = Vec::new();
  anesis::mcp::push_allow_run_for_tests(&mut cmd, &serde_json::json!({}));
  assert!(
    cmd.is_empty(),
    "the default must not permit shell execution"
  );

  let mut cmd = Vec::new();
  anesis::mcp::push_allow_run_for_tests(&mut cmd, &serde_json::json!({ "allow_run": false }));
  assert!(cmd.is_empty());

  let mut cmd = Vec::new();
  anesis::mcp::push_allow_run_for_tests(&mut cmd, &serde_json::json!({ "allow_run": "true" }));
  assert!(cmd.is_empty(), "only a JSON boolean should opt in");

  let mut cmd = Vec::new();
  anesis::mcp::push_allow_run_for_tests(&mut cmd, &serde_json::json!({ "allow_run": true }));
  assert_eq!(cmd, vec!["--allow-run".to_string()]);
}

#[test]
fn mutating_tools_advertise_the_allow_run_option() {
  let tools = anesis::mcp::tools_list_for_tests();
  let tools = tools.as_array().expect("tools_list is an array");

  for name in ["scaffold_project", "apply_addon", "apply_stack"] {
    let tool = tools
      .iter()
      .find(|t| t["name"] == name)
      .unwrap_or_else(|| panic!("tool {name} is missing"));

    assert_eq!(
      tool["inputSchema"]["properties"]["allow_run"]["type"], "boolean",
      "{name} must expose allow_run"
    );
    assert!(
      tool["description"].as_str().unwrap().contains("allow_run"),
      "{name}'s description should warn about run steps"
    );
  }
}
