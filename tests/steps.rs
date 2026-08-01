mod common;

use anesis::addons::{
  manifest::{
    AppendStep, CopyStep, CreateStep, DeleteStep, IfExists, IfNotFound, InjectStep, MoveStep,
    PackagesStep, RenameStep, ReplaceStep, RunStep, Target,
  },
  steps::{
    Rollback, append::execute_append, copy::execute_copy, create::execute_create,
    delete::execute_delete, inject::execute_inject, move_step::execute_move,
    packages::execute_packages, rename::execute_rename, render_string, replace::execute_replace,
    run::execute_run,
  },
};
use assert_fs::prelude::*;
use common::addon_detect_pm_for_tests;

fn empty_ctx() -> tera::Context {
  tera::Context::new()
}

fn ctx_with(key: &str, val: &str) -> tera::Context {
  let mut c = tera::Context::new();
  c.insert(key, val);
  c
}

#[test]
fn create_new_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = CreateStep {
    path: "hello.txt".into(),
    content: "Hello, {{ name }}!".into(),
    if_exists: IfExists::Overwrite,
  };
  let rollbacks = execute_create(&step, dir.path(), &ctx_with("name", "world"), false).unwrap();
  let content = std::fs::read_to_string(dir.path().join("hello.txt")).unwrap();
  assert_eq!(content, "Hello, world!\n");
  assert!(matches!(rollbacks[0], Rollback::DeleteCreatedFile { .. }));
}

#[test]
fn create_overwrites_existing_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("hello.txt").write_str("old content").unwrap();
  let step = CreateStep {
    path: "hello.txt".into(),
    content: "new content".into(),
    if_exists: IfExists::Overwrite,
  };
  let rollbacks = execute_create(&step, dir.path(), &empty_ctx(), false).unwrap();
  let content = std::fs::read_to_string(dir.path().join("hello.txt")).unwrap();
  assert_eq!(content, "new content\n");
  assert!(matches!(rollbacks[0], Rollback::RestoreFile { .. }));
}

#[test]
fn create_skips_existing_file_when_skip() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("hello.txt").write_str("original").unwrap();
  let step = CreateStep {
    path: "hello.txt".into(),
    content: "new content".into(),
    if_exists: IfExists::Skip,
  };
  let rollbacks = execute_create(&step, dir.path(), &empty_ctx(), false).unwrap();
  let content = std::fs::read_to_string(dir.path().join("hello.txt")).unwrap();
  assert_eq!(content, "original");
  assert!(rollbacks.is_empty());
}

#[test]
fn create_nested_dirs() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = CreateStep {
    path: "src/components/Button.tsx".into(),
    content: "export default function Button() {}".into(),
    if_exists: IfExists::Overwrite,
  };
  execute_create(&step, dir.path(), &empty_ctx(), false).unwrap();
  assert!(dir.path().join("src/components/Button.tsx").exists());
}

#[test]
fn inject_after_marker() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.ts")
    .write_str("// imports\nconst app = express();")
    .unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "import cors from 'cors';".into(),
    after: Some("// imports".into()),
    before: None,
    if_not_found: IfNotFound::Error,
  };

  execute_inject(&step, dir.path(), &empty_ctx()).unwrap();

  let result = std::fs::read_to_string(dir.path().join("app.ts")).unwrap();
  let lines: Vec<&str> = result.lines().collect();
  assert_eq!(lines[0], "// imports");
  assert_eq!(lines[1], "import cors from 'cors';");
  assert_eq!(lines[2], "const app = express();");
}

#[test]
fn inject_before_marker() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.ts")
    .write_str("const app = express();")
    .unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "// header".into(),
    after: None,
    before: Some("const app".into()),
    if_not_found: IfNotFound::Error,
  };

  execute_inject(&step, dir.path(), &empty_ctx()).unwrap();

  let lines: Vec<String> = std::fs::read_to_string(dir.path().join("app.ts"))
    .unwrap()
    .lines()
    .map(str::to_string)
    .collect();
  assert_eq!(lines[0], "// header");
  assert_eq!(lines[1], "const app = express();");
}

