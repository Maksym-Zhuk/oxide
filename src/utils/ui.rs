use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use comfy_table::{ContentArrangement, Table, presets::UTF8_FULL_CONDENSED};
use indicatif::{ProgressBar, ProgressStyle};

pub fn catalog_table() -> Table {
  let mut table = Table::new();
  table
    .load_preset(UTF8_FULL_CONDENSED)
    .set_content_arrangement(ContentArrangement::Dynamic);
  table
}

pub fn truncate(s: &str, max: usize) -> String {
  if s.chars().count() <= max {
    return s.to_string();
  }
  let short: String = s.chars().take(max.saturating_sub(1)).collect();
  format!("{}…", short.trim_end())
}

static QUIET: AtomicBool = AtomicBool::new(false);

pub fn set_quiet(quiet: bool) {
  QUIET.store(quiet, Ordering::Relaxed);
}

pub fn is_quiet() -> bool {
  QUIET.load(Ordering::Relaxed)
}

pub fn spinner(msg: impl Into<String>) -> ProgressBar {
  if is_quiet() {
    return ProgressBar::hidden();
  }

  let pb = ProgressBar::new_spinner();
  pb.set_style(
    ProgressStyle::with_template("{spinner:.cyan} {msg}")
      .unwrap()
      .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
  );
  pb.set_message(msg.into());
  pb.enable_steady_tick(Duration::from_millis(80));
  pb
}
