use anesis::addons::lock::{LockEntry, LockFile};
use anesis::addons::steps::Rollback;

fn sample_entry(id: &str) -> LockEntry {
  LockEntry {
    id: id.to_string(),
    version: "1.0.0".to_string(),
    variant: "universal".to_string(),
    commands_executed: vec![],
    journal: vec![],
    inputs: Default::default(),
  }
}

fn entry_with_journal(id: &str, journal: Vec<Rollback>) -> LockEntry {
  LockEntry {
    journal,
    ..sample_entry(id)
  }
}

#[test]
fn load_returns_default_when_no_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  let lock = LockFile::load(dir.path()).unwrap();
  assert!(lock.addons.is_empty());
}

#[test]
fn save_and_load_roundtrip() {
  let dir = assert_fs::TempDir::new().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  lock.mark_command_executed("drizzle", "install");
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert_eq!(loaded.addons.len(), 1);
  assert_eq!(loaded.addons[0].id, "drizzle");
  assert!(loaded.is_command_executed("drizzle", "install"));
}

#[test]
fn is_command_executed_false_when_no_entry() {
  let lock = LockFile::default();
  assert!(!lock.is_command_executed("drizzle", "install"));
}

#[test]
fn addon_version_returns_current_version_when_entry_exists() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  assert_eq!(lock.addon_version("drizzle"), Some("1.0.0"));
}

#[test]
fn mark_command_executed_adds_once() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  lock.mark_command_executed("drizzle", "install");
  lock.mark_command_executed("drizzle", "install");
  let entry = lock.addons.iter().find(|e| e.id == "drizzle").unwrap();
  assert_eq!(entry.commands_executed.len(), 1);
}

#[test]
fn upsert_entry_adds_new() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  assert_eq!(lock.addons.len(), 1);
}

#[test]
fn upsert_entry_replaces_existing() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  lock.upsert_entry(LockEntry {
    id: "drizzle".to_string(),
    version: "2.0.0".to_string(),
    variant: "nestjs".to_string(),
    commands_executed: vec!["install".to_string()],
    journal: vec![],
    inputs: Default::default(),
  });
  assert_eq!(lock.addons.len(), 1);
  assert_eq!(lock.addons[0].version, "2.0.0");
  assert_eq!(lock.addons[0].variant, "nestjs");
}

#[test]
fn mark_command_executed_noop_when_no_entry() {
  let mut lock = LockFile::default();
  lock.mark_command_executed("unknown", "install");
  assert!(lock.addons.is_empty());
}

#[test]
fn load_rejects_delete_created_file_path_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::DeleteCreatedFile {
      path: outside.path().join("victim-file"),
    }],
  ));
  lock.save(dir.path()).unwrap();

  let result = LockFile::load(dir.path());
  assert!(
    result.is_err(),
    "a journal path outside the project root must be rejected"
  );
}

#[test]
fn load_rejects_restore_file_path_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::RestoreFile {
      path: outside.path().join("authorized_keys"),
      original: b"attacker-controlled content".to_vec(),
    }],
  ));
  lock.save(dir.path()).unwrap();

  let result = LockFile::load(dir.path());
  assert!(result.is_err());
}

#[test]
fn load_rejects_rename_file_when_either_side_is_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();

  let mut lock_from = LockFile::default();
  lock_from.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::RenameFile {
      from: outside.path().join("a"),
      to: dir.path().join("b"),
    }],
  ));
  lock_from.save(dir.path()).unwrap();
  assert!(LockFile::load(dir.path()).is_err());

  let mut lock_to = LockFile::default();
  lock_to.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::RenameFile {
      from: dir.path().join("a"),
      to: outside.path().join("b"),
    }],
  ));
  lock_to.save(dir.path()).unwrap();
  assert!(LockFile::load(dir.path()).is_err());
}

#[test]
fn load_rejects_absolute_path_unrelated_to_project() {
  let dir = assert_fs::TempDir::new().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::DeleteCreatedFile {
      path: std::path::PathBuf::from("/etc/passwd"),
    }],
  ));
  lock.save(dir.path()).unwrap();

  assert!(LockFile::load(dir.path()).is_err());
}

#[test]
fn load_accepts_journal_paths_inside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let canon_root = dir.path().canonicalize().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "fine",
    vec![
      Rollback::DeleteCreatedFile {
        path: canon_root.join("created.txt"),
      },
      Rollback::RestoreFile {
        path: canon_root.join("restored.txt"),
        original: b"content".to_vec(),
      },
      Rollback::RenameFile {
        from: canon_root.join("a.txt"),
        to: canon_root.join("b.txt"),
      },
      Rollback::IrreversibleRun {
        command: "echo hi".to_string(),
      },
    ],
  ));
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert_eq!(loaded.addons[0].journal.len(), 4);
}
