use anesis::stacks::{
  manifest::{Author, StackManifest},
  registry::CatalogStack,
};
use anesis::utils::picker::ItemKind;

fn manifest() -> StackManifest {
  StackManifest {
    schema_version: "1".into(),
    id: "s".into(),
    name: "S".into(),
    description: String::new(),
    version: "1.0.0".into(),
    author: Author {
      name: "Maksym Zhuk".into(),
      github: "anesis-dev".into(),
    },
    template: "nest-express".into(),
    addons: Vec::new(),
  }
}

#[test]
fn to_pick_item_includes_star_count_when_positive() {
  let stack = CatalogStack {
    stack_id: "nest-stack".into(),
    name: "Nest Stack".into(),
    description: "A stack".into(),
    star_count: 42,
    config: manifest(),
  };
  let item = stack.to_pick_item();
  assert!(matches!(item.kind, ItemKind::Stack));
  assert_eq!(item.meta, " · 42★");
}

#[test]
fn to_pick_item_omits_star_count_when_zero() {
  let stack = CatalogStack {
    stack_id: "nest-stack".into(),
    name: "Nest Stack".into(),
    description: "A stack".into(),
    star_count: 0,
    config: manifest(),
  };
  let item = stack.to_pick_item();
  assert_eq!(item.meta, "");
}
