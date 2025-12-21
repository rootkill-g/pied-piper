//! Simple Hello World API handler for testing Pied Piper Gateway
//! 
//! This is a minimal WASM module that exports a `handle_request` function
//! which can be called by the Pied Piper HTTP Gateway to handle API requests.

#[no_mangle]
pub extern "C" fn handle_request() -> i32 {
    // In a full implementation, this would:
    // 1. Read request data from stdin or linear memory
    // 2. Process the request
    // 3. Write response to stdout or linear memory
    // 4. Return status code
    
    // For now, just return success (0)
    0
}

#[no_mangle]
pub extern "C" fn _start() {
    // Entry point for WASI modules
    // This can print a message or do initialization
}

// Export a simple greeting function for testing
#[no_mangle]
pub extern "C" fn greet() -> i32 {
    42
}
