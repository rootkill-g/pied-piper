use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    username: String,
    text: String,
    timestamp: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientMessage {
    Join { username: String },
    Message { text: String },
    Leave,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ServerMessage {
    Join { username: String, timestamp: u64 },
    Leave { username: String, timestamp: u64 },
    Message { username: String, text: String, timestamp: u64 },
    History { messages: Vec<ChatMessage> },
    Error { message: String },
}

#[derive(Debug, Deserialize)]
struct WebSocketRequest {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    connection_id: Option<String>,
}

// Storage and time helpers
mod host {
    use super::*;

    #[link(wasm_import_module = "host")]
    extern "C" {
        fn storage_get_v2(key_ptr: *const u8, key_len: usize) -> i32;
        fn storage_set_v2(
            key_ptr: *const u8,
            key_len: usize,
            value_ptr: *const u8,
            value_len: usize,
        ) -> i32;
        fn host_get_result(ptr: *mut u8, len: usize) -> usize;
        fn host_now_millis() -> u64;
    }

    pub fn storage_get(key: &str) -> Option<String> {
        unsafe {
            let ret = storage_get_v2(key.as_ptr(), key.len());
            if ret < 0 {
                return None;
            }
            let size = ret as usize;
            let mut buffer = vec![0u8; size];
            host_get_result(buffer.as_mut_ptr(), size);
            String::from_utf8(buffer).ok()
        }
    }

    pub fn storage_set(key: &str, value: &str) -> bool {
        unsafe {
            let ret = storage_set_v2(
                key.as_ptr(),
                key.len(),
                value.as_ptr(),
                value.len(),
            );
            ret >= 0
        }
    }

    pub fn now_millis() -> u64 {
        unsafe { host_now_millis() }
    }
}

const MAX_HISTORY: usize = 100;
const MAX_MESSAGE_LEN: usize = 1000;
const MAX_USERNAME_LEN: usize = 20;

fn get_messages() -> Vec<ChatMessage> {
    host::storage_get("chat:messages")
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn save_messages(messages: &[ChatMessage]) -> bool {
    if let Ok(json) = serde_json::to_string(messages) {
        host::storage_set("chat:messages", &json)
    } else {
        false
    }
}

fn add_message(username: String, text: String, timestamp: u64) -> Result<(), String> {
    // Validate
    if text.len() > MAX_MESSAGE_LEN {
        return Err(format!("Message too long (max {})", MAX_MESSAGE_LEN));
    }

    let mut messages = get_messages();
    messages.push(ChatMessage {
        username,
        text,
        timestamp,
    });

    // Keep only last MAX_HISTORY messages
    if messages.len() > MAX_HISTORY {
        messages.drain(0..messages.len() - MAX_HISTORY);
    }

    if save_messages(&messages) {
        Ok(())
    } else {
        Err("Failed to save message".to_string())
    }
}

fn get_users() -> Vec<String> {
    host::storage_get("chat:users")
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn save_users(users: &[String]) -> bool {
    if let Ok(json) = serde_json::to_string(users) {
        host::storage_set("chat:users", &json)
    } else {
        false
    }
}

fn add_user(username: String) -> Result<(), String> {
    if username.len() > MAX_USERNAME_LEN {
        return Err(format!("Username too long (max {})", MAX_USERNAME_LEN));
    }

    let mut users = get_users();
    if !users.contains(&username) {
        users.push(username);
        if save_users(&users) {
            Ok(())
        } else {
            Err("Failed to save user list".to_string())
        }
    } else {
        Err("Username already taken".to_string())
    }
}

fn remove_user(username: &str) {
    let mut users = get_users();
    users.retain(|u| u != username);
    save_users(&users);
}

fn get_connection_username(connection_id: &str) -> Option<String> {
    let key = format!("chat:conn:{}", connection_id);
    host::storage_get(&key)
}

fn set_connection_username(connection_id: &str, username: &str) {
    let key = format!("chat:conn:{}", connection_id);
    host::storage_set(&key, username);
}

fn handle_client_message(
    msg: ClientMessage,
    connection_id: &str,
) -> Result<ServerMessage, String> {
    let timestamp = host::now_millis();

    match msg {
        ClientMessage::Join { username } => {
            let username = username.trim().to_string();
            
            if username.is_empty() {
                return Err("Username cannot be empty".to_string());
            }

            add_user(username.clone())?;
            set_connection_username(connection_id, &username);

            Ok(ServerMessage::Join { username, timestamp })
        }

        ClientMessage::Message { text } => {
            let text = text.trim().to_string();
            
            if text.is_empty() {
                return Err("Message cannot be empty".to_string());
            }

            let username = get_connection_username(connection_id)
                .ok_or_else(|| "Not joined to chat".to_string())?;

            add_message(username.clone(), text.clone(), timestamp)?;

            Ok(ServerMessage::Message {
                username,
                text,
                timestamp,
            })
        }

        ClientMessage::Leave => {
            if let Some(username) = get_connection_username(connection_id) {
                remove_user(&username);
                Ok(ServerMessage::Leave { username, timestamp })
            } else {
                Err("Not joined to chat".to_string())
            }
        }
    }
}

fn main() {
    // Read WebSocket message from stdin
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let ws_req: WebSocketRequest = match serde_json::from_str(&input) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Failed to parse request: {}", e);
            return;
        }
    };

    let connection_id = ws_req.connection_id.unwrap_or_else(|| "unknown".to_string());

    // If no message, send history (initial connection)
    if ws_req.message.is_none() {
        let messages = get_messages();
        let response = ServerMessage::History { messages };
        let output = serde_json::to_string(&response).unwrap();
        print!("{}", output);
        io::stdout().flush().unwrap();
        return;
    }

    let message_text = ws_req.message.unwrap();

    // Parse client message
    let client_msg: ClientMessage = match serde_json::from_str(&message_text) {
        Ok(msg) => msg,
        Err(e) => {
            let error_response = ServerMessage::Error {
                message: format!("Invalid message format: {}", e),
            };
            let output = serde_json::to_string(&error_response).unwrap();
            print!("{}", output);
            io::stdout().flush().unwrap();
            return;
        }
    };

    // Handle message
    let response = match handle_client_message(client_msg, &connection_id) {
        Ok(msg) => msg,
        Err(err) => ServerMessage::Error { message: err },
    };

    // Write response to stdout
    let output = serde_json::to_string(&response).unwrap();
    print!("{}", output);
    io::stdout().flush().unwrap();
}
