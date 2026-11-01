use axum::{
    body::Body,
    extract::{ConnectInfo, Query},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use futures_util::StreamExt;
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
const MAX_UPLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(120);
const CHUNK: usize = 128 * 1024;

#[derive(Deserialize)]
struct DlParams {
    bytes: Option<u64>,
}

fn filler() -> Vec<u8> {
    // Deterministic pseudo-random block, avoids compressible zeros.
    let mut v = vec![0u8; CHUNK];
    let mut x: u64 = 0x9E3779B97F4A7C15;
    for b in v.iter_mut() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *b = (x >> 24) as u8;
    }
    v
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../web/index.html"))
}

async fn ping() -> Response {
    (
        StatusCode::NO_CONTENT,
        [(header::CACHE_CONTROL, "no-store")],
    )
        .into_response()
}

async fn download(Query(p): Query<DlParams>) -> Response {
    let bytes = p.bytes.unwrap_or(32 * 1024 * 1024).min(MAX_DOWNLOAD_BYTES);
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(16);

    tokio::spawn(async move {
        let buf = filler();
        let mut sent: u64 = 0;
        while sent < bytes {
            let n = ((bytes - sent) as usize).min(CHUNK);
            let chunk = Bytes::copy_from_slice(&buf[..n]);
            if tx.send(Ok(chunk)).await.is_err() {
                break;
            }
            sent += n as u64;
        }
    });

    Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, bytes.to_string())
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from_stream(ReceiverStream::new(rx)))
        .unwrap()
}

async fn upload(body: Body) -> Response {
    let read_body = async {
        let mut stream = body.into_data_stream();
        let mut total: u64 = 0;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(b) => {
                    total += b.len() as u64;
                    if total > MAX_UPLOAD_BYTES {
                        return Err(());
                    }
                }
                Err(_) => break,
            }
        }
        Ok::<u64, ()>(total)
    };

    match tokio::time::timeout(UPLOAD_TIMEOUT, read_body).await {
        Ok(Ok(total)) => (
            StatusCode::OK,
            [(header::CACHE_CONTROL, "no-store")],
            total.to_string(),
        )
            .into_response(),
        Ok(Err(())) => (
            StatusCode::PAYLOAD_TOO_LARGE,
            [(header::CACHE_CONTROL, "no-store")],
            "payload too large",
        )
            .into_response(),
        Err(_) => (
            StatusCode::REQUEST_TIMEOUT,
            [(header::CACHE_CONTROL, "no-store")],
            "upload timeout",
        )
            .into_response(),
    }
}

async fn ip(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Response {
    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, "no-store")],
        addr.ip().to_string(),
    )
        .into_response()
}

async fn config() -> Response {
    let json = format!(
        "{{\"version\":\"{}\",\"chunk\":{},\"maxBytes\":{}}}",
        env!("CARGO_PKG_VERSION"),
        CHUNK,
        MAX_DOWNLOAD_BYTES
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-store")],
        json,
    )
        .into_response()
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5111);

    let app = Router::new()
        .route("/", get(index))
        .route("/index.html", get(index))
        .route("/api/ping", get(ping))
        .route("/api/download", get(download))
        .route("/api/upload", post(upload))
        .route("/api/ip", get(ip))
        .route("/api/config", get(config));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("bind {addr} failed: {e}"));

    println!("speedtest-rs {} listening on http://{addr}", env!("CARGO_PKG_VERSION"));

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
