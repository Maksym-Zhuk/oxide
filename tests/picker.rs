mod common;

use common::{match_score_for_tests, smart_match_for_tests, tokenize_for_tests};

#[test]
fn tokens_match_across_separators() {
  assert!(smart_match_for_tests(
    "react-vite-ts starter",
    &tokenize_for_tests("react-ts")
  ));
  assert!(smart_match_for_tests(
    "react-vite-ts",
    &tokenize_for_tests("ts react")
  ));
  assert!(smart_match_for_tests("anything", &tokenize_for_tests("")));
  assert!(!smart_match_for_tests(
    "react-vite-ts",
    &tokenize_for_tests("vue")
  ));
}

#[test]
fn exact_and_prefix_rank_above_fuzzy() {
  let q = "react";
  let toks = tokenize_for_tests(q);
  let exact = match_score_for_tests("react", q, &toks);
  let prefix = match_score_for_tests("react-vite-ts", q, &toks);
  let fuzzy = match_score_for_tests("preact-remix", q, &toks);
  assert!(exact > prefix && prefix > fuzzy);
}

#[test]
fn pick_refuses_to_run_without_a_terminal() {
  let items = vec![anesis::utils::picker::PickItem {
    kind: anesis::utils::picker::ItemKind::Template,
    id: "owner/repo".to_string(),
    name: "react-vite-ts".to_string(),
    meta: String::new(),
    description: "A starter".to_string(),
    haystack: "react-vite-ts".to_string(),
  }];

  let error = anesis::utils::picker::pick(&items, "Select a template", false, "")
    .expect_err("a non-interactive stderr must be refused, not read from");

  assert_eq!(
    anesis::utils::errors::exit_code_for(&error),
    anesis::utils::errors::exit_code::NOT_A_TERMINAL
  );
  assert!(
    error.to_string().contains("interactive terminal"),
    "the message should say what is missing: {error}"
  );
}
