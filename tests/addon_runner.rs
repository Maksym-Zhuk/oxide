mod common;

use common::{is_newer_for_tests, rerun_prompt_message_for_tests};

#[test]
fn is_newer_compares_semver_not_strings() {
  assert!(is_newer_for_tests("0.2.0", "0.1.0"));
  assert!(is_newer_for_tests("0.10.0", "0.9.0"));
  assert!(!is_newer_for_tests("0.1.0", "0.1.0"));
  assert!(!is_newer_for_tests("0.1.0", "0.2.0"));
  assert!(is_newer_for_tests("2024-05", "2024-04"));
}

#[test]
fn rerun_prompt_message_is_none_when_versions_match() {
  let prompt = rerun_prompt_message_for_tests("install", Some("1.0.0"), "1.0.0");
  assert!(prompt.is_none());
}

#[test]
fn rerun_prompt_message_mentions_both_versions_when_version_changed() {
  let prompt = rerun_prompt_message_for_tests("install", Some("1.0.0"), "1.1.0");
  assert_eq!(
    prompt.as_deref(),
    Some(
      "Command 'install' was last run with v1.0.0 of this add-on. A new version (v1.1.0) is available. Re-run it now?"
    )
  );
}

#[test]
fn rerun_prompt_message_is_none_when_no_prior_version_recorded() {
  let prompt = rerun_prompt_message_for_tests("install", None, "1.0.0");
  assert!(
    prompt.is_none(),
    "should not prompt to re-run on a fresh install"
  );
}
