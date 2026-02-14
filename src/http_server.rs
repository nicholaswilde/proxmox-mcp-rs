use axum::http::StatusCode;
use axum::{
    extract::{Query, Request, State},
    middleware::{self, Next},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use dashmap::DashMap;
use futures::stream::Stream;
use log::{debug, error, info};
use serde::Deserialize;
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

use crate::mcp::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpServer};

#[derive(Clone)]
struct AppState {
    mcp_server: McpServer,
    sessions: Arc<DashMap<String, mpsc::Sender<Result<Event, Infallible>>>>,
    auth_token: Option<String>,
}

#[derive(Deserialize)]
struct MessageParams {
    session_id: String,
}

pub async fn run_http_server(
    mcp_server: McpServer,
    host: &str,
    port: u16,
    auth_token: Option<String>,
) -> anyhow::Result<()> {
    let state = AppState {
        mcp_server,
        sessions: Arc::new(DashMap::new()),
        auth_token,
    };

    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    info!("Starting HTTP MCP Server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let session_id = Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::channel(100);

    state.sessions.insert(session_id.clone(), tx.clone());

    info!("New SSE session connected: {}", session_id);

    // Send the endpoint event immediately
    let endpoint_url = format!("/message?session_id={}", session_id);
    let _ = tx
        .send(Ok(Event::default().event("endpoint").data(endpoint_url)))
        .await;

    // Create a stream that removes the session on drop
    let stream = ReceiverStream::new(rx);

    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn message_handler(
    State(state): State<AppState>,
    Query(params): Query<MessageParams>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let session_id = params.session_id;

    let tx = if let Some(sender) = state.sessions.get(&session_id) {
        sender.clone()
    } else {
        return (axum::http::StatusCode::NOT_FOUND, "Session not found").into_response();
    };

    let mcp = state.mcp_server.clone();

    tokio::spawn(async move {
        let req_id = req.id.clone();
        debug!(
            "Received HTTP request for session {}: {:?}",
            session_id, req
        );

        let resp = mcp.handle_request(req).await;

        if let Some(id) = req_id {
            let json_resp = match resp {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(id),
                    result: Some(result),
                    error: None,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(id),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: e.to_string(),
                        data: None,
                    }),
                },
            };

            if let Ok(data) = serde_json::to_string(&json_resp) {
                // Send response as 'message' event
                if let Err(e) = tx
                    .send(Ok(Event::default().event("message").data(data)))
                    .await
                {
                    error!("Failed to send SSE event to session {}: {}", session_id, e);
                }
            }

            // Check for notifications
            if mcp.check_notification() {
                let notification = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/tools/list_changed"
                });
                if let Ok(data) = serde_json::to_string(&notification) {
                    if let Err(e) = tx
                        .send(Ok(Event::default().event("message").data(data)))
                        .await
                    {
                        error!(
                            "Failed to send notification to session {}: {}",
                            session_id, e
                        );
                    }
                }
            }
        }
    });

    // Return 202 Accepted immediately
    (axum::http::StatusCode::ACCEPTED, "Accepted").into_response()
}

