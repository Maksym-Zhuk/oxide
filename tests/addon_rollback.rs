mod common;

use anesis::addons::lock::{LockEntry, LockFile};
use anesis::addons::runner::{run_addon_command, undo_addon};
use anesis::addons::steps::Rollback;
use assert_fs::prelude::*;
use common::fixture::{Fixture, build};
use std::collections::HashMap;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn reversible_addon_with_steps(id: &str, steps: serde_json::Value) -> serde_json::Value {
  let mut manifest = build::addon_manifest(id, "1.0.0");
  manifest["variants"][0]["commands"][0]["steps"] = steps;
  manifest
}

#[tokio::test]
async fn a_glob_step_that_partially_succeeds_is_rolled_back_through_the_runner() {
  let fx = Fixture::new();
  fx.seed_project();

  fx.project
    .child("files/a.txt")
    .write_str("start\nMARKER\nend\n")
    .unwrap();
  std::fs::write(
    fx.project.path().join("files/b.bin"),
    [0xffu8, 0xfe, 0x00, 0x01],
  )
  .unwrap();

  let manifest = reversible_addon_with_steps(
    "fixture-addon",
    serde_json::json!([
      { "type": "create", "path": "created.txt", "content": "hello\n", "if_exists": "overwrite" },
      {
        "type": "inject",
        "target": { "type": "glob", "glob": "files/*" },
        "content": "INJECTED",
        "after": "MARKER",
        "if_not_found": "error"
      }
    ]),
  );
  fx.install_addon("fixture-addon", &manifest);

  let err = run_addon_command(
    &fx.offline_ctx(),
    "fixture-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .expect_err("the inject step must fail on the binary file");

  assert!(
    err.to_string().contains("step 2"),
    "the error should identify which step failed: {err:#}"
  );
  assert!(
    !fx.project.path().join("created.txt").exists(),
    "step 1's create must have been rolled back"
  );
  let a_txt = std::fs::read_to_string(fx.project.path().join("files/a.txt")).unwrap();
  assert_eq!(
    a_txt, "start\nMARKER\nend\n",
    "step 2's partial rewrite of a.txt must have been rolled back too — this is the \
     defect-1 fix: a bare `?` inside the glob loop used to drop the partial rollback, \
     leaving a.txt permanently modified even though the whole command reported failure"
  );

  let lock = LockFile::load(fx.project.path()).unwrap();
  assert!(
    lock.addons.is_empty(),
    "a fully rolled-back command must not leave a lock entry behind"
  );
}

#[tokio::test]
#[cfg(unix)]
async fn a_lock_save_failure_rolls_back_the_commands_file_changes() {
  let fx = Fixture::new();
  fx.seed_project();

  let lock_path = fx.project.path().join("anesis.lock");
  std::fs::write(&lock_path, r#"{"addons":[]}"#).unwrap();
  std::fs::set_permissions(&lock_path, std::fs::Permissions::from_mode(0o444)).unwrap();

  let manifest = reversible_addon_with_steps(
    "fixture-addon",
    serde_json::json!([
      { "type": "create", "path": "created.txt", "content": "hello\n", "if_exists": "overwrite" }
    ]),
  );
  fx.install_addon("fixture-addon", &manifest);

  let result = run_addon_command(
    &fx.offline_ctx(),
    "fixture-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await;

  std::fs::set_permissions(&lock_path, std::fs::Permissions::from_mode(0o644)).unwrap();

  let err = result.expect_err("a read-only anesis.lock must fail the save");
  assert!(
    format!("{err:#}").contains("Failed to save anesis.lock"),
    "unexpected error: {err:#}"
  );
  assert!(
    !fx.project.path().join("created.txt").exists(),
    "a lock-save failure must roll back the fully-applied file changes — otherwise the \
     project ends up with created.txt on disk but no lock entry to undo it with"
  );

  let lock = LockFile::load(fx.project.path()).unwrap();
  assert!(
    lock.addons.is_empty(),
    "the pre-existing empty lock file must be unchanged, since the save that would have \
     recorded the addon never succeeded"
  );
}

#[test]
#[cfg(unix)]
fn undo_makes_progress_on_every_entry_it_can_and_keeps_the_rest_for_retry() {
  let fx = Fixture::new();
  fx.seed_project();

  let root = fx.project.path().canonicalize().unwrap();
  let x = root.join("x.txt");
  let y = root.join("y.txt");
  let z = root.join("z.txt");
  std::fs::write(&x, "modified-x").unwrap();
  std::fs::write(&y, "modified-y").unwrap();
  std::fs::write(&z, "modified-z").unwrap();

  std::fs::set_permissions(&y, std::fs::Permissions::from_mode(0o444)).unwrap();

  let mut lock = LockFile::load(fx.project.path()).unwrap();
  lock.addons.push(LockEntry {
    id: "fixture-addon".to_string(),
    version: "1.0.0".to_string(),
    variant: "universal".to_string(),
    commands_executed: vec!["install".to_string()],
    journal: vec![
      Rollback::RestoreFile {
        path: x.clone(),
        original: b"orig-x".to_vec(),
      },
      Rollback::RestoreFile {
        path: y.clone(),
        original: b"orig-y".to_vec(),
      },
      Rollback::RestoreFile {
        path: z.clone(),
        original: b"orig-z".to_vec(),
      },
    ],
    inputs: HashMap::new(),
  });
  lock.save(fx.project.path()).unwrap();

  let err = undo_addon("fixture-addon", fx.project.path(), true)
    .expect_err("y.txt's read-only permission must make the undo partially fail");
  assert!(
    err.to_string().contains("partially reverted"),
    "unexpected error: {err:#}"
  );
  assert!(err.to_string().contains("1 change"), "{err:#}");

  assert_eq!(std::fs::read_to_string(&x).unwrap(), "orig-x");
  assert_eq!(std::fs::read_to_string(&z).unwrap(), "orig-z");
  assert_eq!(
    std::fs::read_to_string(&y).unwrap(),
    "modified-y",
    "y.txt's restore failed, so it must be untouched"
  );

  let lock = LockFile::load(fx.project.path()).unwrap();
  let entry = lock
    .addons
    .iter()
    .find(|e| e.id == "fixture-addon")
    .expect("a partially-undone addon must still be recorded in the lock");
  assert_eq!(
    entry.journal.len(),
    1,
    "only the failed entry should remain — x.txt and z.txt already succeeded and must not \
     be retried (RenameFile rollbacks are not idempotent on a second application)"
  );
  assert!(matches!(
    &entry.journal[0],
    Rollback::RestoreFile { path, .. } if path == &y
  ));

  std::fs::set_permissions(&y, std::fs::Permissions::from_mode(0o644)).unwrap();
  undo_addon("fixture-addon", fx.project.path(), true).unwrap();
  assert_eq!(std::fs::read_to_string(&y).unwrap(), "orig-y");

  let lock = LockFile::load(fx.project.path()).unwrap();
  assert!(
    !lock.addons.iter().any(|e| e.id == "fixture-addon"),
    "a fully-undone addon must be removed from the lock"
  );
}
