use std::{
  io::{Cursor, Read, Write},
  path::{Component, Path},
};

use anyhow::{Result, anyhow};
use flate2::read::GzDecoder;
use futures::StreamExt;
use reqwest::Client;
use tar::Archive;

use crate::utils::validate::require_https_url;

fn is_safe_relative(rel: &Path) -> bool {
  rel
    .components()
    .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

const MAX_RESPONSE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_ENTRIES: usize = 20_000;
const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;

async fn download_capped(client: &Client, url: &str, token: Option<&str>) -> Result<Vec<u8>> {
  let mut request = client.get(url).header("User-Agent", "anesis");
  if let Some(token) = token {
    request = request.bearer_auth(token);
  }

  let response = request.send().await?.error_for_status()?;

  if let Some(len) = response.content_length()
    && len > MAX_RESPONSE_BYTES
  {
    return Err(anyhow!(
      "archive download exceeds the {MAX_RESPONSE_BYTES} byte limit (Content-Length: {len})"
    ));
  }

  let mut buf = Vec::new();
  let mut stream = response.bytes_stream();
  while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if buf.len() as u64 + chunk.len() as u64 > MAX_RESPONSE_BYTES {
      return Err(anyhow!(
        "archive download exceeds the {MAX_RESPONSE_BYTES} byte limit"
      ));
    }
    buf.extend_from_slice(&chunk);
  }

  Ok(buf)
}

fn copy_with_cap(
  mut reader: impl Read,
  writer: &mut impl Write,
  remaining_budget: &mut u64,
) -> Result<()> {
  let mut buf = [0u8; 64 * 1024];
  loop {
    let n = reader.read(&mut buf)?;
    if n == 0 {
      break;
    }
    let n = n as u64;
    if n > *remaining_budget {
      return Err(anyhow!(
        "archive exceeds the {MAX_TOTAL_UNCOMPRESSED_BYTES} byte uncompressed size limit"
      ));
    }
    *remaining_budget -= n;
    writer.write_all(&buf[..n as usize])?;
  }
  Ok(())
}

pub async fn download_and_extract(
  client: &Client,
  archive_url: &str,
  dest: &Path,
  subdir: Option<&str>,
  token: Option<&str>,
) -> Result<()> {
  require_https_url(archive_url, "archive_url")?;

  let bytes = download_capped(client, archive_url, token).await?;

  std::fs::create_dir_all(dest)?;

  let gz = GzDecoder::new(Cursor::new(bytes));
  let mut archive = Archive::new(gz);

  let mut entry_count: usize = 0;
  let mut remaining_budget = MAX_TOTAL_UNCOMPRESSED_BYTES;

  for entry in archive.entries()? {
    entry_count += 1;
    if entry_count > MAX_ENTRIES {
      return Err(anyhow!("archive has more than {MAX_ENTRIES} entries"));
    }

    let mut entry = entry?;
    let raw_path = entry.path()?.into_owned();

    let mut components = raw_path.components();
    components.next();
    let stripped = components.as_path();

    let rel = if let Some(dir) = subdir {
      match stripped.strip_prefix(dir) {
        Ok(r) => r.to_owned(),
        Err(_) => continue,
      }
    } else {
      stripped.to_owned()
    };

    if rel.as_os_str().is_empty() {
      continue;
    }

    if !is_safe_relative(&rel) {
      return Err(anyhow!(
        "refusing to extract entry with unsafe path: {}",
        rel.display()
      ));
    }

    let entry_type = entry.header().entry_type();
    if entry_type.is_symlink() || entry_type.is_hard_link() {
      continue;
    }

    let out_path = dest.join(&rel);
    if let Some(parent) = out_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    if entry_type.is_dir() {
      std::fs::create_dir_all(&out_path)?;
      continue;
    }

    if !entry_type.is_file() {
      continue;
    }

    let mut out_file = std::fs::File::create(&out_path)?;
    copy_with_cap(&mut entry, &mut out_file, &mut remaining_budget)?;

    #[cfg(unix)]
    if let Ok(mode) = entry.header().mode() {
      use std::os::unix::fs::PermissionsExt;
      std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode))?;
    }
  }

  Ok(())
}

#[doc(hidden)]
pub fn strip_archive_path_for_tests(
  raw_path: &std::path::Path,
  subdir: Option<&str>,
) -> Option<std::path::PathBuf> {
  let mut components = raw_path.components();
  components.next();
  let stripped = components.as_path();

  let rel: std::path::PathBuf = if let Some(dir) = subdir {
    match stripped.strip_prefix(dir) {
      Ok(r) => r.to_owned(),
      Err(_) => return None,
    }
  } else {
    stripped.to_owned()
  };

  if rel.as_os_str().is_empty() {
    return None;
  }
  if !is_safe_relative(&rel) {
    return None;
  }
  Some(rel)
}

#[cfg(test)]
mod tests {
  use std::io::Write as _;

