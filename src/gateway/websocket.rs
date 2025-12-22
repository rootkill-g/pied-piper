use anyhow::{Context, Result};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::network::NetworkClient;
use crate::wasm::{ModuleLoader, WasmRuntime, WasmRuntimeConfig};

/// WebSocket message format for WASM communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    /// Message type (e.g., "request", "response", "event", "ping", "pong")
    pub r#type: String,

    /// Request ID for correlation
    pub id: Option<String>,

    /// Payload data
    pub data: serde_json::Value,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// WebSocket handler state
pub struct WsHandler {
    network: NetworkClient,
    loader: Arc<ModuleLoader>,
}

impl WsHandler {
    pub fn new(network: NetworkClient, loader: Arc<ModuleLoader>) -> Self {
        Self { network, loader }
    }

    /// Handle WebSocket upgrade for CID-based modules
    pub async fn handle_ws_cid(self: Arc<Self>, ws: WebSocketUpgrade, cid: String) -> Response {
        info!("WebSocket upgrade requested for CID: {}", cid);

        ws.on_upgrade(move |socket| async move {
            if let Err(e) = self.handle_socket(socket, cid).await {
                error!("WebSocket error: {}", e);
            }
        })
    }

    /// Handle WebSocket upgrade for named applications
    pub async fn handle_ws_app(self: Arc<Self>, ws: WebSocketUpgrade, name: String) -> Response {
        info!("WebSocket upgrade requested for app: {}", name);

        // Resolve name to CID
        let cid = match self.resolve_name(&name).await {
            Ok(Some(cid)) => cid,
            Ok(None) => {
                warn!("Application '{}' not found", name);
                return ws.on_upgrade(|mut socket| async move {
                    let error_msg = WsMessage {
                        r#type: "error".to_string(),
                        id: None,
                        data: serde_json::json!({
                            "error": "Application not found",
                            "name": name
                        }),
                        metadata: None,
                    };

                    if let Ok(json) = serde_json::to_string(&error_msg) {
                        let _ = socket.send(Message::Text(json)).await;
                    }
                    let _ = socket.close().await;
                });
            }
            Err(e) => {
                error!("Error resolving name {}: {}", name, e);
                return ws.on_upgrade(|mut socket| async move {
                    let error_msg = WsMessage {
                        r#type: "error".to_string(),
                        id: None,
                        data: serde_json::json!({
                            "error": "Name resolution failed",
                            "message": e.to_string()
                        }),
                        metadata: None,
                    };

                    if let Ok(json) = serde_json::to_string(&error_msg) {
                        let _ = socket.send(Message::Text(json)).await;
                    }
                    let _ = socket.close().await;
                });
            }
        };

        ws.on_upgrade(move |socket| async move {
            if let Err(e) = self.handle_socket(socket, cid).await {
                error!("WebSocket error: {}", e);
            }
        })
    }