#[test]
fn inject_marker_whitespace_insensitive() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.module.ts")
    .write_str("@Module({\n  imports: [\n    //  anesis:module-imports\n  ],\n})\n")
    .unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.module.ts".into(),
    },
    content: "    FooModule,".into(),
    after: None,
    before: Some("\t\t// anesis:module-imports".into()),
    if_not_found: IfNotFound::Error,
  };

  execute_inject(&step, dir.path(), &empty_ctx()).unwrap();

  let content = std::fs::read_to_string(dir.path().join("app.module.ts")).unwrap();
  assert!(content.contains("FooModule,\n    //  anesis:module-imports"));
}

#[test]
fn inject_marker_not_found_error() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.ts")
    .write_str("const app = express();")
    .unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "line".into(),
    after: Some("// nonexistent".into()),
    before: None,
    if_not_found: IfNotFound::Error,
  };

  assert!(execute_inject(&step, dir.path(), &empty_ctx()).is_err());
}

#[test]
fn inject_marker_not_found_skip() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.ts")
    .write_str("const app = express();")
    .unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "line".into(),
    after: Some("// nonexistent".into()),
    before: None,
    if_not_found: IfNotFound::Skip,
  };

  execute_inject(&step, dir.path(), &empty_ctx()).unwrap();
  let content = std::fs::read_to_string(dir.path().join("app.ts")).unwrap();
  assert_eq!(content, "const app = express();");
}

#[test]
fn inject_no_marker_prepends() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("line2").unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "line1".into(),
    after: None,
    before: None,
    if_not_found: IfNotFound::Skip,
  };

  execute_inject(&step, dir.path(), &empty_ctx()).unwrap();

  let lines: Vec<String> = std::fs::read_to_string(dir.path().join("app.ts"))
    .unwrap()
    .lines()
    .map(str::to_string)
    .collect();
  assert_eq!(lines[0], "line1");
  assert_eq!(lines[1], "line2");
}

#[test]
fn replace_substitutes_text() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("const PORT = 3000;").unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    find: "3000".into(),
    replace: "4000".into(),
    if_not_found: IfNotFound::Error,
  };

  execute_replace(&step, dir.path(), &empty_ctx()).unwrap();
  let content = std::fs::read_to_string(dir.path().join("app.ts")).unwrap();
  assert_eq!(content, "const PORT = 4000;");
}

#[test]
fn replace_not_found_error() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("const PORT = 3000;").unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    find: "9999".into(),
    replace: "0".into(),
    if_not_found: IfNotFound::Error,
  };

  assert!(execute_replace(&step, dir.path(), &empty_ctx()).is_err());
}

#[test]
fn replace_not_found_skip_leaves_file_unchanged() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("const PORT = 3000;").unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    find: "9999".into(),
    replace: "0".into(),
    if_not_found: IfNotFound::Skip,
  };

  execute_replace(&step, dir.path(), &empty_ctx()).unwrap();
  let content = std::fs::read_to_string(dir.path().join("app.ts")).unwrap();
  assert_eq!(content, "const PORT = 3000;");
}

#[test]
fn replace_rollback_is_restore_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("original").unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    find: "original".into(),
    replace: "replaced".into(),
    if_not_found: IfNotFound::Error,
  };

  let rollbacks = execute_replace(&step, dir.path(), &empty_ctx()).unwrap();
  assert!(matches!(rollbacks[0], Rollback::RestoreFile { .. }));
}

#[test]
fn appends_to_existing_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("line1").unwrap();

  let step = AppendStep {
    target: Target::File {
      file: "file.txt".into(),
    },
    content: "line2".into(),
  };

  execute_append(&step, dir.path(), &empty_ctx()).unwrap();

  let content = std::fs::read_to_string(dir.path().join("file.txt")).unwrap();
  let lines: Vec<&str> = content.lines().collect();
  assert_eq!(lines[0], "line1");
  assert_eq!(lines[1], "line2");
}

