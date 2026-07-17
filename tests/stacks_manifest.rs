use std::collections::HashMap;

use anesis::stacks::manifest::*;

#[test]
fn parses_and_defaults_command() {
  let stack: StackManifest = serde_json::from_str(
    r#"{"schema_version":"1","id":"s","name":"S","template":"nest-express",
        "addons":[{"id":"nest-config"},{"id":"nest-prisma","command":"generate","inputs":{"db":"postgres"}}]}"#,
  )
  .unwrap();
  validate(&stack).unwrap();
  assert_eq!(stack.addons[0].command, "install");
  assert_eq!(stack.addons[1].command, "generate");
  assert_eq!(stack.addons[1].inputs.get("db").unwrap(), "postgres");
}

#[test]
fn rejects_empty_template_and_addon_id() {
  let mut stack: StackManifest =
    serde_json::from_str(r#"{"schema_version":"1","id":"s","name":"S","template":"nest"}"#).unwrap();
  stack.template = " ".into();
  assert!(validate(&stack).is_err());
  stack.template = "nest".into();
  stack.addons.push(StackAddon { id: "".into(), command: "install".into(), inputs: HashMap::new() });
  assert!(validate(&stack).is_err());
}
