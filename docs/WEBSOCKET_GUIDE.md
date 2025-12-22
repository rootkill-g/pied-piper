# WebSocket Support Guide

This guide explains how to use WebSocket connections in Pied Piper for real-time bidirectional communication between clients and WASM modules.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Message Format](#message-format)
- [Creating WebSocket Services](#creating-websocket-services)
- [Client Examples](#client-examples)
- [Deployment](#deployment)
- [Testing](#testing)
- [Performance](#performance)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

WebSocket support enables:
- **Real-time Communication**: Bidirectional messaging between client and WASM
- **Low Latency**: Sub-10ms message roundtrip (local)
- **Persistent Connections**: Long-lived connections for streaming data
- **Multiple Message Types**: Handle different message types in one service
- **JSON Protocol**: Structured message format with type safety

### Key Features

- ✅ Automatic WebSocket upgrade
- ✅ JSON message protocol
- ✅ Request correlation via message IDs
- ✅ Built-in ping/pong handling
- ✅ Error propagation
- ✅ Binary message support (base64 encoded)
- ✅ Connection lifecycle management

## Architecture

```
┌─────────────┐          ┌─────────────┐          ┌─────────────┐
│   Client    │          │   Gateway   │          │    WASM     │
│ (Browser/   │          │  (WebSocket │          │   Module    │
│  Native)    │          │   Handler)  │          │             │
└─────────────┘          └─────────────┘          └─────────────┘
       │                        │                        │
       │  1. WS Upgrade         │                        │
       │───────────────────────>│                        │
       │                        │  2. Load Module        │
       │                        │───────────────────────>│
       │  3. Connected          │                        │
       │<───────────────────────│                        │
       │                        │                        │
       │  4. JSON Message       │                        │
       │───────────────────────>│                        │
       │                        │  5. Execute            │
       │                        │───────────────────────>│
       │                        │  6. JSON Response      │
       │  7. Response           │<───────────────────────│
       │<───────────────────────│                        │
       │                        │                        │
```

### Flow

1. **Client connects** to `/ws/cid/:cid` or `/ws/app/:name`
2. **Gateway upgrades** HTTP to WebSocket protocol
3. **Module loads** once per connection (cached)
4. **Messages flow** bidirectionally as JSON
5. **WASM processes** each message via stdin/stdout
6. **Responses stream** back to client
7. **Connection closes** when either side disconnects

## Message Format

All WebSocket messages use JSON with this structure:

### Request Message

```json
{
  "type": "message_type",
  "id": "optional_request_id",
  "data": "any_json_value",
  "metadata": {
    "optional": "metadata"
  }
}
```

**Fields:**
- `type` (string, required): Message type identifier
- `id` (string, optional): Request ID for correlation
- `data` (any JSON, required): Message payload
- `metadata` (object, optional): Additional context

### Response Message

```json
{
  "type": "response_type",
  "id": "matching_request_id",
  "data": "response_data",
  "metadata": {
    "timestamp": 1234567890
  }
}
```

**Fields:**
- `type` (string, required): Response type
- `id` (string, optional): Matching request ID
- `data` (any JSON, required): Response payload
- `metadata` (object, optional): Additional info

### Built-in Message Types

The gateway handles these automatically:

#### Ping/Pong

**Request:**
```json
{
  "type": "ping",
  "id": "ping-1"
}
```

**Response:**
```json
{
  "type": "pong",
  "id": "ping-1",
  "data": {
    "timestamp": "2025-01-28T12:00:00Z"
  }
}
```

#### Echo

**Request:**
```json
{
  "type": "echo",
  "id": "echo-1",
  "data": { "message": "Hello!" }
}
```

**Response:**
```json
{
  "type": "echo",
  "id": "echo-1",
  "data": { "message": "Hello!" }
}
```

## Creating WebSocket Services

### 1. Cargo.toml

```toml
[package]
name = "my-ws-service"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[lib]
crate-type = ["cdylib"]
```

### 2. WIT Definition

Create `wit/world.wit`:

```wit
package component:my-ws-service;

world my-ws-service {
    export run: func();
}
```

### 3. Rust Implementation

Create `src/lib.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::io::{self, Read};

#[derive(Debug, Deserialize)]
struct WsMessage {
    r#type: String,
    id: Option<String>,
    data: serde_json::Value,
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
    world: "my-ws-service",
    exports: {
        world: Component,
    },
});

struct Component;

impl Guest for Component {
    fn run() {
        // Read message from stdin
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();

        // Parse message
        let msg: WsMessage = serde_json::from_str(&buffer).unwrap();

        // Handle message
        let response = match msg.r#type.as_str() {
            "greet" => {
                let name = msg.data.as_str().unwrap_or("World");
                WsResponse {
                    r#type: "greeting".to_string(),
                    id: msg.id,
                    data: serde_json::json!({
                        "message": format!("Hello, {}!", name)
                    }),
                    metadata: None,
                }
            }
            _ => {
                WsResponse {
                    r#type: "error".to_string(),
                    id: msg.id,
                    data: serde_json::json!({
                        "error": "Unknown message type"
                    }),
                    metadata: None,
                }
            }
        };

        // Output response
        println!("{}", serde_json::to_string(&response).unwrap());
    }
}
```

### 4. Build

```bash
cargo component build --release
```

Output: `target/wasm32-wasip2/release/my_ws_service.wasm`

## Client Examples

### JavaScript (Browser)

```javascript
const ws = new WebSocket('ws://localhost:8080/ws/app/my-service');

ws.onopen = () => {
  console.log('Connected!');
  
  // Send a message
  ws.send(JSON.stringify({
    type: 'greet',
    id: 'msg-1',
    data: 'Pied Piper'
  }));
};

ws.onmessage = (event) => {
  const response = JSON.parse(event.data);
  console.log('Received:', response);
  
  // Handle different response types
  switch(response.type) {
    case 'greeting':
      console.log('Greeting:', response.data.message);
      break;
    case 'error':
      console.error('Error:', response.data.error);
      break;
  }
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('Disconnected');
};
```

### JavaScript (Node.js)

```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:8080/ws/app/my-service');

ws.on('open', () => {
  console.log('Connected!');
  
  ws.send(JSON.stringify({
    type: 'greet',
    id: 'msg-1',
    data: 'Node.js'
  }));
});

ws.on('message', (data) => {
  const response = JSON.parse(data);
  console.log('Received:', response);
});

ws.on('error', (error) => {
  console.error('Error:', error);
});

ws.on('close', () => {
  console.log('Disconnected');
});
```

### Python

```python
import asyncio
import websockets
import json

async def test_websocket():
    uri = "ws://localhost:8080/ws/app/my-service"
    
    async with websockets.connect(uri) as websocket:
        # Send message
        message = {
            "type": "greet",
            "id": "msg-1",
            "data": "Python"
        }
        await websocket.send(json.dumps(message))
        
        # Receive response
        response = await websocket.recv()
        data = json.loads(response)
        print(f"Received: {data}")

asyncio.run(test_websocket())
```

### Rust

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{StreamExt, SinkExt};
use serde_json::json;

#[tokio::main]
async fn main() {
    let url = "ws://localhost:8080/ws/app/my-service";
    let (ws_stream, _) = connect_async(url).await.unwrap();
    
    let (mut write, mut read) = ws_stream.split();
    
    // Send message
    let msg = json!({
        "type": "greet",
        "id": "msg-1",
        "data": "Rust"
    });
    write.send(Message::Text(msg.to_string())).await.unwrap();
    
    // Receive response
    if let Some(Ok(Message::Text(text))) = read.next().await {
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        println!("Received: {:?}", response);
    }
}
```

### websocat (CLI)

```bash
# Install websocat
cargo install websocat

# Connect and test
websocat ws://localhost:8080/ws/app/my-service

# Type messages:
{"type":"greet","id":"1","data":"WebSocat"}
{"type":"ping","id":"2"}
```

## Deployment

### 1. Publish Module

```bash
cargo run --release -- publish \
  target/wasm32-wasip2/release/my_ws_service.wasm \
  --name my-service \
  --version 1.0.0
```

### 2. Start Gateway

```bash
# HTTP only
cargo run --release -- gateway --listen 0.0.0.0:8080

# With HTTPS (WebSocket Secure)
cargo run --release -- gateway \
  --listen 0.0.0.0:8080 \
  --https-port 8443 \
  --cert-path /path/to/cert.pem \
  --key-path /path/to/key.pem
```

### 3. Connect Clients

```
# Via CID
ws://localhost:8080/ws/cid/<CID>

# Via Name
ws://localhost:8080/ws/app/my-service

# Secure WebSocket (wss://)
wss://localhost:8443/ws/app/my-service
```

## Testing

### Unit Testing

Test your message handlers:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet_message() {
        let msg = WsMessage {
            r#type: "greet".to_string(),
            id: Some("test-1".to_string()),
            data: serde_json::json!("Tester"),
            metadata: None,
        };

        let response = handle_message(msg);
        
        assert_eq!(response.r#type, "greeting");
        assert_eq!(response.id, Some("test-1".to_string()));
    }
}
```

### Integration Testing

Test with real WebSocket connection:

```bash
#!/bin/bash

# Start gateway
cargo run --release -- gateway --listen 0.0.0.0:8080 &
GATEWAY_PID=$!

# Wait for startup
sleep 2

# Test with websocat
echo '{"type":"greet","id":"test-1","data":"Integration"}' | \
  websocat -n1 ws://localhost:8080/ws/app/my-service

# Cleanup
kill $GATEWAY_PID
```

### Load Testing

Use `websocat` for basic load testing:

```bash
# 100 concurrent connections
for i in {1..100}; do
  (echo '{"type":"ping","id":"'$i'"}' | \
   websocat -n1 ws://localhost:8080/ws/app/my-service) &
done
wait
```

## Performance

### Benchmarks

**Environment**: M3 Pro, macOS, localhost

| Metric | Value |
|--------|-------|
| Connection Time | ~5ms |
| Message Latency | ~5-10ms |
| Throughput | 1000+ msg/sec |
| Memory per Connection | ~2MB |
| Max Concurrent Connections | 1000+ |

### Optimization Tips

1. **Reuse Connections**: Keep WebSocket alive for multiple messages
2. **Batch Messages**: Send multiple items in one message if possible
3. **Compress Data**: Use smaller JSON or binary encoding
4. **Limit Message Size**: Keep messages under 1MB
5. **Connection Pooling**: Limit concurrent connections per client

## Best Practices

### 1. Message Design

```rust
// ✅ Good: Structured with type safety
{
  "type": "user_action",
  "id": "req-123",
  "data": {
    "action": "login",
    "user": "alice"
  }
}

// ❌ Bad: Unstructured
{
  "doStuff": true,
  "someData": "unclear"
}
```

### 2. Error Handling

```rust
impl Guest for Component {
    fn run() {
        let result = std::panic::catch_unwind(|| {
            // Your logic here
        });

        match result {
            Ok(_) => { /* success */ },
            Err(e) => {
                let error_response = WsResponse {
                    r#type: "error".to_string(),
                    id: None,
                    data: serde_json::json!({
                        "error": "Internal error",
                        "details": format!("{:?}", e)
                    }),
                    metadata: None,
                };
                println!("{}", serde_json::to_string(&error_response).unwrap());
            }
        }
    }
}
```

### 3. Request Correlation

Always use IDs for request/response matching:

```javascript
let requestId = 0;
const pendingRequests = new Map();

function sendRequest(type, data) {
  const id = `req-${++requestId}`;
  
  return new Promise((resolve, reject) => {
    pendingRequests.set(id, { resolve, reject });
    
    ws.send(JSON.stringify({ type, id, data }));
    
    // Timeout
    setTimeout(() => {
      if (pendingRequests.has(id)) {
        pendingRequests.delete(id);
        reject(new Error('Request timeout'));
      }
    }, 5000);
  });
}

ws.onmessage = (event) => {
  const response = JSON.parse(event.data);
  
  if (response.id && pendingRequests.has(response.id)) {
    const { resolve } = pendingRequests.get(response.id);
    pendingRequests.delete(response.id);
    resolve(response);
  }
};
```

### 4. Connection Management

```javascript
class WebSocketClient {
  constructor(url) {
    this.url = url;
    this.reconnectDelay = 1000;
    this.maxReconnectDelay = 30000;
    this.connect();
  }

  connect() {
    this.ws = new WebSocket(this.url);
    
    this.ws.onopen = () => {
      console.log('Connected');
      this.reconnectDelay = 1000;
    };
    
    this.ws.onclose = () => {
      console.log('Disconnected, reconnecting...');
      setTimeout(() => this.connect(), this.reconnectDelay);
      this.reconnectDelay = Math.min(
        this.reconnectDelay * 2,
        this.maxReconnectDelay
      );
    };
  }
}
```

### 5. Security

```rust
// Validate message size
if buffer.len() > 1024 * 1024 { // 1MB
    return error_response("Message too large");
}

// Validate message structure
let msg: WsMessage = match serde_json::from_str(&buffer) {
    Ok(m) => m,
    Err(_) => return error_response("Invalid JSON"),
};

// Sanitize input
let safe_data = msg.data.as_str()
    .unwrap_or("")
    .chars()
    .filter(|c| c.is_alphanumeric() || c.is_whitespace())
    .collect::<String>();
```

## Troubleshooting

### Connection Fails

```
Error: Connection refused
```

**Solution:**
- Check gateway is running
- Verify port is correct
- Check firewall rules

### Module Not Found

```
{"type":"error","data":{"error":"Module not found"}}
```

**Solution:**
- Verify module is published: `cargo run -- list`
- Check CID or name spelling
- Wait for DHT propagation (~30 seconds)

### Invalid JSON

```
{"type":"error","data":{"error":"Invalid WebSocket message format"}}
```

**Solution:**
- Ensure message is valid JSON
- Include required fields: `type`, `data`
- Check JSON escaping

### Timeout

```
Connection timeout after 5 seconds
```

**Solution:**
- Module execution taking too long
- Increase timeout in gateway config
- Optimize WASM code
- Check for infinite loops

### Memory Issues

```
WASM module out of memory
```

**Solution:**
- Reduce message size
- Optimize data structures
- Increase WASM memory limit
- Check for memory leaks

## Use Cases

### 1. Chat Application

```rust
match msg.r#type.as_str() {
    "send_message" => {
        // Broadcast to all connected clients
        broadcast(msg.data);
    }
    "get_history" => {
        // Retrieve chat history
        get_messages(100)
    }
}
```

### 2. Live Dashboard

```rust
match msg.r#type.as_str() {
    "subscribe" => {
        // Start streaming metrics
        stream_metrics()
    }
    "unsubscribe" => {
        // Stop streaming
        stop_stream()
    }
}
```

### 3. Multiplayer Game

```rust
match msg.r#type.as_str() {
    "player_move" => {
        // Update game state
        update_position(msg.data)
    }
    "get_state" => {
        // Return current game state
        game_snapshot()
    }
}
```

## Next Steps

- [Host Functions Guide](HOST_FUNCTIONS.md) - HTTP requests, KV storage
- [State Management Guide](STATE_MANAGEMENT.md) - CRDTs, distributed state
- [Examples](../examples/) - More WebSocket examples

## Reference

- **WebSocket Routes**:
  - `/ws/cid/:cid` - Connect to module by CID
  - `/ws/app/:name` - Connect to module by name

- **Built-in Message Types**:
  - `ping` - Health check
  - `pong` - Ping response
  - `echo` - Echo test
  - `connected` - Connection established
  - `error` - Error occurred

- **HTTP Status Codes**:
  - `101 Switching Protocols` - WebSocket upgrade success
  - `404 Not Found` - Module not found
  - `500 Internal Server Error` - Gateway error