async fn auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(ref token) = state.auth_token {
        // 1. Check Header
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str == format!("Bearer {}", token) {
                    return Ok(next.run(req).await);
                }
            }
        }

        // 2. Check Query Param
        if let Some(query) = req.uri().query() {
            let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

            if let Some(t) = params.get("token") {
                if t == token {
                    return Ok(next.run(req).await);
                }
            }
        }

        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt; // for oneshot

    fn create_test_app(token: Option<String>) -> Router {
        // Create a dummy McpServer
        // We can't easily create a real McpServer without network, but for middleware testing
        // we just need the State to exist.
        // We'll create a dummy ProxmoxClient (it won't be used by middleware).
        let client = crate::proxmox::ProxmoxClient::new("localhost", 8006, true).unwrap();
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let mcp_server = McpServer::new(clients, "default".to_string(), false);

        let state = AppState {
            mcp_server,
            sessions: Arc::new(DashMap::new()),
            auth_token: token,
        };

        Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_auth_no_token_configured() {
        let app = create_test_app(None);

        let req = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_token_missing() {
        let app = create_test_app(Some("secret".to_string()));

        let req = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_token_header_valid() {
        let app = create_test_app(Some("secret".to_string()));

        let req = Request::builder()
            .uri("/test")
            .header("Authorization", "Bearer secret")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_token_header_invalid() {
        let app = create_test_app(Some("secret".to_string()));

        let req = Request::builder()
            .uri("/test")
            .header("Authorization", "Bearer wrong")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_token_query_valid() {
        let app = create_test_app(Some("secret".to_string()));

        let req = Request::builder()
            .uri("/test?token=secret")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_token_query_invalid() {
        let app = create_test_app(Some("secret".to_string()));

        let req = Request::builder()
            .uri("/test?token=wrong")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_sse_handler() {
        let client = crate::proxmox::ProxmoxClient::new("localhost", 8006, true).unwrap();
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let mcp_server = McpServer::new(clients, "default".to_string(), false);

        let state = AppState {
            mcp_server,
            sessions: Arc::new(DashMap::new()),
            auth_token: None,
        };

        let app = Router::new()
            .route("/sse", get(sse_handler))
            .with_state(state);

        let req = Request::builder().uri("/sse").body(Body::empty()).unwrap();
        let response = app.oneshot(req).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/event-stream"
        );
    }

    #[tokio::test]
    async fn test_message_handler_not_found() {
        let client = crate::proxmox::ProxmoxClient::new("localhost", 8006, true).unwrap();
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let mcp_server = McpServer::new(clients, "default".to_string(), false);

        let state = AppState {
            mcp_server,
            sessions: Arc::new(DashMap::new()),
            auth_token: None,
        };

        let app = Router::new()
            .route("/message", post(message_handler))
            .with_state(state);

        let req = Request::builder()
            .uri("/message?session_id=unknown")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "ping",
                    "id": 1
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_message_handler_success() {
        let client = crate::proxmox::ProxmoxClient::new("localhost", 8006, true).unwrap();
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let mcp_server = McpServer::new(clients, "default".to_string(), false);

        let sessions = Arc::new(DashMap::new());
        let (tx, mut rx) = mpsc::channel(100);
        let session_id = "test-session".to_string();
        sessions.insert(session_id.clone(), tx);

        let state = AppState {
            mcp_server,
            sessions,
            auth_token: None,
        };

        let app = Router::new()
            .route("/message", post(message_handler))
            .with_state(state);

        let req = Request::builder()
            .uri(format!("/message?session_id={}", session_id))
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "ping",
                    "id": 1
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        // Wait for spawned task to send message
        let msg = rx.recv().await.unwrap().unwrap();
        // Event is opaque, but receiving it confirms the path works
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("data"));
    }

    #[tokio::test]
    async fn test_message_handler_notification() {
        let client = crate::proxmox::ProxmoxClient::new("localhost", 8006, true).unwrap();
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let mcp_server = McpServer::new(clients, "default".to_string(), false);

        let sessions = Arc::new(DashMap::new());
        let (tx, mut rx) = mpsc::channel(100);
        let session_id = "test-session".to_string();
        sessions.insert(session_id.clone(), tx);

        let state = AppState {
            mcp_server,
            sessions,
            auth_token: None,
        };

        let app = Router::new()
            .route("/message", post(message_handler))
            .with_state(state);

        let req = Request::builder()
            .uri(format!("/message?session_id={}", session_id))
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": "load_all_tools",
                        "arguments": {}
                    },
                    "id": 1
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        // Receive response
        let _ = rx.recv().await.unwrap().unwrap();
        // Receive notification
        let msg = rx.recv().await.unwrap().unwrap();
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("list_changed"));
    }

    #[tokio::test]
    async fn test_message_handler_invalid_json() {
        let client = crate::proxmox::ProxmoxClient::new("localhost", 8006, true).unwrap();
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let mcp_server = McpServer::new(clients, "default".to_string(), false);

        let state = AppState {
            mcp_server,
            sessions: Arc::new(DashMap::new()),
            auth_token: None,
        };

        let app = Router::new()
            .route("/message", post(message_handler))
            .with_state(state);

        let req = Request::builder()
            .uri("/message?session_id=test")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from("invalid json"))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_message_handler_error() {
        let client = crate::proxmox::ProxmoxClient::new("localhost", 8006, true).unwrap();
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let mcp_server = McpServer::new(clients, "default".to_string(), false);

        let sessions = Arc::new(DashMap::new());
        let (tx, mut rx) = mpsc::channel(100);
        let session_id = "test-session".to_string();
        sessions.insert(session_id.clone(), tx);

        let state = AppState {
            mcp_server,
            sessions,
            auth_token: None,
        };

        let app = Router::new()
            .route("/message", post(message_handler))
            .with_state(state);

        // Request a tool that doesn't exist to trigger error
        let req = Request::builder()
            .uri(format!("/message?session_id={}", session_id))
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": "non_existent_tool",
                        "arguments": {}
                    },
                    "id": 1
                }))
                .unwrap(),
            ))
            .unwrap();

        let _ = app.oneshot(req).await.unwrap();

        let msg = rx.recv().await.unwrap().unwrap();
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("error"));
        assert!(debug_str.contains("-32603"));
    }
}
