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
