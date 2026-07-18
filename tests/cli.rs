use assert_cmd::Command;
use predicates::str::contains;

fn cmd() -> Command {
  assert_cmd::cargo::cargo_bin_cmd!("anesis")
}

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
  cmd().assert().failure();
}

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
  cmd()
    .args(["new", "--help"])
    .assert()
    .success()
    .stdout(contains("[TEMPLATE_NAME]"))
    .stdout(contains("pick interactively"));
}

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

#[test]
fn template_update_missing_arg() {
  cmd()
    .args(["template", "update"])
    .assert()
    .failure()
    .stderr(contains("TEMPLATE_URL"));
}

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

#[test]
fn version_flag() {
  cmd()
    .arg("--version")
    .assert()
    .success()
    .stdout(contains("anesis"));
}
