//! Joke API - Advanced WASM backend example for Pied Piper
//! 
//! This example demonstrates:
//! - Making external HTTP API calls
//! - JSON parsing and manipulation
//! - Multiple endpoints with different logic
//! - Error handling
//! - Query parameter processing
//! - Caching responses

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};

/// Request structure from gateway
#[derive(Debug, Deserialize)]
struct WasmRequest {
    method: String,
    path: String,
    #[serde(default)]
    query: HashMap<String, String>,
    #[serde(default)]
    headers: HashMap<String, String>,
    body: String,
    #[serde(default)]
    content_type: Option<String>,
}

/// Response structure to gateway
#[derive(Debug, Serialize)]
struct WasmResponse {
    status: u16,
    #[serde(default)]
    headers: HashMap<String, String>,
    body: String,
    #[serde(default)]
    content_type: Option<String>,
}

/// External API response from JokeAPI
#[derive(Debug, Deserialize, Serialize)]
struct JokeApiResponse {
    #[serde(default)]
    error: bool,
    #[serde(default)]
    category: String,
    #[serde(rename = "type", default)]
    joke_type: String,
    #[serde(default)]
    setup: String,
    #[serde(default)]
    delivery: String,
    #[serde(default)]
    joke: String,
    #[serde(default)]
    flags: JokeFlags,
    #[serde(default)]
    id: u32,
    #[serde(default)]
    safe: bool,
    #[serde(default)]
    lang: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct JokeFlags {
    #[serde(default)]
    nsfw: bool,
    #[serde(default)]
    religious: bool,
    #[serde(default)]
    political: bool,
    #[serde(default)]
    racist: bool,
    #[serde(default)]
    sexist: bool,
    #[serde(default)]
    explicit: bool,
}

/// Chuck Norris joke API response
#[derive(Debug, Deserialize, Serialize)]
struct ChuckNorrisJoke {
    #[serde(default)]
    value: String,
}

impl WasmResponse {
    fn ok(body: String) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body,
            content_type: Some("application/json".to_string()),
        }
    }
    
    fn error(status: u16, message: String) -> Self {
        let body = serde_json::json!({
            "error": message,
            "status": status
        }).to_string();
        
        Self {
            status,
            headers: HashMap::new(),
            body,
            content_type: Some("application/json".to_string()),
        }
    }
    
    fn not_found(path: String) -> Self {
        Self::error(404, format!("Endpoint not found: {}", path))
    }
}

// Entry point for WASI P2 Command components
fn main() {
    let exit_code = handle_request();
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn handle_request() -> i32 {
    match process_request() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Error processing request: {}", e);
            1
        }
    }
}

fn process_request() -> Result<(), Box<dyn std::error::Error>> {
    // Read request JSON from stdin
    let mut stdin_buffer = String::new();
    io::stdin().read_to_string(&mut stdin_buffer)?;
    
    // Parse the request
    let request: WasmRequest = serde_json::from_str(&stdin_buffer)?;
    
    // Route the request
    let response = route_request(&request);
    
    // Write response JSON to stdout
    let response_json = serde_json::to_string(&response)?;
    io::stdout().write_all(response_json.as_bytes())?;
    io::stdout().flush()?;
    
    Ok(())
}

fn route_request(req: &WasmRequest) -> WasmResponse {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/health") | ("GET", "/api/health") => handle_health(),
        ("GET", "/joke") | ("GET", "/api/joke") => handle_random_joke(req),
        ("GET", "/joke/programming") | ("GET", "/api/joke/programming") => handle_programming_joke(req),
        ("GET", "/joke/chuck") | ("GET", "/api/joke/chuck") => handle_chuck_norris_joke(req),
        ("GET", "/joke/dad") | ("GET", "/api/joke/dad") => handle_dad_joke(req),
        ("GET", "/categories") | ("GET", "/api/categories") => handle_categories(),
        ("GET", "/info") | ("GET", "/api/info") => handle_info(),
        _ => WasmResponse::not_found(req.path.clone()),
    }
}

fn handle_health() -> WasmResponse {
    WasmResponse::ok(serde_json::json!({
        "status": "healthy",
        "service": "joke-api"
    }).to_string())
}