#[test]
fn append_rollback_is_restore_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("original").unwrap();

  let step = AppendStep {
    target: Target::File {
      file: "file.txt".into(),
    },
    content: "appended".into(),
  };

  let rollbacks = execute_append(&step, dir.path(), &empty_ctx()).unwrap();
  assert!(matches!(rollbacks[0], Rollback::RestoreFile { .. }));
}

#[test]
fn append_target_file_renders_template_variables() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("user.txt").write_str("original").unwrap();

  let step = AppendStep {
    target: Target::File {
      file: "{{ resource_name }}.txt".into(),
    },
    content: "appended".into(),
  };

  execute_append(&step, dir.path(), &ctx_with("resource_name", "user")).unwrap();

  dir
    .child("user.txt")
    .assert("original\nappended".to_string());
}

#[test]
fn deletes_existing_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("remove-me.txt").write_str("bye").unwrap();

  let step = DeleteStep {
    target: Target::File {
      file: "remove-me.txt".into(),
    },
  };
  execute_delete(&step, dir.path(), &empty_ctx()).unwrap();

  assert!(!dir.path().join("remove-me.txt").exists());
}

#[test]
fn delete_rollback_stores_original_bytes() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("important data").unwrap();

  let step = DeleteStep {
    target: Target::File {
      file: "file.txt".into(),
    },
  };
  let rollbacks = execute_delete(&step, dir.path(), &empty_ctx()).unwrap();

  match &rollbacks[0] {
    Rollback::RestoreFile { original, .. } => assert_eq!(original, b"important data"),
    _ => panic!("expected RestoreFile rollback"),
  }
}

#[test]
fn delete_nonexistent_file_is_ok() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = DeleteStep {
    target: Target::File {
      file: "ghost.txt".into(),
    },
  };
  let rollbacks = execute_delete(&step, dir.path(), &empty_ctx()).unwrap();
  assert!(rollbacks.is_empty());
}

#[test]
fn renames_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("old.txt").write_str("data").unwrap();

  let step = RenameStep {
    from: "old.txt".into(),
    to: "new.txt".into(),
  };
  execute_rename(&step, dir.path(), &tera::Context::new()).unwrap();

  assert!(!dir.path().join("old.txt").exists());
  assert!(dir.path().join("new.txt").exists());
}

#[test]
fn rename_rollback_reverses() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("old.txt").write_str("data").unwrap();

  let step = RenameStep {
    from: "old.txt".into(),
    to: "new.txt".into(),
  };
  let rollbacks = execute_rename(&step, dir.path(), &tera::Context::new()).unwrap();
  let canon_dir = dir.path().canonicalize().unwrap();

  match &rollbacks[0] {
    Rollback::RenameFile { from, to } => {
      assert_eq!(from, &canon_dir.join("new.txt"));
      assert_eq!(to, &canon_dir.join("old.txt"));
    }
    _ => panic!("expected RenameFile rollback"),
  }
}

#[test]
fn rename_source_missing_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = RenameStep {
    from: "ghost.txt".into(),
    to: "new.txt".into(),
  };
  assert!(execute_rename(&step, dir.path(), &tera::Context::new()).is_err());
}

#[test]
fn rename_target_exists_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("a.txt").write_str("a").unwrap();
  dir.child("b.txt").write_str("b").unwrap();

  let step = RenameStep {
    from: "a.txt".into(),
    to: "b.txt".into(),
  };
  assert!(execute_rename(&step, dir.path(), &tera::Context::new()).is_err());
}

#[test]
fn moves_file_to_new_directory() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("data").unwrap();

  let step = MoveStep {
    from: "file.txt".into(),
    to: "subdir/file.txt".into(),
  };
  execute_move(&step, dir.path(), &tera::Context::new()).unwrap();

  assert!(!dir.path().join("file.txt").exists());
  assert!(dir.path().join("subdir/file.txt").exists());
}

