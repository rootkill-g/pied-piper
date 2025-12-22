use serde::{Deserialize, Serialize};
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
    query: Option<String>,
    body: Option<String>,
}

#[derive(Debug, Serialize)]
struct Response {
    status: u16,
    body: String,
    headers: Vec<(String, String)>,
}

// Storage helpers using extern functions
mod storage {
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
        fn storage_delete_v2(key_ptr: *const u8, key_len: usize) -> i32;
        fn host_get_result(ptr: *mut u8, len: usize) -> usize;
    }

    pub fn get(key: &str) -> Option<String> {
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

    pub fn set(key: &str, value: &str) -> bool {
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

    pub fn delete(key: &str) -> bool {
        unsafe {
            let ret = storage_delete_v2(key.as_ptr(), key.len());
            ret >= 0
        }
    }
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

    // Parse query parameters
    let query_params: Vec<(String, String)> = req
        .query
        .as_ref()
        .map(|q| parse_query(q))
        .unwrap_or_default();

    let id_param = query_params
        .iter()
        .find(|(k, _)| k == "id")
        .map(|(_, v)| v.as_str());

    match (method, path) {
        // GET / - List all todos
        // GET /?id=1 - Get specific todo
        ("GET", "/") => {
            if let Some(id) = id_param {
                match get_todo(id) {
                    Some(todo) => {
                        let body = serde_json::to_string(&todo).unwrap();
                        Response {
                            status: 200,
                            body,
                            headers: vec![
                                ("Content-Type".to_string(), "application/json".to_string())
                            ],
                        }
                    }
                    None => Response {
                        status: 404,
                        body: r#"{"error":"Todo not found"}"#.to_string(),
                        headers: vec![
                            ("Content-Type".to_string(), "application/json".to_string())
                        ],
                    },
                }
            } else {
                let todos = list_todos();
                let body = serde_json::to_string(&todos).unwrap();
                Response {
                    status: 200,
                    body,
                    headers: vec![
                        ("Content-Type".to_string(), "application/json".to_string())
                    ],
                }
            }
        }

        // POST / - Create todo
        ("POST", "/") => {
            let body = req.body.unwrap_or_default();
            match serde_json::from_str::<CreateTodo>(&body) {
                Ok(create_req) => match create_todo(create_req.title) {
                    Ok(todo) => {
                        let body = serde_json::to_string(&todo).unwrap();
                        Response {
                            status: 201,
                            body,
                            headers: vec![
                                ("Content-Type".to_string(), "application/json".to_string())
                            ],
                        }
                    }
                    Err(err) => Response {
                        status: 500,
                        body: format!(r#"{{"error":"{}"}}"#, err),
                        headers: vec![
                            ("Content-Type".to_string(), "application/json".to_string())
                        ],
                    },
                },
                Err(_) => Response {
                    status: 400,
                    body: r#"{"error":"Invalid request body"}"#.to_string(),
                    headers: vec![
                        ("Content-Type".to_string(), "application/json".to_string())
                    ],
                },
            }
        }

        // PUT / - Update todo
        ("PUT", "/") => {
            let body = req.body.unwrap_or_default();
            match serde_json::from_str::<UpdateTodo>(&body) {
                Ok(update_req) => {
                    match update_todo(update_req.id, update_req.title, update_req.done) {
                        Ok(todo) => {
                            let body = serde_json::to_string(&todo).unwrap();
                            Response {
                                status: 200,
                                body,
                                headers: vec![
                                    ("Content-Type".to_string(), "application/json".to_string())
                                ],
                            }
                        }
                        Err(err) => {
                            let status = if err.contains("not found") { 404 } else { 500 };
                            Response {
                                status,
                                body: format!(r#"{{"error":"{}"}}"#, err),
                                headers: vec![
                                    ("Content-Type".to_string(), "application/json".to_string())
                                ],
                            }
                        }
                    }
                }
                Err(_) => Response {
                    status: 400,
                    body: r#"{"error":"Invalid request body"}"#.to_string(),
                    headers: vec![
                        ("Content-Type".to_string(), "application/json".to_string())
                    ],
                },
            }
        }

        // DELETE /?id=1 - Delete todo
        ("DELETE", "/") => {
            if let Some(id) = id_param {
                if delete_todo(id) {
                    Response {
                        status: 200,
                        body: r#"{"success":true}"#.to_string(),
                        headers: vec![
                            ("Content-Type".to_string(), "application/json".to_string())
                        ],
                    }
                } else {
                    Response {
                        status: 404,
                        body: r#"{"error":"Todo not found"}"#.to_string(),
                        headers: vec![
                            ("Content-Type".to_string(), "application/json".to_string())
                        ],
                    }
                }
            } else {
                Response {
                    status: 400,
                    body: r#"{"error":"Missing id parameter"}"#.to_string(),
                    headers: vec![
                        ("Content-Type".to_string(), "application/json".to_string())
                    ],
                }
            }
        }

        _ => Response {
            status: 404,
            body: r#"{"error":"Not found"}"#.to_string(),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string())
            ],
        },
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
            let error_response = Response {
                status: 400,
                body: format!(r#"{{"error":"Invalid request: {}"}}"#, e),
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string())
                ],
            };
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
