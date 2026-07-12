use assert_cmd::Command;
use predicates::str::contains;

fn cmd() -> Command {
  assert_cmd::cargo::cargo_bin_cmd!("anesis")
}

// ── Help / top-level ─────────────────────────────────────────────────────────

#[test]
fn help_flag() {
  cmd()
    .arg("--help")
    .assert()
    .success()
    .stdout(contains("Usage"));
}

#[test]
fn no_args_shows_help() {
  // clap prints help and exits with error when no subcommand is given
  cmd().assert().failure();
}

// ── template subcommand ───────────────────────────────────────────────────────

#[test]
fn template_help() {
  cmd()
    .args(["template", "--help"])
    .assert()
    .success()
    .stdout(contains("(anesis t)"))
    .stdout(contains("install"))
    .stdout(contains("list"))
    .stdout(contains("remove"))
    .stdout(contains("publish"));
}

#[test]
fn template_install_arg_is_optional() {
  // Omitting the name is allowed — `template install` falls back to the
  // interactive picker rather than erroring.
  cmd()
    .args(["template", "install", "--help"])
    .assert()
    .success()
    .stdout(contains("[TEMPLATE_NAME]"))
    .stdout(contains("pick interactively"));
}

#[test]
fn template_remove_missing_arg() {
  cmd()
    .args(["template", "remove"])
    .assert()
    .failure()
    .stderr(contains("TEMPLATE_NAME"));
}

#[test]
fn template_publish_missing_arg() {
  cmd()
    .args(["template", "publish"])
    .assert()
    .failure()
    .stderr(contains("TEMPLATE_URL"));
}

#[test]
fn template_unknown_subcommand() {
  cmd()
    .args(["template", "frobnicate"])
    .assert()
    .failure()
    .stderr(contains("unrecognized subcommand"));
}

// ── addon subcommand ──────────────────────────────────────────────────────────

#[test]
fn addon_help() {
  cmd()
    .args(["addon", "--help"])
    .assert()
    .success()
    .stdout(contains("(anesis a)"))
    .stdout(contains("install"))
    .stdout(contains("list"))
    .stdout(contains("remove"));
}

#[test]
fn addon_install_arg_is_optional() {
  // Omitting the id is allowed — `addon install` falls back to the interactive
  // picker rather than erroring.
  cmd()
    .args(["addon", "install", "--help"])
    .assert()
    .success()
    .stdout(contains("[ADDON_ID]"))
    .stdout(contains("pick interactively"));
}

#[test]
fn addon_remove_missing_arg() {
  cmd()
    .args(["addon", "remove"])
    .assert()
    .failure()
    .stderr(contains("ADDON_ID"));
}

#[test]
fn addon_unknown_subcommand() {
  cmd()
    .args(["addon", "frobnicate"])
    .assert()
    .failure()
    .stderr(contains("unrecognized subcommand"));
}

// ── new subcommand ────────────────────────────────────────────────────────────

#[test]
fn new_help() {
  cmd()
    .args(["new", "--help"])
    .assert()
    .success()
    .stdout(contains("(anesis n)"))
    .stdout(contains("template"));
}

#[test]
fn new_missing_both_args() {
  cmd().arg("new").assert().failure().stderr(contains("NAME"));
}

#[test]
fn new_template_arg_is_optional() {
  // Omitting the template is allowed — `new` falls back to the interactive
  // picker rather than erroring, so the positional renders as optional in help.
  cmd()
    .args(["new", "--help"])
    .assert()
    .success()
    .stdout(contains("[TEMPLATE_NAME]"))
    .stdout(contains("pick interactively"));
}

// ── auth subcommands ──────────────────────────────────────────────────────────

#[test]
fn login_help() {
  cmd()
    .args(["login", "--help"])
    .assert()
    .success()
    .stdout(contains("(anesis in)"));
}

#[test]
fn logout_help() {
  cmd()
    .args(["logout", "--help"])
    .assert()
    .success()
    .stdout(contains("(anesis out)"));
}

#[test]
fn account_help() {
  cmd().args(["account", "--help"]).assert().success();
}

#[test]
fn upgrade_help() {
  cmd()
    .args(["upgrade", "--help"])
    .assert()
    .success()
    .stdout(contains("latest Anesis release"));
}

#[test]
fn use_help() {
  cmd()
    .args(["use", "--help"])
    .assert()
    .success()
    .stdout(contains("anesis use [ADDON_ID] [COMMAND]"));
}

#[test]
fn use_addon_id_is_optional() {
  // Omitting the addon id is allowed — `use` falls back to the interactive
  // addon picker rather than erroring.
  cmd()
    .args(["use", "--help"])
    .assert()
    .success()
    .stdout(contains("[ADDON_ID]"))
    .stdout(contains("pick interactively"));
}

#[test]
fn top_level_addon_execution_is_not_available_anymore() {
  cmd()
    .args(["drizzle", "install"])
    .assert()
    .failure()
    .stderr(contains("unrecognized subcommand"));
}

// ── aliases ───────────────────────────────────────────────────────────────────

#[test]
fn alias_t_for_template() {
  cmd()
    .args(["t", "--help"])
    .assert()
    .success()
    .stdout(contains("install"));
}

#[test]
fn alias_n_for_new() {
  cmd().args(["n", "--help"]).assert().success();
}

#[test]
fn alias_in_for_login() {
  cmd().args(["in", "--help"]).assert().success();
}

#[test]
fn alias_out_for_logout() {
  cmd().args(["out", "--help"]).assert().success();
}

#[test]
fn alias_a_for_addon() {
  cmd()
    .args(["a", "--help"])
    .assert()
    .success()
    .stdout(contains("install"));
}

// ── template subcommand aliases ───────────────────────────────────────────────

#[test]
fn template_update_missing_arg() {
  cmd()
    .args(["template", "update"])
    .assert()
    .failure()
    .stderr(contains("TEMPLATE_URL"));
}

// ── addon subcommand aliases ──────────────────────────────────────────────────

#[test]
fn addon_publish_missing_arg() {
  cmd()
    .args(["addon", "publish"])
    .assert()
    .failure()
    .stderr(contains("ADDON_URL"));
}

#[test]
fn addon_update_missing_arg() {
  cmd()
    .args(["addon", "update"])
    .assert()
    .failure()
    .stderr(contains("ADDON_URL"));
}

// ── version flag ──────────────────────────────────────────────────────────────

#[test]
fn version_flag() {
  cmd()
    .arg("--version")
    .assert()
    .success()
    .stdout(contains("anesis"));
}