#[test]
fn move_creates_destination_dirs() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("data").unwrap();

  let step = MoveStep {
    from: "file.txt".into(),
    to: "a/b/c/file.txt".into(),
  };
  execute_move(&step, dir.path(), &tera::Context::new()).unwrap();
  assert!(dir.path().join("a/b/c/file.txt").exists());
}

#[test]
fn move_source_missing_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = MoveStep {
    from: "ghost.txt".into(),
    to: "dest.txt".into(),
  };
  assert!(execute_move(&step, dir.path(), &tera::Context::new()).is_err());
}

#[test]
fn move_target_exists_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("a.txt").write_str("a").unwrap();
  dir.child("b.txt").write_str("b").unwrap();

  let step = MoveStep {
    from: "a.txt".into(),
    to: "b.txt".into(),
  };
  assert!(execute_move(&step, dir.path(), &tera::Context::new()).is_err());
}

#[test]
fn copy_creates_new_file() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("template.txt").write_str("hello").unwrap();

  let step = CopyStep {
    src: "template.txt".into(),
    dest: "output.txt".into(),
    if_exists: IfExists::Overwrite,
    render: true,
  };

  execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    false,
  )
  .unwrap();

  let content = std::fs::read_to_string(project_dir.path().join("output.txt")).unwrap();
  assert_eq!(content, "hello");
}

#[test]
fn copy_ask_keeps_the_existing_file_when_there_is_nobody_to_ask() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("src.txt").write_str("from addon").unwrap();
  project_dir.child("dst.txt").write_str("mine").unwrap();

  let step = CopyStep {
    src: "src.txt".into(),
    dest: "dst.txt".into(),
    if_exists: IfExists::Ask,
    render: true,
  };

  let rollbacks = execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    true,
  )
  .unwrap();

  assert_eq!(
    std::fs::read_to_string(project_dir.path().join("dst.txt")).unwrap(),
    "mine",
    "the prompt defaults to keeping the file, so non-interactive must too"
  );
  assert!(
    rollbacks.is_empty(),
    "nothing changed, nothing to roll back"
  );
}

#[test]
fn create_ask_keeps_the_existing_file_when_there_is_nobody_to_ask() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("hello.txt").write_str("mine").unwrap();

  let step = CreateStep {
    path: "hello.txt".into(),
    content: "from addon".into(),
    if_exists: IfExists::Ask,
  };

  let rollbacks = execute_create(&step, dir.path(), &empty_ctx(), true).unwrap();

  assert_eq!(
    std::fs::read_to_string(dir.path().join("hello.txt")).unwrap(),
    "mine"
  );
  assert!(rollbacks.is_empty());
}

#[test]
fn copy_new_file_rollback_is_delete_created() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("src.txt").write_str("content").unwrap();

  let step = CopyStep {
    src: "src.txt".into(),
    dest: "dst.txt".into(),
    if_exists: IfExists::Overwrite,
    render: true,
  };

  let rollbacks = execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    false,
  )
  .unwrap();
  assert!(matches!(rollbacks[0], Rollback::DeleteCreatedFile { .. }));
}

#[test]
fn copy_overwrites_existing_file() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir
    .child("template.txt")
    .write_str("new content")
    .unwrap();
  project_dir
    .child("output.txt")
    .write_str("old content")
    .unwrap();

  let step = CopyStep {
    src: "template.txt".into(),
    dest: "output.txt".into(),
    if_exists: IfExists::Overwrite,
    render: true,
  };

  execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    false,
  )
  .unwrap();

  let content = std::fs::read_to_string(project_dir.path().join("output.txt")).unwrap();
  assert_eq!(content, "new content");
}

#[test]
fn copy_overwrite_rollback_is_restore_file() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("src.txt").write_str("new").unwrap();
  project_dir.child("dst.txt").write_str("original").unwrap();

  let step = CopyStep {
    src: "src.txt".into(),
    dest: "dst.txt".into(),
    if_exists: IfExists::Overwrite,
    render: true,
  };

  let rollbacks = execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    false,
  )
  .unwrap();
  assert!(matches!(rollbacks[0], Rollback::RestoreFile { .. }));
}

