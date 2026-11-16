//! Client requests against a minimal fake device.

use std::sync::{Arc, Mutex};

use bitaxe_api::client::BitaxeClient;
use bitaxe_api::models::{Error, Settings};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Clone, PartialEq)]
struct Request {
    method: String,
    path: String,
    body: Option<Value>,
}

type Requests = Arc<Mutex<Vec<Request>>>;

/// Serves `info` for `GET /api/system/info` and `status` for everything else.
async fn fake_device(info: &'static str, status: u16) -> (String, Requests) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let requests = Requests::default();
    let recorded = requests.clone();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_request(&mut socket).await;

            let (status, body) = if request.method == "GET" && request.path == "/api/system/info" {
                (200, info.to_string())
            } else {
                (status, String::new())
            };
            recorded.lock().unwrap().push(request);

            let response = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });

    (addr, requests)
}

async fn read_request(socket: &mut tokio::net::TcpStream) -> Request {
    let mut data = Vec::new();
    let mut buf = [0u8; 4096];

    loop {
        let n = socket.read(&mut buf).await.unwrap();
        data.extend_from_slice(&buf[..n]);

        let text = String::from_utf8_lossy(&data);
        if let Some(header_end) = text.find("\r\n\r\n") {
            let headers = &text[..header_end];
            let length = headers
                .lines()
                .find_map(|l| {
                    let (name, value) = l.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())?
                })
                .unwrap_or(0);

            if data.len() >= header_end + 4 + length || n == 0 {
                let mut request_line = headers.lines().next().unwrap().split(' ');
                let body = &data[header_end + 4..];
                return Request {
                    method: request_line.next().unwrap().to_string(),
                    path: request_line.next().unwrap().to_string(),
                    body: (!body.is_empty()).then(|| serde_json::from_slice(body).unwrap()),
                };
            }
        }
    }
}

fn fixture(name: &str) -> &'static str {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    Box::leak(std::fs::read_to_string(path).unwrap().into_boxed_str())
}

#[tokio::test]
async fn updates_settings_with_legacy_keys_on_older_firmware() {
    let (addr, requests) = fake_device(fixture("v2.9.0"), 200).await;
    let client = BitaxeClient::new(&addr).unwrap();

    client
        .update_settings(Settings {
            fanspeed: Some(70),
            stratum_user: Some("worker".into()),
            ..Default::default()
        })
        .await
        .unwrap();

    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].method, "PATCH");
    assert_eq!(requests[1].path, "/api/system");
    assert_eq!(
        requests[1].body,
        Some(serde_json::json!({"fanspeed": 70, "stratumUser": "worker"}))
    );
}

#[tokio::test]
async fn updates_pool_slots_on_v2_15() {
    let (addr, requests) = fake_device(fixture("v2.15.4"), 200).await;
    let client = BitaxeClient::new(&addr).unwrap();

    client
        .update_settings(Settings {
            fanspeed: Some(70),
            stratum_user: Some("worker".into()),
            ..Default::default()
        })
        .await
        .unwrap();

    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        requests[1].body,
        Some(serde_json::json!({"manualFanSpeed": 70}))
    );
    assert_eq!(requests[2].method, "PUT");
    assert_eq!(requests[2].path, "/api/system/pools/0");

    let pool = requests[2].body.as_ref().unwrap();
    assert_eq!(pool["stratumUser"], "worker");
    assert_eq!(pool["stratumURL"], "public-pool.io");
    assert_eq!(pool["stratumPassword"], "*****");
    assert!(pool.get("id").is_none());
}

#[tokio::test]
async fn rejected_requests_are_errors() {
    let (addr, _) = fake_device(fixture("v2.15.4"), 400).await;
    let client = BitaxeClient::new(&addr).unwrap();

    let err = client
        .update_settings(Settings {
            overheat_mode: Some(true),
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(matches!(err, Error::ApiClient(status, _) if status.as_u16() == 400));
}