    /// Handle the WebSocket connection
    async fn handle_socket(&self, socket: WebSocket, cid: String) -> Result<()> {
        let (mut sender, mut receiver) = socket.split();

        info!("WebSocket connection established for CID: {}", cid);

        // Send welcome message
        let welcome = WsMessage {
            r#type: "connected".to_string(),
            id: None,
            data: serde_json::json!({
                "message": "WebSocket connected",
                "cid": cid
            }),
            metadata: None,
        };

        sender
            .send(Message::Text(serde_json::to_string(&welcome)?))
            .await?;

        // Fetch module once
        let module_bytes = match self.fetch_module(&cid).await? {
            Some(bytes) => bytes,
            None => {
                let error_msg = WsMessage {
                    r#type: "error".to_string(),
                    id: None,
                    data: serde_json::json!({
                        "error": "Module not found",
                        "cid": cid
                    }),
                    metadata: None,
                };

                sender
                    .send(Message::Text(serde_json::to_string(&error_msg)?))
                    .await?;

                return Ok(());
            }
        };

        // Process messages
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received WebSocket text message: {} bytes", text.len());

                    match self.process_message(&text, &module_bytes).await {
                        Ok(response) => {
                            if let Err(e) = sender.send(Message::Text(response)).await {
                                error!("Failed to send response: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Error processing message: {}", e);

                            let error_response = WsMessage {
                                r#type: "error".to_string(),
                                id: None,
                                data: serde_json::json!({
                                    "error": "Processing failed",
                                    "message": e.to_string()
                                }),
                                metadata: None,
                            };

                            if let Ok(json) = serde_json::to_string(&error_response) {
                                let _ = sender.send(Message::Text(json)).await;
                            }
                        }
                    }
                }
                Ok(Message::Binary(data)) => {
                    debug!("Received WebSocket binary message: {} bytes", data.len());

                    // Convert binary to base64 and process
                    let base64_data =
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);

                    let ws_msg = WsMessage {
                        r#type: "binary".to_string(),
                        id: None,
                        data: serde_json::json!({
                            "data": base64_data,
                            "length": data.len()
                        }),
                        metadata: None,
                    };

                    let text = serde_json::to_string(&ws_msg)?;

                    match self.process_message(&text, &module_bytes).await {
                        Ok(response) => {
                            let _ = sender.send(Message::Text(response)).await;
                        }
                        Err(e) => {
                            error!("Error processing binary message: {}", e);
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    debug!("Received WebSocket ping");
                    let _ = sender.send(Message::Pong(data)).await;
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received WebSocket pong");
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket close received");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        info!("WebSocket connection closed for CID: {}", cid);
        Ok(())
    }

    /// Process a WebSocket message through WASM
    async fn process_message(&self, message: &str, module_bytes: &[u8]) -> Result<String> {
        // Parse the incoming message
        let ws_msg: WsMessage =
            serde_json::from_str(message).context("Invalid WebSocket message format")?;

        debug!("Processing WebSocket message type: {}", ws_msg.r#type);

        // Handle built-in message types
        match ws_msg.r#type.as_str() {
            "ping" => {
                let response = WsMessage {
                    r#type: "pong".to_string(),
                    id: ws_msg.id.clone(),
                    data: serde_json::json!({"timestamp": chrono::Utc::now().to_rfc3339()}),
                    metadata: None,
                };
                return Ok(serde_json::to_string(&response)?);
            }
            "echo" => {
                let response = WsMessage {
                    r#type: "echo".to_string(),
                    id: ws_msg.id.clone(),
                    data: ws_msg.data.clone(),
                    metadata: ws_msg.metadata.clone(),
                };
                return Ok(serde_json::to_string(&response)?);
            }
            _ => {}
        }

        // Pass to WASM module for processing
        let config = WasmRuntimeConfig {
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            max_execution_time: std::time::Duration::from_secs(5),
            enable_async: true,
            enable_wasi: true,
            enable_fuel: true,
            initial_fuel: 1_000_000,
        };

        let runtime = WasmRuntime::new(config)?;

        // Check if component or core module
        let is_component = module_bytes.len() >= 5
            && module_bytes[0] == 0x00
            && module_bytes[1] == 0x61
            && module_bytes[2] == 0x73
            && module_bytes[3] == 0x6d
            && module_bytes[4] == 0x0d;

        if is_component {
            let component = runtime.load_component(module_bytes)?;
            let message_json = serde_json::to_string(&ws_msg)?;
            let mut store = runtime.create_store_with_stdin(message_json.into_bytes())?;

            runtime
                .execute_component_command(&mut store, &component)
                .await?;

            let stdout_bytes = runtime.get_stdout(&store);
            let response_str =
                String::from_utf8(stdout_bytes).context("Invalid UTF-8 in WASM output")?;

            // Try to parse as WsMessage, otherwise wrap it
            match serde_json::from_str::<WsMessage>(&response_str) {
                Ok(msg) => Ok(serde_json::to_string(&msg)?),
                Err(_) => {
                    // Wrap raw output in WsMessage
                    let wrapped = WsMessage {
                        r#type: "response".to_string(),
                        id: ws_msg.id.clone(),
                        data: serde_json::json!({"output": response_str}),
                        metadata: None,
                    };
                    Ok(serde_json::to_string(&wrapped)?)
                }
            }
        } else {
            // Core module path
            let module = runtime.load_module(module_bytes)?;
            let message_json = serde_json::to_string(&ws_msg)?;
            let mut store = runtime.create_store_with_stdin(message_json.into_bytes())?;

            let instance = runtime.instantiate_with_wasi(&mut store, &module).await?;

            // Try to call a WebSocket handler function
            if let Some(func) = instance.get_func(&mut store, "handle_ws_message") {
                func.call_async(&mut store, &[], &mut []).await?;

                let stdout_bytes = runtime.get_stdout(&store);
                let response_str = String::from_utf8(stdout_bytes)?;

                Ok(response_str)
            } else {
                // No handler function, return error
                let error_response = WsMessage {
                    r#type: "error".to_string(),
                    id: ws_msg.id.clone(),
                    data: serde_json::json!({
                        "error": "No WebSocket handler",
                        "message": "Module must export 'handle_ws_message' function"
                    }),
                    metadata: None,
                };
                Ok(serde_json::to_string(&error_response)?)
            }
        }
    }

    /// Resolve application name to CID
    async fn resolve_name(&self, name: &str) -> Result<Option<String>> {
        self.network.resolve_name(name).await
    }

    /// Fetch module bytes from cache or network
    async fn fetch_module(&self, cid: &str) -> Result<Option<Vec<u8>>> {
        use crate::wasm::ModuleCid;

        let module_cid = ModuleCid::new(cid.to_string());

        // Try cache first
        if let Some((_info, bytes)) = self.loader.get_from_cache(&module_cid).await {
            return Ok(Some(bytes.to_vec()));
        }

        // Try network
        if let Some(metadata) = self.network.find_module_by_cid(&module_cid).await? {
            for provider_str in metadata.providers {
                if let Ok(peer_id) = provider_str.parse() {
                    if let Ok(Some(bytes)) = self.network.fetch_module(&module_cid, peer_id).await {
                        return Ok(Some(bytes));
                    }
                }
            }
        }

        Ok(None)
    }
}