#[test]
fn copy_skip_leaves_existing_file_unchanged() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("src.txt").write_str("new content").unwrap();
  project_dir.child("dst.txt").write_str("original").unwrap();

  let step = CopyStep {
    src: "src.txt".into(),
    dest: "dst.txt".into(),
    if_exists: IfExists::Skip,
    render: true,
  };

  let rollbacks = execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    false,
  )
  .unwrap();
  assert!(rollbacks.is_empty());

  let content = std::fs::read_to_string(project_dir.path().join("dst.txt")).unwrap();
  assert_eq!(content, "original");
}

#[test]
fn copy_renders_template_vars_in_path_and_content() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir
    .child("controller.ts")
    .write_str("@Controller('{{ resource_name_kebab }}')\nexport class Controller {}")
    .unwrap();

  let step = CopyStep {
    src: "controller.ts".into(),
    dest: "src/{{ resource_name_kebab }}/{{ resource_name_kebab }}.controller.ts".into(),
    if_exists: IfExists::Overwrite,
    render: true,
  };

  execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &ctx_with("resource_name_kebab", "blog-post"),
    false,
  )
  .unwrap();

  let dest = project_dir
    .path()
    .join("src/blog-post/blog-post.controller.ts");
  assert!(dest.exists());
  let content = std::fs::read_to_string(dest).unwrap();
  assert_eq!(
    content,
    "@Controller('blog-post')\nexport class Controller {}"
  );
}

#[test]
fn copy_creates_destination_subdirectory() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("src.txt").write_str("data").unwrap();

  let step = CopyStep {
    src: "src.txt".into(),
    dest: "subdir/dst.txt".into(),
    if_exists: IfExists::Overwrite,
    render: true,
  };

  execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    false,
  )
  .unwrap();
  assert!(project_dir.path().join("subdir/dst.txt").exists());
}

#[test]
fn copy_with_render_false_leaves_content_literal() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir
    .child("workflow.yml")
    .write_str("run: ${{ secrets.TOKEN }}")
    .unwrap();

  let step = CopyStep {
    src: "workflow.yml".into(),
    dest: "workflow.yml".into(),
    if_exists: IfExists::Overwrite,
    render: false,
  };

  execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    false,
  )
  .unwrap();

  let content = std::fs::read_to_string(project_dir.path().join("workflow.yml")).unwrap();
  assert_eq!(content, "run: ${{ secrets.TOKEN }}");
}

#[test]
fn copy_binary_file_preserves_bytes() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  let bytes: Vec<u8> = vec![0xff, 0xfe, 0x00, 0x01, 0x89, 0x50, 0x4e, 0x47];
  addon_dir.child("image.bin").write_binary(&bytes).unwrap();

  let step = CopyStep {
    src: "image.bin".into(),
    dest: "image.bin".into(),
    if_exists: IfExists::Overwrite,
    render: true,
  };

  execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    false,
  )
  .unwrap();

  let written = std::fs::read(project_dir.path().join("image.bin")).unwrap();
  assert_eq!(written, bytes);
}

#[cfg(unix)]
#[test]
fn copy_does_not_propagate_executable_bit() {
  use std::os::unix::fs::PermissionsExt;

  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  let src = addon_dir.child("run.sh");
  src.write_str("#!/bin/sh\necho hi").unwrap();
  std::fs::set_permissions(src.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

  let step = CopyStep {
    src: "run.sh".into(),
    dest: "run.sh".into(),
    if_exists: IfExists::Overwrite,
    render: true,
  };

  execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    false,
  )
  .unwrap();

  let mode = std::fs::metadata(project_dir.path().join("run.sh"))
    .unwrap()
    .permissions()
    .mode();
  assert_eq!(
    mode & 0o111,
    0,
    "copy must never make the destination executable"
  );
}

