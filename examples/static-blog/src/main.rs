use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Post {
    id: String,
    title: String,
    content: String,
    created_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CreatePost {
    title: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct UpdatePost {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Request {
    method: String,
    path: String,
    #[serde(default)]
    query: HashMap<String, String>,
    #[serde(default)]
    headers: HashMap<String, String>,
    body: String,
}

#[derive(Debug, Serialize)]
struct Response {
    status: u16,
    body: String,
    headers: HashMap<String, String>,
}

mod host {
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
        fn host_now_millis() -> i64;
    }

    pub fn storage_get(key: &str) -> Option<String> {
        unsafe {
            let mut val_len: u32 = 1024; // Start with reasonable size
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

    pub fn storage_set(key: &str, value: &str) -> bool {
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

    pub fn storage_delete(key: &str) -> bool {
        unsafe {
            let ret = host_storage_delete(key.as_ptr(), key.len());
            ret >= 0
        }
    }

    pub fn now_millis() -> u64 {
        unsafe { host_now_millis() as u64 }
    }
}

const MAX_TITLE_LEN: usize = 200;
const MAX_CONTENT_LEN: usize = 50_000;

fn get_post_ids() -> Vec<String> {
    host::storage_get("blog:posts")
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn save_post_ids(ids: &[String]) -> bool {
    if let Ok(json) = serde_json::to_string(ids) {
        host::storage_set("blog:posts", &json)
    } else {
        false
    }
}

fn get_next_id() -> String {
    let key = "blog:next_id";
    let current = host::storage_get(key)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);
    let next_id = current.to_string();
    host::storage_set(key, &(current + 1).to_string());
    next_id
}

fn get_post(id: &str) -> Option<Post> {
    let key = format!("blog:post:{}", id);
    host::storage_get(&key)
        .and_then(|json| serde_json::from_str(&json).ok())
}

fn list_posts() -> Vec<Post> {
    get_post_ids()
        .iter()
        .filter_map(|id| get_post(id))
        .collect()
}

fn create_post(title: String, content: String) -> Result<Post, String> {
    if title.len() > MAX_TITLE_LEN {
        return Err(format!("Title too long (max {})", MAX_TITLE_LEN));
    }
    if content.len() > MAX_CONTENT_LEN {
        return Err(format!("Content too long (max {})", MAX_CONTENT_LEN));
    }

    let id = get_next_id();
    let post = Post {
        id: id.clone(),
        title,
        content,
        created_at: host::now_millis(),
        updated_at: None,
    };

    let key = format!("blog:post:{}", id);
    let json = serde_json::to_string(&post)
        .map_err(|e| format!("Serialization error: {}", e))?;

    if !host::storage_set(&key, &json) {
        return Err("Failed to save post".to_string());
    }

    let mut ids = get_post_ids();
    ids.push(id);
    if !save_post_ids(&ids) {
        return Err("Failed to update post index".to_string());
    }

    Ok(post)
}

fn update_post(id: String, title: Option<String>, content: Option<String>) -> Result<Post, String> {
    let mut post = get_post(&id)
        .ok_or_else(|| format!("Post {} not found", id))?;

    if let Some(new_title) = title {
        if new_title.len() > MAX_TITLE_LEN {
            return Err(format!("Title too long (max {})", MAX_TITLE_LEN));
        }
        post.title = new_title;
    }

    if let Some(new_content) = content {
        if new_content.len() > MAX_CONTENT_LEN {
            return Err(format!("Content too long (max {})", MAX_CONTENT_LEN));
        }
        post.content = new_content;
    }

    post.updated_at = Some(host::now_millis());

    let key = format!("blog:post:{}", id);
    let json = serde_json::to_string(&post)
        .map_err(|e| format!("Serialization error: {}", e))?;

    if host::storage_set(&key, &json) {
        Ok(post)
    } else {
        Err("Failed to update post".to_string())
    }
}

fn delete_post(id: &str) -> Result<(), String> {
    let key = format!("blog:post:{}", id);
    if !host::storage_delete(&key) {
        return Err("Failed to delete post".to_string());
    }

    let mut ids = get_post_ids();
    ids.retain(|post_id| post_id != id);
    if save_post_ids(&ids) {
        Ok(())
    } else {
        Err("Failed to update post index".to_string())
    }
}

fn json_response(status: u16, body: String) -> Response {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    Response { status, body, headers }
}

fn handle_api_request(req: Request) -> Response {
    let method = req.method.as_str();
    let path = req.path.as_str();

    let id_param = req.query.get("id").map(|s| s.as_str());

    match (method, path) {
        ("GET", "/api/posts") => {
            if let Some(id) = id_param {
                match get_post(id) {
                    Some(post) => json_response(200, serde_json::to_string(&post).unwrap()),
                    None => json_response(404, r#"{"error":"Post not found"}"#.to_string()),
                }
            } else {
                let posts = list_posts();
                json_response(200, serde_json::to_string(&posts).unwrap())
            }
        }

        ("POST", "/api/posts") => {
            match serde_json::from_str::<CreatePost>(&req.body) {
                Ok(create_req) => match create_post(create_req.title, create_req.content) {
                    Ok(post) => json_response(201, serde_json::to_string(&post).unwrap()),
                    Err(err) => json_response(500, format!(r#"{{"error":"{}"}}"#, err)),
                },
                Err(_) => json_response(400, r#"{"error":"Invalid request body"}"#.to_string()),
            }
        }

        ("PUT", "/api/posts") => {
            match serde_json::from_str::<UpdatePost>(&req.body) {
                Ok(update_req) => {
                    match update_post(update_req.id, update_req.title, update_req.content) {
                        Ok(post) => json_response(200, serde_json::to_string(&post).unwrap()),
                        Err(err) => {
                            let status = if err.contains("not found") { 404 } else { 500 };
                            json_response(status, format!(r#"{{"error":"{}"}}"#, err))
                        }
                    }
                }
                Err(_) => json_response(400, r#"{"error":"Invalid request body"}"#.to_string()),
            }
        }

        ("DELETE", "/api/posts") => {
            if let Some(id) = id_param {
                match delete_post(id) {
                    Ok(_) => json_response(200, r#"{"success":true}"#.to_string()),
                    Err(err) => json_response(500, format!(r#"{{"error":"{}"}}"#, err)),
                }
            } else {
                json_response(400, r#"{"error":"Missing id parameter"}"#.to_string())
            }
        }

        _ => json_response(404, r#"{"error":"Not found"}"#.to_string()),
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let request: Request = match serde_json::from_str(&input) {
        Ok(req) => req,
        Err(e) => {
            let error_response = json_response(400, format!(r#"{{"error":"Invalid request: {}"}}"#, e));
            let output = serde_json::to_string(&error_response).unwrap();
            print!("{}", output);
            return;
        }
    };

    // Only handle API requests; gateway serves static assets
    if !request.path.starts_with("/api/") {
        let error_response = json_response(404, r#"{"error":"Not found"}"#.to_string());
        let output = serde_json::to_string(&error_response).unwrap();
        print!("{}", output);
        return;
    }

    let response = handle_api_request(request);
    let output = serde_json::to_string(&response).unwrap();
    print!("{}", output);
    io::stdout().flush().unwrap();
}
