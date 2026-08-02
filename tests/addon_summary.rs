use std::path::PathBuf;

use anesis::addons::steps::Rollback;
use anesis::addons::summary::ChangeSummary;

fn path(s: &str) -> PathBuf {
  PathBuf::from(s)
}

#[test]
fn no_rollbacks_renders_no_changes() {
  let summary = ChangeSummary::from_rollbacks(&[]);
  assert!(summary.is_empty());
  assert_eq!(summary.render(), "No changes.");
}

#[test]
fn delete_created_file_counts_as_created() {
  let rollbacks = vec![Rollback::DeleteCreatedFile {
    path: path("new.txt"),
  }];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  assert_eq!(summary.created, 1);
  assert_eq!(summary.files_changed(), 1);
  assert_eq!(summary.render(), "1 file changed, 1 created");
}

#[test]
fn restore_file_counts_as_modified() {
  let rollbacks = vec![Rollback::restore_file_for_tests(
    path("existing.txt"),
    b"old".to_vec(),
  )];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  assert_eq!(summary.modified, 1);
  assert_eq!(summary.render(), "1 file changed, 1 modified");
}

#[test]
fn rename_file_counts_as_renamed() {
  let rollbacks = vec![Rollback::RenameFile {
    from: path("b.txt"),
    to: path("a.txt"),
  }];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  assert_eq!(summary.renamed, 1);
  assert_eq!(summary.render(), "1 file changed, 1 renamed");
}

#[test]
fn irreversible_run_is_reported_separately_from_file_changes() {
  let rollbacks = vec![Rollback::IrreversibleRun {
    command: "npm install".to_string(),
  }];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  assert_eq!(summary.files_changed(), 0);
  assert_eq!(summary.irreversible_runs, 1);
  assert_eq!(summary.render(), "1 shell command run (not reversible)");
}

#[test]
fn mixed_rollbacks_render_every_category() {
  let rollbacks = vec![
    Rollback::DeleteCreatedFile {
      path: path("new.txt"),
    },
    Rollback::restore_file_for_tests(path("existing.txt"), b"old".to_vec()),
    Rollback::restore_file_for_tests(path("existing2.txt"), b"old2".to_vec()),
    Rollback::RenameFile {
      from: path("b.txt"),
      to: path("a.txt"),
    },
    Rollback::IrreversibleRun {
      command: "npm install".to_string(),
    },
    Rollback::IrreversibleRun {
      command: "npm run build".to_string(),
    },
  ];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  assert_eq!(summary.files_changed(), 4);
  assert_eq!(
    summary.render(),
    "4 files changed, 1 created, 2 modified, 1 renamed, 2 shell commands run (not reversible)"
  );
}