#[test]
fn copy_without_render_does_not_propagate_executable_bit() {
  use std::os::unix::fs::PermissionsExt;

  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  let src = addon_dir.child("run.sh");
  src.write_binary(b"#!/bin/sh\necho hi").unwrap();
  std::fs::set_permissions(src.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

  let step = CopyStep {
    src: "run.sh".into(),
    dest: "run.sh".into(),
    if_exists: IfExists::Overwrite,
    render: false,
  };

  execute_copy(
    &step,
    addon_dir.path(),
    project_dir.path(),
    &empty_ctx(),
    false,
  )
  .unwrap();

  let mode = std::fs::metadata(project_dir.path().join("run.sh"))
    .unwrap()
    .permissions()
    .mode();
  assert_eq!(
    mode & 0o111,
    0,
    "copy must never make the destination executable"
  );
}

#[test]
fn render_string_renders_multiline_control_blocks() {
  let mut ctx = tera::Context::new();
  ctx.insert("enabled", &true);
  let rendered = render_string("{% if enabled %}\nyes\n{% endif %}", &ctx).unwrap();
  assert_eq!(rendered.trim(), "yes");
}

#[test]
fn render_string_substitutes_variable() {
  let mut ctx = tera::Context::new();
  ctx.insert("env", "production");
  let result = render_string(".env.{{ env }}", &ctx).unwrap();
  assert_eq!(result, ".env.production");
}

#[test]
fn render_string_no_variable_unchanged() {
  let ctx = tera::Context::new();
  let result = render_string("plain-string", &ctx).unwrap();
  assert_eq!(result, "plain-string");
}

#[test]
fn render_string_multiple_variables() {
  let mut ctx = tera::Context::new();
  ctx.insert("prefix", "my");
  ctx.insert("suffix", "app");
  let result = render_string("{{ prefix }}-{{ suffix }}", &ctx).unwrap();
  assert_eq!(result, "my-app");
}

#[test]
fn create_path_traversal_blocked() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = CreateStep {
    path: "../../etc/passwd".into(),
    content: "evil".into(),
    if_exists: IfExists::Overwrite,
  };
  assert!(execute_create(&step, dir.path(), &empty_ctx(), false).is_err());
}

#[cfg(unix)]
#[test]
fn create_target_that_is_a_dangling_symlink_is_blocked() {
  use std::os::unix::fs::symlink;

  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();
  let outside_target = outside.path().join("pwned");

  symlink(&outside_target, dir.path().join("hook")).unwrap();
  assert!(
    !outside_target.exists(),
    "sanity check: the symlink must be dangling"
  );

  let step = CreateStep {
    path: "hook".into(),
    content: "evil".into(),
    if_exists: IfExists::Overwrite,
  };
  let result = execute_create(&step, dir.path(), &empty_ctx(), false);

  assert!(result.is_err(), "dangling symlink escape must be blocked");
  assert!(
    !outside_target.exists(),
    "the write must not have followed the symlink outside the project root"
  );
}

#[cfg(unix)]
#[test]
fn create_under_dangling_symlink_directory_is_blocked() {
  use std::os::unix::fs::symlink;

  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();
  let outside_dir = outside.path().join("pwned-dir");

  symlink(&outside_dir, dir.path().join("scripts")).unwrap();
  assert!(
    !outside_dir.exists(),
    "sanity check: the symlink must be dangling"
  );

  let step = CreateStep {
    path: "scripts/new-file.txt".into(),
    content: "evil".into(),
    if_exists: IfExists::Overwrite,
  };
  let result = execute_create(&step, dir.path(), &empty_ctx(), false);

  assert!(result.is_err(), "dangling symlink escape must be blocked");
  assert!(!outside_dir.exists());
}

#[test]
fn inject_content_uses_template_vars() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("// imports\n").unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "import {{ lib }} from '{{ lib }}';".into(),
    after: Some("// imports".into()),
    before: None,
    if_not_found: IfNotFound::Error,
  };

  let mut ctx = tera::Context::new();
  ctx.insert("lib", "cors");
  execute_inject(&step, dir.path(), &ctx).unwrap();

  let content = std::fs::read_to_string(dir.path().join("app.ts")).unwrap();
  assert!(content.contains("import cors from 'cors';"));
}

