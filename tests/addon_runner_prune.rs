mod common;

use common::prune_empty_dirs_for_tests;

#[test]
fn stops_exactly_at_the_project_root() {
  let root = assert_fs::TempDir::new().unwrap();
  let nested = root.path().join("src").join("components");
  std::fs::create_dir_all(&nested).unwrap();

  prune_empty_dirs_for_tests(Some(&nested), root.path());

  assert!(
    !root.path().join("src").exists(),
    "empty dirs under the project root should be pruned"
  );
  assert!(root.path().exists(), "the project root itself must survive");
}

#[test]
fn does_not_climb_above_a_path_outside_the_root() {
  let root = assert_fs::TempDir::new().unwrap();
  let unrelated = root.path().join("elsewhere").join("deep");
  std::fs::create_dir_all(&unrelated).unwrap();
  let project_root = root.path().join("project");
  std::fs::create_dir_all(&project_root).unwrap();

  prune_empty_dirs_for_tests(Some(&unrelated), &project_root);

  assert!(
    unrelated.parent().unwrap().exists(),
    "a path outside the project root must be left alone"
  );
  assert!(
    root.path().exists(),
    "must never climb above the temp root itself"
  );
}