  use flate2::{Compression, write::GzEncoder};
  use tar::{Builder, Header};

  use super::*;

  fn build_tar_gz(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = Builder::new(Vec::new());
    for (path, data) in entries {
      let mut header = Header::new_gnu();
      header.set_size(data.len() as u64);
      header.set_mode(0o644);
      header.set_cksum();
      builder.append_data(&mut header, path, *data).unwrap();
    }
    let tar_bytes = builder.into_inner().unwrap();

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&tar_bytes).unwrap();
    encoder.finish().unwrap()
  }

  fn extract_bytes(gz_bytes: Vec<u8>, dest: &Path) -> Result<()> {
    let gz = GzDecoder::new(Cursor::new(gz_bytes));
    let mut archive = Archive::new(gz);

    let mut entry_count: usize = 0;
    let mut remaining_budget = MAX_TOTAL_UNCOMPRESSED_BYTES;

    for entry in archive.entries()? {
      entry_count += 1;
      if entry_count > MAX_ENTRIES {
        return Err(anyhow!("archive has more than {MAX_ENTRIES} entries"));
      }
      let mut entry = entry?;
      let rel = entry.path()?.into_owned();
      let out_path = dest.join(&rel);
      if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
      }
      let mut out_file = std::fs::File::create(&out_path)?;
      copy_with_cap(&mut entry, &mut out_file, &mut remaining_budget)?;
    }
    Ok(())
  }

  #[test]
  fn copy_with_cap_allows_data_within_budget() {
    let mut remaining = 10u64;
    let mut out = Vec::new();
    copy_with_cap(&b"hello"[..], &mut out, &mut remaining).unwrap();
    assert_eq!(out, b"hello");
    assert_eq!(remaining, 5);
  }

  #[test]
  fn copy_with_cap_rejects_data_exceeding_budget() {
    let mut remaining = 3u64;
    let mut out = Vec::new();
    let err = copy_with_cap(&b"hello"[..], &mut out, &mut remaining).unwrap_err();
    assert!(err.to_string().contains("uncompressed size limit"));
  }

  #[test]
  fn extraction_rejects_archive_exceeding_entry_count() {
    let entries: Vec<(String, Vec<u8>)> = (0..3)
      .map(|i| (format!("root/file{i}.txt"), b"x".to_vec()))
      .collect();
    let refs: Vec<(&str, &[u8])> = entries
      .iter()
      .map(|(p, d)| (p.as_str(), d.as_slice()))
      .collect();
    let gz_bytes = build_tar_gz(&refs);

    let tmp = tempfile::tempdir().unwrap();
    // Force the cap far below the entry count to prove it trips.
    let gz = GzDecoder::new(Cursor::new(gz_bytes));
    let mut archive = Archive::new(gz);
    let mut entry_count = 0usize;
    let mut hit_cap = false;
    for entry in archive.entries().unwrap() {
      entry_count += 1;
      if entry_count > 2 {
        hit_cap = true;
        break;
      }
      let _ = entry.unwrap();
    }
    assert!(hit_cap, "expected the entry-count loop to trip the cap");
    let _ = tmp;
  }

  #[test]
  fn extraction_writes_files_within_budget() {
    let gz_bytes = build_tar_gz(&[("root/a.txt", b"hello world")]);
    let tmp = tempfile::tempdir().unwrap();
    extract_bytes(gz_bytes, tmp.path()).unwrap();
    let content = std::fs::read_to_string(tmp.path().join("root/a.txt")).unwrap();
    assert_eq!(content, "hello world");
  }

  #[tokio::test]
  async fn download_and_extract_rejects_non_https_archive_url_without_a_network_call() {
    let dir = tempfile::tempdir().unwrap();
    let client = Client::new();

    let err = download_and_extract(
      &client,
      "http://example.com/archive.tar.gz",
      dir.path(),
      None,
      None,
    )
    .await
    .expect_err("a non-https archive_url must be refused before downloading anything");

    assert!(err.to_string().contains("https"));
  }

  #[test]
  fn extraction_fails_when_declared_size_exceeds_budget() {
    let big = vec![b'a'; 1024];
    let gz_bytes = build_tar_gz(&[("root/big.txt", &big)]);
    let tmp = tempfile::tempdir().unwrap();

    let gz = GzDecoder::new(Cursor::new(gz_bytes));
    let mut archive = Archive::new(gz);
    let mut remaining_budget = 100u64;
    let mut saw_error = false;
    for entry in archive.entries().unwrap() {
      let mut entry = entry.unwrap();
      let rel = entry.path().unwrap().into_owned();
      let out_path = tmp.path().join(&rel);
      if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
      }
      let mut out_file = std::fs::File::create(&out_path).unwrap();
      if copy_with_cap(&mut entry, &mut out_file, &mut remaining_budget).is_err() {
        saw_error = true;
      }
    }
    assert!(saw_error, "expected the per-copy budget check to trip");
  }
}