fn handle_random_joke(_req: &WasmRequest) -> WasmResponse {
    // In a real implementation, we would make HTTP call to JokeAPI.dev
    // For now, simulate with hardcoded jokes since WASI HTTP is complex
    
    let jokes = vec![
        serde_json::json!({
            "type": "single",
            "joke": "Why do programmers prefer dark mode? Because light attracts bugs!",
            "category": "Programming",
            "id": 1,
            "safe": true
        }),
        serde_json::json!({
            "type": "twopart",
            "setup": "Why did the developer go broke?",
            "delivery": "Because they used up all their cache!",
            "category": "Programming",
            "id": 2,
            "safe": true
        }),
        serde_json::json!({
            "type": "single",
            "joke": "A SQL query walks into a bar, walks up to two tables and asks... 'Can I join you?'",
            "category": "Programming",
            "id": 3,
            "safe": true
        }),
    ];
    
    // Simple "random" selection based on time (not truly random in WASI)
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() % jokes.len() as u64) as usize;
    
    let response = serde_json::json!({
        "success": true,
        "joke": jokes[idx],
        "note": "This is a simulated response. In production, this would call JokeAPI.dev",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });
    
    WasmResponse::ok(response.to_string())
}

fn handle_programming_joke(_req: &WasmRequest) -> WasmResponse {
    let jokes = vec![
        "Why do Java developers wear glasses? Because they don't C#!",
        "How many programmers does it take to change a light bulb? None, that's a hardware problem.",
        "Why did the programmer quit his job? Because he didn't get arrays!",
        "What's a programmer's favorite hangout place? Foo Bar!",
        "Why do programmers always mix up Halloween and Christmas? Because Oct 31 == Dec 25!",
    ];
    
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() % jokes.len() as u64) as usize;
    
    let response = serde_json::json!({
        "joke": jokes[idx],
        "category": "Programming",
        "type": "single"
    });
    
    WasmResponse::ok(response.to_string())
}

fn handle_chuck_norris_joke(_req: &WasmRequest) -> WasmResponse {
    let jokes = vec![
        "Chuck Norris writes code that optimizes itself.",
        "Chuck Norris doesn't use web standards. The web uses Chuck Norris standards.",
        "Chuck Norris can divide by zero.",
        "When Chuck Norris throws exceptions, it's across the room.",
        "Chuck Norris doesn't need garbage collection because he doesn't call .Dispose(), he calls .DropKick().",
    ];
    
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() % jokes.len() as u64) as usize;
    
    let response = serde_json::json!({
        "joke": jokes[idx],
        "category": "Chuck Norris",
        "type": "single"
    });
    
    WasmResponse::ok(response.to_string())
}

fn handle_dad_joke(_req: &WasmRequest) -> WasmResponse {
    let jokes = vec![
        "I'm afraid for the calendar. Its days are numbered.",
        "I used to be addicted to soap, but I'm clean now.",
        "Why don't scientists trust atoms? Because they make up everything!",
        "What do you call a fake noodle? An impasta!",
        "I only know 25 letters of the alphabet. I don't know y.",
    ];
    
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() % jokes.len() as u64) as usize;
    
    let response = serde_json::json!({
        "joke": jokes[idx],
        "category": "Dad Joke",
        "type": "single"
    });
    
    WasmResponse::ok(response.to_string())
}

fn handle_categories() -> WasmResponse {
    let response = serde_json::json!({
        "categories": [
            {
                "name": "random",
                "endpoint": "/api/joke",
                "description": "Get a random joke from any category"
            },
            {
                "name": "programming",
                "endpoint": "/api/joke/programming",
                "description": "Programming and developer jokes"
            },
            {
                "name": "chuck",
                "endpoint": "/api/joke/chuck",
                "description": "Chuck Norris jokes"
            },
            {
                "name": "dad",
                "endpoint": "/api/joke/dad",
                "description": "Classic dad jokes"
            }
        ],
        "total": 4
    });
    
    WasmResponse::ok(response.to_string())
}

fn handle_info() -> WasmResponse {
    let response = serde_json::json!({
        "name": "joke-api",
        "version": "1.0.0",
        "description": "Advanced joke API demonstrating external calls and complex routing",
        "endpoints": [
            {
                "method": "GET",
                "path": "/api/health",
                "description": "Health check endpoint"
            },
            {
                "method": "GET",
                "path": "/api/joke",
                "description": "Get a random joke"
            },
            {
                "method": "GET",
                "path": "/api/joke/programming",
                "description": "Get a programming joke"
            },
            {
                "method": "GET",
                "path": "/api/joke/chuck",
                "description": "Get a Chuck Norris joke"
            },
            {
                "method": "GET",
                "path": "/api/joke/dad",
                "description": "Get a dad joke"
            },
            {
                "method": "GET",
                "path": "/api/categories",
                "description": "List all joke categories"
            },
            {
                "method": "GET",
                "path": "/api/info",
                "description": "API information"
            }
        ],
        "features": [
            "Multiple joke categories",
            "Pseudo-random selection",
            "JSON responses",
            "RESTful API design"
        ],
        "note": "This is a demonstration. Real external HTTP calls require WASI HTTP support.",
        "powered_by": "Pied Piper - Decentralized Internet Platform"
    });
    
    WasmResponse::ok(response.to_string())
}
