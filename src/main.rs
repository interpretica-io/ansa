//! ansa — a tiny single point where independent agents drop and pick up
//! messages for one another. Start it, and any agent that can speak HTTP can
//! leave a message addressed to another agent and read its own inbox.

mod hub;
mod print;
mod skills;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Notify;
use tokio::time::Instant;

use hub::{Hub, SendRequest};

struct AppState {
    hub: Mutex<Hub>,
    /// Pinged on every send so long-polling inbox readers wake up promptly.
    bell: Notify,
}

const USAGE: &str = "\
usage: ansa                                     start the hub (env: ANSA_ADDR, ANSA_DATA)
       ansa install-skill claude|codex|chatgpt  teach an assistant to use the bus
       ansa --version | --help
";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => serve().await,
        Some("install-skill") => std::process::exit(skills::install(&args[1..])),
        Some("--version" | "-V") => println!("ansa {}", env!("CARGO_PKG_VERSION")),
        Some("--help" | "-h") => print!("{USAGE}"),
        Some(other) => {
            eprintln!("unknown argument: {other}");
            eprint!("{USAGE}");
            std::process::exit(2);
        }
    }
}

async fn serve() {
    let addr: SocketAddr = std::env::var("ANSA_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:7777".to_string())
        .parse()
        .expect("ANSA_ADDR must be host:port");
    let data_path = std::env::var("ANSA_DATA").ok().map(PathBuf::from);

    let hub = Hub::new(data_path.clone());
    let loaded = hub.all().len();
    let state = std::sync::Arc::new(AppState {
        hub: Mutex::new(hub),
        bell: Notify::new(),
    });

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/send", post(send))
        .route("/inbox/:agent", get(inbox))
        .route("/messages", get(messages))
        .route("/agents", get(agents))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("cannot bind {addr}: {e}"));

    eprintln!("ansa listening on http://{addr}");
    match &data_path {
        Some(p) => eprintln!(
            "persisting to {} ({loaded} message(s) replayed)",
            p.display()
        ),
        None => eprintln!("in-memory only (set ANSA_DATA=path to persist)"),
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

/// POST /send  { "from": "...", "to": "...", "body": <any json> }
async fn send(
    State(state): State<std::sync::Arc<AppState>>,
    Json(req): Json<SendRequest>,
) -> impl IntoResponse {
    if req.from.is_empty() || req.to.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "from and to are required"})),
        );
    }
    let msg = {
        let mut hub = state.hub.lock().unwrap();
        hub.send(req)
    };
    print::message(&msg);
    state.bell.notify_waiters();
    (StatusCode::OK, Json(json!({ "id": msg.id, "ts": msg.ts })))
}

#[derive(Deserialize)]
struct InboxQuery {
    /// Seconds to long-poll for new messages before returning empty. Default 0.
    wait: Option<u64>,
    /// Don't advance the read cursor.
    peek: Option<bool>,
    /// Read everything after this id instead of using the stored cursor.
    since: Option<u64>,
}

/// GET /inbox/:agent?wait=&peek=&since=
async fn inbox(
    State(state): State<std::sync::Arc<AppState>>,
    Path(agent): Path<String>,
    Query(q): Query<InboxQuery>,
) -> impl IntoResponse {
    let wait = q.wait.unwrap_or(0);
    let peek = q.peek.unwrap_or(false);
    let deadline = Instant::now() + Duration::from_secs(wait);

    loop {
        // Register interest *before* checking, so a send that lands while we're
        // deciding whether to wait isn't lost.
        let notified = state.bell.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();

        let msgs = {
            let mut hub = state.hub.lock().unwrap();
            hub.inbox(&agent, q.since, peek)
        };

        if !msgs.is_empty() || wait == 0 {
            return Json(json!({ "agent": agent, "messages": msgs }));
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Json(json!({ "agent": agent, "messages": [] }));
        }

        tokio::select! {
            _ = &mut notified => {}
            _ = tokio::time::sleep(remaining) => {
                return Json(json!({ "agent": agent, "messages": [] }));
            }
        }
    }
}

/// GET /messages — the full log, for debugging.
async fn messages(State(state): State<std::sync::Arc<AppState>>) -> impl IntoResponse {
    let hub = state.hub.lock().unwrap();
    Json(json!({ "messages": hub.all() }))
}

/// GET /agents — every name seen so far.
async fn agents(State(state): State<std::sync::Arc<AppState>>) -> impl IntoResponse {
    let hub = state.hub.lock().unwrap();
    Json(json!({ "agents": hub.agents() }))
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    eprintln!("\nansa shutting down");
}
