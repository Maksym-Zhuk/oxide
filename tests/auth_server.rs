use anesis::auth::server::{CALLBACK_PORTS, User, bind_local_auth_server, serve_local_auth_server};

async fn start(state: &str) -> (u16, tokio::task::JoinHandle<anyhow::Result<User>>) {
  let bound = bind_local_auth_server(state.to_string(), "http://localhost:3000")
    .await
    .expect("a callback port should be free");
  let port = bound.port();
  let handle = tokio::spawn(async move { serve_local_auth_server(bound).await });
  (port, handle)
}

fn client() -> reqwest::Client {
  reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()
    .unwrap()
}

async fn get(port: u16, query: &[(&str, &str)]) -> reqwest::Response {
  let url = format!("http://127.0.0.1:{port}/callback");
  for _ in 0..50 {
    match client().get(&url).query(query).send().await {
      Ok(response) => return response,
      Err(_) => tokio::time::sleep(std::time::Duration::from_millis(20)).await,
    }
  }
  panic!("callback listener on port {port} never came up");
}

#[tokio::test]
async fn callback_with_valid_state_returns_user() {
  let state = "validstate0000000000000000000000";
  let (port, handle) = start(state).await;

  get(
    port,
    &[
      ("state", state),
      ("token", "secret-token"),
      ("name", "testuser"),
    ],
  )
  .await;

  let user = handle.await.unwrap().unwrap();
  assert_eq!(user.token, "secret-token");
  assert_eq!(user.name, "testuser");
}

#[tokio::test]
async fn callback_with_invalid_state_redirects_to_error() {
  let (port, handle) = start("correctstate00000000000000000000").await;

  let res = get(
    port,
    &[("state", "wrongstate"), ("token", "tok"), ("name", "user")],
  )
  .await;

  let location = res
    .headers()
    .get("location")
    .and_then(|v| v.to_str().ok())
    .unwrap_or("");
  assert!(
    location.contains("invalid_state"),
    "expected invalid_state redirect, got: {location}"
  );

  handle.abort();
}

#[tokio::test]
async fn callback_without_state_redirects_to_error() {
  let (port, handle) = start("somestate").await;

  let res = get(port, &[("token", "tok"), ("name", "user")]).await;

  let location = res
    .headers()
    .get("location")
    .and_then(|v| v.to_str().ok())
    .unwrap_or("");
  assert!(
    location.contains("missing_state"),
    "expected missing_state redirect, got: {location}"
  );

  handle.abort();
}

#[tokio::test]
async fn a_second_listener_falls_back_to_the_next_port() {
  let first = bind_local_auth_server("a".repeat(32), "http://localhost:3000")
    .await
    .unwrap();
  let second = bind_local_auth_server("b".repeat(32), "http://localhost:3000")
    .await
    .unwrap();

  assert_ne!(
    first.port(),
    second.port(),
    "two listeners must not claim the same port"
  );
  for listener in [&first, &second] {
    assert!(
      CALLBACK_PORTS.contains(&listener.port()),
      "port {} is outside the advertised range",
      listener.port()
    );
  }
}

#[test]
fn every_callback_port_is_unprivileged_and_distinct() {
  assert!(CALLBACK_PORTS.iter().all(|&p| p >= 1024));

  let mut sorted = CALLBACK_PORTS.to_vec();
  sorted.sort_unstable();
  sorted.dedup();
  assert_eq!(sorted.len(), CALLBACK_PORTS.len(), "ports must be distinct");
}
