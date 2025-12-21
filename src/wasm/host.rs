use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use wasmtime::*;

/// Host functions that can be called from Wasm modules
pub struct HostFunctions {
    /// Shared state (for future use with network, storage, etc.)
    state: Arc<RwLock<HostState>>,
}

/// State shared between host functions
struct HostState {
    /// Log messages from Wasm
    log_messages: Vec<String>,
}

impl HostFunctions {
    /// Create new host functions
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HostState {
                log_messages: Vec::new(),
            })),
        }
    }
    
    /// Add host functions to a linker
    pub fn add_to_linker(&self, linker: &mut Linker<crate::wasm::runtime::WasiState>) -> Result<()> {
        // Host logging function
        let state_clone = self.state.clone();
        linker.func_wrap(
            "host",
            "log",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>, ptr: i32, len: i32| -> Result<()> {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => anyhow::bail!("Failed to find memory export"),
                };
                
                let data = memory
                    .data(&caller)
                    .get(ptr as usize..(ptr + len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access"))?;
                
                let message = String::from_utf8_lossy(data);
                info!("Wasm module log: {}", message);
                
                // Store in state
                let state = state_clone.clone();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let mut state = state.write().await;
                        state.log_messages.push(message.to_string());
                    })
                });
                
                Ok(())
            },
        )?;
        
        // Host function to get current time (milliseconds since epoch)
        linker.func_wrap(
            "host",
            "now_millis",
            |_caller: Caller<'_, crate::wasm::runtime::WasiState>| -> i64 {
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
            },
        )?;
        
        // Host function to generate random bytes (for testing)
        linker.func_wrap(
            "host",
            "random_u32",
            |_caller: Caller<'_, crate::wasm::runtime::WasiState>| -> u32 {
                use rand::Rng;
                rand::rng().random()
            },
        )?;
        
        debug!("Added host functions to linker");
        Ok(())
    }
    
    /// Get logged messages from Wasm
    pub async fn get_logs(&self) -> Vec<String> {
        let state = self.state.read().await;
        state.log_messages.clone()
    }
    
    /// Clear logged messages
    pub async fn clear_logs(&self) {
        let mut state = self.state.write().await;
        state.log_messages.clear();
    }
}

impl Default for HostFunctions {
    fn default() -> Self {
        Self::new()
    }
}

/// Host functions for network operations (future implementation)
pub struct NetworkHostFunctions;

impl NetworkHostFunctions {
    /// HTTP client function (placeholder)
    pub fn add_http_functions(_linker: &mut Linker<crate::wasm::runtime::WasiState>) -> Result<()> {
        // TODO: Implement in Phase 3
        // - http_request(url, method, headers, body) -> response
        // - http_get(url) -> body
        // - http_post(url, body) -> response
        Ok(())
    }
}

/// Host functions for storage operations (future implementation)  
pub struct StorageHostFunctions;

impl StorageHostFunctions {
    /// Key-value storage functions (placeholder)
    pub fn add_storage_functions(_linker: &mut Linker<crate::wasm::runtime::WasiState>) -> Result<()> {
        // TODO: Implement in Phase 3
        // - storage_get(key) -> value
        // - storage_put(key, value) -> ()
        // - storage_delete(key) -> ()
        // - storage_list(prefix) -> keys
        Ok(())
    }
}

/// Host functions for cryptographic operations
pub struct CryptoHostFunctions;

impl CryptoHostFunctions {
    /// Add crypto functions to linker
    pub fn add_crypto_functions(linker: &mut Linker<crate::wasm::runtime::WasiState>) -> Result<()> {
        // Blake3 hash function
        linker.func_wrap(
            "crypto",
            "blake3_hash",
            |mut caller: Caller<'_, crate::wasm::runtime::WasiState>, data_ptr: i32, data_len: i32, out_ptr: i32| -> Result<()> {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => anyhow::bail!("Failed to find memory export"),
                };
                
                // Read input data
                let data = memory
                    .data(&caller)
                    .get(data_ptr as usize..(data_ptr + data_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for input"))?;
                
                // Compute hash
                let hash = blake3::hash(data);
                let hash_bytes = hash.as_bytes();
                
                // Write hash to output
                let mem_data = memory.data_mut(&mut caller);
                let out_slice = mem_data
                    .get_mut(out_ptr as usize..(out_ptr + 32) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for output"))?;
                
                out_slice.copy_from_slice(hash_bytes);
                Ok(())
            },
        )?;
        
        debug!("Added crypto functions to linker");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_host_functions_creation() {
        let host_funcs = HostFunctions::new();
        let logs = host_funcs.get_logs().await;
        assert_eq!(logs.len(), 0);
    }
    
    #[tokio::test]
    async fn test_clear_logs() {
        let host_funcs = HostFunctions::new();
        host_funcs.clear_logs().await;
        let logs = host_funcs.get_logs().await;
        assert_eq!(logs.len(), 0);
    }
}
