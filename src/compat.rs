use anyhow::{Result, anyhow};
use semver::{Version, VersionReq};

pub const SUPPORTED_SCHEMA_VERSION: u64 = 1;

fn running_version() -> Result<Version> {
  let full = Version::parse(env!("CARGO_PKG_VERSION"))
    .map_err(|e| anyhow!("anesis has an unparseable own version: {e}"))?;
  Ok(Version::new(full.major, full.minor, full.patch))
}

pub fn check_schema_version(kind: &str, id: &str, declared: &str) -> Result<()> {
  let declared_trimmed = declared.trim();

  let Ok(version) = declared_trimmed.parse::<u64>() else {
    return Err(anyhow!(
      "{kind} '{id}' declares schema_version '{declared_trimmed}', which is not a version number.\n\
       Expected a whole number such as \"{SUPPORTED_SCHEMA_VERSION}\"."
    ));
  };

  if version > SUPPORTED_SCHEMA_VERSION {
    return Err(anyhow!(
      "{kind} '{id}' needs manifest schema version {version}, but anesis {} understands up to {SUPPORTED_SCHEMA_VERSION}.\n\
       An unrecognised manifest shape can silently drop fields, so this is refused rather than applied best-effort.\n\
       Upgrade with `anesis upgrade` and try again.",
      env!("CARGO_PKG_VERSION")
    ));
  }

  Ok(())
}

pub fn check_anesis_version(template_name: &str, requirement: &str) -> Result<()> {
  let requirement = requirement.trim();
  if requirement.is_empty() {
    return Ok(());
  }

  let Ok(req) = VersionReq::parse(requirement) else {
    eprintln!(
      "warning: template '{template_name}' declares anesisVersion '{requirement}', \
       which is not a valid semver range — skipping the compatibility check."
    );
    return Ok(());
  };

  let current = running_version()?;
  if !req.matches(&current) {
    return Err(anyhow!(
      "template '{template_name}' requires anesis {requirement}, but this is anesis {current}.\n\
       Upgrade with `anesis upgrade` and try again."
    ));
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_supported_schema_version_is_accepted() {
    assert!(check_schema_version("addon", "acme/x", "1").is_ok());
  }

  #[test]
  fn an_older_schema_version_is_accepted() {
    assert!(check_schema_version("stack", "acme/s", "0").is_ok());
  }

  #[test]
  fn a_newer_schema_version_is_refused_with_an_upgrade_hint() {
    let err = check_schema_version("addon", "acme/x", "2").unwrap_err();
    let message = err.to_string();
    assert!(message.contains("acme/x"), "{message}");
    assert!(message.contains("schema version 2"), "{message}");
    assert!(message.contains("anesis upgrade"), "{message}");
  }

  #[test]
  fn a_non_numeric_schema_version_is_refused() {
    let err = check_schema_version("addon", "acme/x", "one").unwrap_err();
    assert!(err.to_string().contains("not a version number"));
  }

  #[test]
  fn whitespace_around_the_schema_version_is_tolerated() {
    assert!(check_schema_version("addon", "acme/x", " 1 ").is_ok());
  }

  #[test]
  fn a_satisfied_anesis_version_range_passes() {
    assert!(check_anesis_version("acme/t", ">=0.9.0").is_ok());
  }

  #[test]
  fn an_unsatisfiable_anesis_version_range_is_refused() {
    let err = check_anesis_version("acme/t", ">=99.0.0").unwrap_err();
    let message = err.to_string();
    assert!(message.contains("acme/t"), "{message}");
    assert!(message.contains(">=99.0.0"), "{message}");
    assert!(message.contains("anesis upgrade"), "{message}");
  }

  #[test]
  fn an_empty_anesis_version_is_ignored() {
    assert!(check_anesis_version("acme/t", "   ").is_ok());
  }

  #[test]
  fn an_unparseable_anesis_version_warns_instead_of_failing() {
    assert!(check_anesis_version("acme/t", "not a range").is_ok());
  }

  #[test]
  fn the_running_version_ignores_prerelease_metadata() {
    let version = running_version().unwrap();
    assert!(version.pre.is_empty());
    assert!(version.build.is_empty());
  }
}
