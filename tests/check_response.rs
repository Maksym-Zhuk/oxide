use anesis::utils::errors::check_response;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn one_shot_server(status_line: &'static str, body: &'static str) -> String {
  let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
  let addr = listener.local_addr().unwrap();

  tokio::spawn(async move {
    let (mut socket, _) = listener.accept().await.unwrap();
    let mut buf = [0u8; 1024];
    let _ = socket.read(&mut buf).await;
    let response = format!(
      "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
      body.len()
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;
  });

  format!("http://{addr}/")
}

#[tokio::test]
async fn surfaces_the_server_message_on_a_client_error() {
  let url = one_shot_server(
    "HTTP/1.1 409 Conflict",
    r#"{"status":409,"error":"CONFLICT","message":"addon 'acme/tool' already belongs to another owner"}"#,
  )
  .await;

  let response = reqwest::get(&url).await.unwrap();
  let err = check_response(response, "addon").await.unwrap_err();

  assert_eq!(
    err.to_string(),
    "addon 'acme/tool' already belongs to another owner"
  );
}

#[tokio::test]
async fn falls_back_to_a_classified_message_when_body_has_no_message_field() {
  let url = one_shot_server("HTTP/1.1 404 Not Found", "not json at all").await;

  let response = reqwest::get(&url).await.unwrap();
  let err = check_response(response, "template").await.unwrap_err();

  assert!(err.to_string().contains("template"));
  assert!(err.to_string().to_lowercase().contains("not found"));
}

#[tokio::test]
async fn success_status_passes_the_response_through_unchanged() {
  let url = one_shot_server("HTTP/1.1 200 OK", r#"{"message":"ok"}"#).await;

  let response = reqwest::get(&url).await.unwrap();
  let response = check_response(response, "template").await.unwrap();

  assert_eq!(response.status(), reqwest::StatusCode::OK);
}