#[test]
fn replace_uses_template_var_in_replacement() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("config.ts")
    .write_str("const PORT = 3000;")
    .unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "config.ts".into(),
    },
    find: "3000".into(),
    replace: "{{ port }}".into(),
    if_not_found: IfNotFound::Error,
  };

  let mut ctx = tera::Context::new();
  ctx.insert("port", "4000");
  execute_replace(&step, dir.path(), &ctx).unwrap();

  let content = std::fs::read_to_string(dir.path().join("config.ts")).unwrap();
  assert_eq!(content, "const PORT = 4000;");
}

#[test]
fn append_uses_template_var_in_content() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("line1").unwrap();

  let step = AppendStep {
    target: Target::File {
      file: "file.txt".into(),
    },
    content: "# added by {{ author }}".into(),
  };

  let mut ctx = tera::Context::new();
  ctx.insert("author", "anesis");
  execute_append(&step, dir.path(), &ctx).unwrap();

  let content = std::fs::read_to_string(dir.path().join("file.txt")).unwrap();
  assert!(content.contains("# added by anesis"));
}

#[test]
fn rename_uses_template_vars_in_paths() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("env.example").write_str("data").unwrap();

  let step = RenameStep {
    from: "{{ src }}".into(),
    to: "{{ dest }}".into(),
  };

  let mut ctx = tera::Context::new();
  ctx.insert("src", "env.example");
  ctx.insert("dest", ".env.local");
  execute_rename(&step, dir.path(), &ctx).unwrap();

  assert!(!dir.path().join("env.example").exists());
  assert!(dir.path().join(".env.local").exists());
}

#[test]
fn move_uses_template_vars_in_paths() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("data").unwrap();

  let step = MoveStep {
    from: "{{ src }}".into(),
    to: "subdir/{{ dest }}".into(),
  };

  let mut ctx = tera::Context::new();
  ctx.insert("src", "file.txt");
  ctx.insert("dest", "moved.txt");
  execute_move(&step, dir.path(), &ctx).unwrap();

  assert!(!dir.path().join("file.txt").exists());
  assert!(dir.path().join("subdir/moved.txt").exists());
}

#[test]
fn create_uses_template_var_in_path() {
  let dir = assert_fs::TempDir::new().unwrap();

  let step = CreateStep {
    path: ".env.{{ env }}".into(),
    content: "ENV={{ env }}".into(),
    if_exists: IfExists::Overwrite,
  };

  let mut ctx = tera::Context::new();
  ctx.insert("env", "test");
  execute_create(&step, dir.path(), &ctx, false).unwrap();

  let content = std::fs::read_to_string(dir.path().join(".env.test")).unwrap();
  assert_eq!(content, "ENV=test\n");
}

#[test]
fn append_to_file_with_trailing_newline_no_double_newline() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("line1\n").unwrap();

  let step = AppendStep {
    target: Target::File {
      file: "file.txt".into(),
    },
    content: "line2".into(),
  };

  execute_append(&step, dir.path(), &empty_ctx()).unwrap();

  let content = std::fs::read_to_string(dir.path().join("file.txt")).unwrap();
  let lines: Vec<&str> = content.lines().collect();
  assert_eq!(lines.len(), 2);
  assert_eq!(lines[1], "line2");
}

#[test]
fn packages_detects_by_lockfile_then_manifest() {
  let dir = assert_fs::TempDir::new().unwrap();

  dir.child("package.json").write_str("{}").unwrap();
  assert_eq!(addon_detect_pm_for_tests(dir.path()).unwrap(), "npm");

  dir.child("pnpm-lock.yaml").write_str("").unwrap();
  assert_eq!(addon_detect_pm_for_tests(dir.path()).unwrap(), "pnpm");
}

#[test]
fn packages_no_manifest_is_error() {
  let dir = assert_fs::TempDir::new().unwrap();
  assert!(addon_detect_pm_for_tests(dir.path()).is_err());
}

