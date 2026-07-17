mod common;

use common::push_inputs_for_tests;
use serde_json::json;

#[test]
fn push_inputs_expands_object_into_flags() {
  let mut cmd = vec!["use".to_string()];
  push_inputs_for_tests(&mut cmd, &json!({ "inputs": { "db": "postgres", "port": 5432 } }));
  assert!(cmd.windows(2).any(|w| w[0] == "--input" && w[1] == "db=postgres"));
  assert!(cmd.windows(2).any(|w| w[0] == "--input" && w[1] == "port=5432"));
}

#[test]
fn push_inputs_noop_without_inputs() {
  let mut cmd = vec!["status".to_string()];
  push_inputs_for_tests(&mut cmd, &json!({ "path": "/tmp" }));
  assert_eq!(cmd, vec!["status".to_string()]);
}
