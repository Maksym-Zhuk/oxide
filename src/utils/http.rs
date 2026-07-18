use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

use crate::{auth::token::get_auth_user, context::AppContext};

#[derive(serde::Deserialize)]
struct Paginated<T> {
  data: Vec<T>,
  total_pages: i64,
}

pub async fn fetch_paginated<T: DeserializeOwned>(
  ctx: &AppContext,
  path: &str,
  label: &str,
) -> Result<Vec<T>> {
  let token = get_auth_user(&ctx.paths.auth).ok().map(|u| u.token);

  let fetch_page = |page: i64| {
    let mut req = ctx
      .client
      .get(format!("{}{path}", ctx.backend_url))
      .query(&[("page", page), ("page_size", 100)]);
    if let Some(token) = &token {
      req = req.bearer_auth(token);
    }
    async move {
      let res: Paginated<T> = req
        .send()
        .await
        .with_context(|| format!("Failed to connect to server for the {label}"))?
        .error_for_status()
        .with_context(|| format!("Server returned an error for the {label}"))?
        .json()
        .await
        .with_context(|| format!("Failed to parse the {label}"))?;
      Ok::<_, anyhow::Error>(res)
    }
  };

  let first = fetch_page(1).await?;
  let total_pages = first.total_pages;
  let mut all = first.data;
  if total_pages > 1 {
    let rest = futures::future::try_join_all((2..=total_pages).map(fetch_page)).await?;
    for page in rest {
      all.extend(page.data);
    }
  }
  Ok(all)
}