#[test]
fn run_executes_in_project_root_and_records_irreversible() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = RunStep {
    command: "echo hi > marker.txt".to_string(),
    description: String::new(),
  };
  let rb = execute_run(&step, dir.path(), &empty_ctx(), true, true).unwrap();
  assert!(
    dir.path().join("marker.txt").exists(),
    "command ran in project root"
  );
  assert!(matches!(rb.as_slice(), [Rollback::IrreversibleRun { .. }]));
}

#[test]
fn run_nonzero_exit_is_error() {
  let step = RunStep {
    command: "exit 3".to_string(),
    description: String::new(),
  };
  let out = execute_run(&step, &std::env::temp_dir(), &empty_ctx(), true, true);
  assert!(out.is_err());
}

#[test]
fn delete_target_file_renders_template_path() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("user.txt").write_str("bye").unwrap();

  let step = DeleteStep {
    target: Target::File {
      file: "{{ resource_name }}.txt".into(),
    },
  };
  execute_delete(&step, dir.path(), &ctx_with("resource_name", "user")).unwrap();

  assert!(!dir.path().join("user.txt").exists());
}

#[test]
fn replace_target_file_renders_template_path() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("user.ts")
    .write_str("const PORT = 3000;")
    .unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "{{ resource_name }}.ts".into(),
    },
    find: "3000".into(),
    replace: "4000".into(),
    if_not_found: IfNotFound::Error,
  };
  execute_replace(&step, dir.path(), &ctx_with("resource_name", "user")).unwrap();

  let content = std::fs::read_to_string(dir.path().join("user.ts")).unwrap();
  assert_eq!(content, "const PORT = 4000;");
}

#[test]
fn delete_target_glob_renders_template_pattern() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("blog-post/a.tmp").write_str("a").unwrap();
  dir.child("blog-post/b.tmp").write_str("b").unwrap();
  dir.child("blog-post/keep.txt").write_str("keep").unwrap();

  let step = DeleteStep {
    target: Target::Glob {
      glob: "{{ resource_name_kebab }}/*.tmp".into(),
    },
  };
  execute_delete(
    &step,
    dir.path(),
    &ctx_with("resource_name_kebab", "blog-post"),
  )
  .unwrap();

  assert!(!dir.path().join("blog-post/a.tmp").exists());
  assert!(!dir.path().join("blog-post/b.tmp").exists());
  assert!(dir.path().join("blog-post/keep.txt").exists());
}

#[test]
fn run_is_refused_non_interactively_without_allow_run() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = RunStep {
    command: "echo pwned > pwned.txt".to_string(),
    description: String::new(),
  };

  let err = execute_run(&step, dir.path(), &empty_ctx(), true, false)
    .expect_err("a non-interactive run step must not execute without --allow-run");

  assert!(
    !dir.path().join("pwned.txt").exists(),
    "the command must not have run"
  );
  assert!(
    err.to_string().contains("--allow-run"),
    "the error should say how to opt in: {err}"
  );
}

#[test]
fn run_executes_non_interactively_once_allow_run_is_given() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = RunStep {
    command: "echo ok > allowed.txt".to_string(),
    description: String::new(),
  };

  execute_run(&step, dir.path(), &empty_ctx(), true, true).unwrap();

  assert!(dir.path().join("allowed.txt").exists());
}

#[test]
fn packages_step_is_refused_non_interactively_without_allow_run() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();

  let step = PackagesStep {
    dependencies: vec!["left-pad".to_string()],
    dev_dependencies: vec![],
  };

  let err = execute_packages(&step, dir.path(), true, false)
    .expect_err("a non-interactive packages step must not execute without --allow-run");

  assert!(
    err.to_string().contains("--allow-run"),
    "the error should say how to opt in: {err}"
  );
}

#[test]
fn packages_step_with_no_dependencies_is_a_noop_even_without_allow_run() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();

  let step = PackagesStep {
    dependencies: vec![],
    dev_dependencies: vec![],
  };

  let rollbacks = execute_packages(&step, dir.path(), true, false).unwrap();
  assert!(rollbacks.is_empty());
}
