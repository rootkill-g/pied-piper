use serde::{Deserialize, Serialize};
use std::io::{self, Read};

#[derive(Debug, Deserialize)]
struct WsMessage {
    r#type: String,
    id: Option<String>,
    data: serde_json::Value,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct WsResponse {
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

wit_bindgen::generate!({
    world: "ws-echo",
});

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        // Read message from stdin
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Failed to read stdin: {}", e);
            return;
        }

        // Parse WebSocket message
        let ws_msg: WsMessage = match serde_json::from_str(&buffer) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Failed to parse WebSocket message: {}", e);
                return;
            }
        };

        // Handle different message types
        let response = match ws_msg.r#type.as_str() {
            "echo" => {
                // Simple echo
                WsResponse {
                    r#type: "echo_response".to_string(),
                    id: ws_msg.id.clone(),
                    data: ws_msg.data.clone(),
                    metadata: Some(serde_json::json!({
                        "original_type": ws_msg.r#type,
                        "echoed_at": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                    })),
                }
            }
            "uppercase" => {
                // Convert text to uppercase
                let text = ws_msg.data.as_str().unwrap_or("");
                WsResponse {
                    r#type: "uppercase_response".to_string(),
                    id: ws_msg.id.clone(),
                    data: serde_json::json!(text.to_uppercase()),
                    metadata: None,
                }
            }
            "reverse" => {
                // Reverse text
                let text = ws_msg.data.as_str().unwrap_or("");
                let reversed: String = text.chars().rev().collect();
                WsResponse {
                    r#type: "reverse_response".to_string(),
                    id: ws_msg.id.clone(),
                    data: serde_json::json!(reversed),
                    metadata: None,
                }
            }
            "count" => {
                // Count characters
                let text = ws_msg.data.as_str().unwrap_or("");
                WsResponse {
                    r#type: "count_response".to_string(),
                    id: ws_msg.id.clone(),
                    data: serde_json::json!({
                        "length": text.len(),
                        "words": text.split_whitespace().count(),
                        "lines": text.lines().count()
                    }),
                    metadata: None,
                }
            }
            _ => {
                // Unknown type - echo back with info
                WsResponse {
                    r#type: "unknown".to_string(),
                    id: ws_msg.id.clone(),
                    data: serde_json::json!({
                        "error": "Unknown message type",
                        "received_type": ws_msg.r#type,
                        "supported_types": ["echo", "uppercase", "reverse", "count"]
                    }),
                    metadata: None,
                }
            }
        };

        // Output response to stdout
        if let Ok(json) = serde_json::to_string(&response) {
            println!("{}", json);
        }
    }
}
