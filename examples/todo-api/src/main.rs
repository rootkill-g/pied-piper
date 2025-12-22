use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Todo {
    id: String,
    title: String,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct CreateTodo {
    title: String,
}

#[derive(Debug, Deserialize)]
struct UpdateTodo {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    done: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct Request {
    method: String,
    path: String,
    #[serde(default)]
    query: HashMap<String, String>,
    body: Option<String>,
}

#[derive(Debug, Serialize)]
struct Response {
    status: u16,
    body: String,
    headers: HashMap<String, String>,
}

// Storage helpers using extern functions
mod storage {
    use super::*;

    #[link(wasm_import_module = "env")]
    extern "C" {
        fn host_storage_get(key_ptr: *const u8, key_len: usize, val_ptr: *mut u8, val_len_ptr: *mut usize) -> i32;
        fn host_storage_set(
            key_ptr: *const u8,
            key_len: usize,
            value_ptr: *const u8,
            value_len: usize,
        ) -> i32;
        fn host_storage_delete(key_ptr: *const u8, key_len: usize) -> i32;
    }

    pub fn get(key: &str) -> Option<String> {
        unsafe {
            let mut val_len: u32 = 1024;
            let mut buffer = vec![0u8; val_len as usize];
            let ret = host_storage_get(key.as_ptr(), key.len(), buffer.as_mut_ptr(), &mut val_len as *mut u32 as *mut usize);
            if ret == 0 {
                return None;  // Not found
            }
            if val_len > buffer.len() as u32 {
                buffer.resize(val_len as usize, 0);
                host_storage_get(key.as_ptr(), key.len(), buffer.as_mut_ptr(), &mut val_len as *mut u32 as *mut usize);
            }
            buffer.truncate(val_len as usize);
            String::from_utf8(buffer).ok()
        }
    }

    pub fn set(key: &str, value: &str) -> bool {
        unsafe {
            let ret = host_storage_set(
                key.as_ptr(),
                key.len(),
                value.as_ptr(),
                value.len(),
            );
            ret >= 0
        }
    }

    pub fn delete(key: &str) -> bool {
        unsafe {
            let ret = host_storage_delete(key.as_ptr(), key.len());
            ret >= 0
        }
    }
}

fn json_response(status: u16, body: String) -> Response {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    Response { status, body, headers }
}

fn get_next_id() -> String {
    let key = "todo:next_id";
    let current = storage::get(key)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);

    let next_id = current.to_string();
    storage::set(key, &(current + 1).to_string());
    next_id
}

fn get_todo(id: &str) -> Option<Todo> {
    let key = format!("todo:{}", id);
    storage::get(&key)
        .and_then(|json| serde_json::from_str(&json).ok())
}

fn list_todos() -> Vec<Todo> {
    // In a real app, we'd maintain an index
    // For this demo, we'll scan IDs 1-1000
    let mut todos = Vec::new();
    let max_id = storage::get("todo:next_id")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);

    for id in 1..max_id {
        if let Some(todo) = get_todo(&id.to_string()) {
            todos.push(todo);
        }
    }
    todos
}

fn create_todo(title: String) -> Result<Todo, String> {
    let id = get_next_id();
    let todo = Todo {
        id: id.clone(),
        title,
        done: false,
    };

    let key = format!("todo:{}", id);
    let json = serde_json::to_string(&todo)
        .map_err(|e| format!("Serialization error: {}", e))?;

    if storage::set(&key, &json) {
        Ok(todo)
    } else {
        Err("Failed to save todo".to_string())
    }
}

fn update_todo(id: String, title: Option<String>, done: Option<bool>) -> Result<Todo, String> {
    let mut todo = get_todo(&id)
        .ok_or_else(|| format!("Todo {} not found", id))?;

    if let Some(new_title) = title {
        todo.title = new_title;
    }
    if let Some(new_done) = done {
        todo.done = new_done;
    }

    let key = format!("todo:{}", id);
    let json = serde_json::to_string(&todo)
        .map_err(|e| format!("Serialization error: {}", e))?;

    if storage::set(&key, &json) {
        Ok(todo)
    } else {
        Err("Failed to update todo".to_string())
    }
}

fn delete_todo(id: &str) -> bool {
    let key = format!("todo:{}", id);
    storage::delete(&key)
}

fn parse_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.split('=');
            let key = parts.next()?;
            let value = parts.next()?;
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

fn handle_request(req: Request) -> Response {
    let method = req.method.as_str();
    let path = req.path.as_str();

    // Get id from query parameters
    let id_param = req.query.get("id").map(|s| s.as_str());

    match (method, path) {
        // GET / - List all todos
        // GET /?id=1 - Get specific todo
        ("GET", "/") => {
            if let Some(id) = id_param {
                match get_todo(id) {
                    Some(todo) => json_response(200, serde_json::to_string(&todo).unwrap()),
                    None => json_response(404, r#"{"error":"Todo not found"}"#.to_string()),
                }
            } else {
                let todos = list_todos();
                json_response(200, serde_json::to_string(&todos).unwrap())
            }
        }

        // POST / - Create todo
        ("POST", "/") => {
            let body = req.body.unwrap_or_default();
            match serde_json::from_str::<CreateTodo>(&body) {
                Ok(create_req) => match create_todo(create_req.title) {
                    Ok(todo) => json_response(201, serde_json::to_string(&todo).unwrap()),
                    Err(err) => json_response(500, format!(r#"{{"error":"{}"}}"#, err)),
                },
                Err(_) => json_response(400, r#"{"error":"Invalid request body"}"#.to_string()),
            }
        }

        // PUT / - Update todo
        ("PUT", "/") => {
            let body = req.body.unwrap_or_default();
            match serde_json::from_str::<UpdateTodo>(&body) {
                Ok(update_req) => {
                    match update_todo(update_req.id, update_req.title, update_req.done) {
                        Ok(todo) => json_response(200, serde_json::to_string(&todo).unwrap()),
                        Err(err) => {
                            let status = if err.contains("not found") { 404 } else { 500 };
                            json_response(status, format!(r#"{{"error":"{}"}}"#, err))
                        }
                    }
                }
                Err(_) => json_response(400, r#"{"error":"Invalid request body"}"#.to_string()),
            }
        }

        // DELETE /?id=1 - Delete todo
        ("DELETE", "/") => {
            if let Some(id) = id_param {
                if delete_todo(id) {
                    json_response(200, r#"{"success":true}"#.to_string())
                } else {
                    json_response(404, r#"{"error":"Todo not found"}"#.to_string())
                }
            } else {
                json_response(400, r#"{"error":"Missing id parameter"}"#.to_string())
            }
        }

        _ => json_response(404, r#"{"error":"Not found"}"#.to_string()),
    }
}

fn main() {
    // Read request from stdin
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    // Parse request
    let request: Request = match serde_json::from_str(&input) {
        Ok(req) => req,
        Err(e) => {
            let error_response = json_response(400, format!(r#"{{"error":"Invalid request: {}"}}"#, e));
            let output = serde_json::to_string(&error_response).unwrap();
            print!("{}", output);
            return;
        }
    };

    // Handle request
    let response = handle_request(request);

    // Write response to stdout
    let output = serde_json::to_string(&response).unwrap();
    print!("{}", output);
    io::stdout().flush().unwrap();
}
