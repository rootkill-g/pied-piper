use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use wasmtime::*;
use wasmtime::component::Linker as ComponentLinker;

/// Host functions that can be called from Wasm modules
pub struct HostFunctions {
    /// Shared state (for future use with network, storage, etc.)
    state: Arc<RwLock<HostState>>,
    /// HTTP client for network requests
    http_client: reqwest::Client,
    /// Key-value storage
    storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
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
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
            storage: Arc::new(RwLock::new(HashMap::new())),
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
        
        // Add HTTP client functions
        NetworkHostFunctions::add_http_functions(linker, self.http_client.clone())?;
        
        // Add storage functions
        StorageHostFunctions::add_storage_functions(linker, self.storage.clone())?;
        
        // Add crypto functions
        CryptoHostFunctions::add_crypto_functions(linker)?;
        
        debug!("Added all host functions to linker");
        Ok(())
    }

    /// Add host functions to a component linker (WASI P2)
    pub fn add_to_component_linker(
        &self,
        linker: &mut ComponentLinker<crate::wasm::runtime::WasiState>,
    ) -> Result<()> {
        let mut host_instance = linker.instance("host")?;
        let state_clone = self.state.clone();
        host_instance.func_wrap_async("log", move |_store, (message,): (String,)| {
            let state = state_clone.clone();
            Box::new(async move {
                info!("Wasm component log: {}", message);
                let mut state = state.write().await;
                state.log_messages.push(message);
                Ok(())
            })
        })?;

        host_instance.func_wrap("now_millis", |_store, (): ()| -> Result<(i64,)> {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            Ok((now,))
        })?;

        host_instance.func_wrap("random_u32", |_store, (): ()| -> Result<(u32,)> {
            use rand::Rng;
            Ok((rand::rng().random(),))
        })?;

        let mut http_instance = linker.instance("http")?;
        let http_client_get = self.http_client.clone();
        http_instance.func_wrap_async("get", move |_store, (url,): (String,)| {
            let client = http_client_get.clone();
            Box::new(async move {
                let (status, body) = match client.get(&url).send().await {
                    Ok(response) => {
                        let status = response.status().as_u16() as u32;
                        match response.bytes().await {
                            Ok(bytes) => (status, bytes.to_vec()),
                            Err(_) => (500, Vec::new()),
                        }
                    }
                    Err(_) => (0, Vec::new()),
                };
                Ok((status, body))
            })
        })?;

        let http_client_post = self.http_client.clone();
        http_instance.func_wrap_async(
            "post",
            move |_store, (url, body): (String, Vec<u8>)| {
                let client = http_client_post.clone();
                Box::new(async move {
                    let (status, body) = match client.post(&url).body(body).send().await {
                        Ok(response) => {
                            let status = response.status().as_u16() as u32;
                            match response.bytes().await {
                                Ok(bytes) => (status, bytes.to_vec()),
                                Err(_) => (500, Vec::new()),
                            }
                        }
                        Err(_) => (0, Vec::new()),
                    };
                    Ok((status, body))
                })
            },
        )?;

        let mut storage_instance = linker.instance("storage")?;
        let storage_get = self.storage.clone();
        storage_instance.func_wrap_async("get", move |_store, (key,): (String,)| {
            let storage = storage_get.clone();
            Box::new(async move {
                let storage = storage.read().await;
                match storage.get(&key) {
                    Some(value) => Ok((true, value.clone())),
                    None => Ok((false, Vec::new())),
                }
            })
        })?;

        let storage_set = self.storage.clone();
        storage_instance.func_wrap_async(
            "set",
            move |_store, (key, value): (String, Vec<u8>)| {
                let storage = storage_set.clone();
                Box::new(async move {
                    storage.write().await.insert(key, value);
                    Ok((true,))
                })
            },
        )?;

        let storage_delete = self.storage.clone();
        storage_instance.func_wrap_async("delete", move |_store, (key,): (String,)| {
            let storage = storage_delete.clone();
            Box::new(async move { Ok((storage.write().await.remove(&key).is_some(),)) })
        })?;

        let storage_count = self.storage.clone();
        storage_instance.func_wrap_async("list_count", move |_store, (): ()| {
            let storage = storage_count.clone();
            Box::new(async move { Ok((storage.read().await.len() as u32,)) })
        })?;

        let mut crypto_instance = linker.instance("crypto")?;
        crypto_instance.func_wrap(
            "blake3_hash",
            |_store, (data,): (Vec<u8>,)| -> Result<(Vec<u8>,)> {
            let hash = blake3::hash(&data);
            Ok((hash.as_bytes().to_vec(),))
        })?;

        debug!("Added component host functions to linker");
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
    /// HTTP client function implementation
    pub fn add_http_functions(
        linker: &mut Linker<crate::wasm::runtime::WasiState>,
        http_client: reqwest::Client,
    ) -> Result<()> {
        // HTTP GET function
        // Returns: status code (i32), response length written to output buffer
        let http_client_get = http_client.clone();
        linker.func_wrap(
            "http",
            "get",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>, 
                  url_ptr: i32, 
                  url_len: i32,
                  out_ptr: i32,
                  out_max_len: i32| -> Result<i64> {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => anyhow::bail!("Failed to find memory export"),
                };
                
                // Read URL from Wasm memory
                let url_bytes = memory
                    .data(&caller)
                    .get(url_ptr as usize..(url_ptr + url_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for URL"))?;
                
                let url = String::from_utf8_lossy(url_bytes).to_string();
                debug!("HTTP GET request to: {}", url);
                
                // Make HTTP request (blocking in place for async)
                let client = http_client_get.clone();
                let (status, body) = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        match client.get(&url).send().await {
                            Ok(response) => {
                                let status = response.status().as_u16() as i32;
                                match response.bytes().await {
                                    Ok(bytes) => (status, bytes.to_vec()),
                                    Err(_) => (500, vec![]),
                                }
                            }
                            Err(_) => (0, vec![]),
                        }
                    })
                });
                
                // Write response body to Wasm memory
                let write_len = std::cmp::min(body.len(), out_max_len as usize);
                let mem_data = memory.data_mut(&mut caller);
                let out_slice = mem_data
                    .get_mut(out_ptr as usize..(out_ptr as usize + write_len))
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for output"))?;
                
                out_slice.copy_from_slice(&body[..write_len]);
                
                // Return status in high 32 bits, length in low 32 bits
                let result = ((status as i64) << 32) | (write_len as i64);
                Ok(result)
            },
        )?;
        
        // HTTP POST function
        let http_client_post = http_client.clone();
        linker.func_wrap(
            "http",
            "post",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>,
                  url_ptr: i32,
                  url_len: i32,
                  body_ptr: i32,
                  body_len: i32,
                  out_ptr: i32,
                  out_max_len: i32| -> Result<i64> {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => anyhow::bail!("Failed to find memory export"),
                };
                
                // Read URL
                let url_bytes = memory
                    .data(&caller)
                    .get(url_ptr as usize..(url_ptr + url_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for URL"))?;
                let url = String::from_utf8_lossy(url_bytes).to_string();
                
                // Read request body
                let body_bytes = memory
                    .data(&caller)
                    .get(body_ptr as usize..(body_ptr + body_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for body"))?
                    .to_vec();
                
                debug!("HTTP POST request to: {}", url);
                
                // Make HTTP request
                let client = http_client_post.clone();
                let (status, response_body) = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        match client.post(&url).body(body_bytes).send().await {
                            Ok(response) => {
                                let status = response.status().as_u16() as i32;
                                match response.bytes().await {
                                    Ok(bytes) => (status, bytes.to_vec()),
                                    Err(_) => (500, vec![]),
                                }
                            }
                            Err(_) => (0, vec![]),
                        }
                    })
                });
                
                // Write response to Wasm memory
                let write_len = std::cmp::min(response_body.len(), out_max_len as usize);
                let mem_data = memory.data_mut(&mut caller);
                let out_slice = mem_data
                    .get_mut(out_ptr as usize..(out_ptr as usize + write_len))
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for output"))?;
                
                out_slice.copy_from_slice(&response_body[..write_len]);
                
                let result = ((status as i64) << 32) | (write_len as i64);
                Ok(result)
            },
        )?;
        
        debug!("Added HTTP functions to linker");
        Ok(())
    }
}

/// Host functions for storage operations (future implementation)  
pub struct StorageHostFunctions;

impl StorageHostFunctions {
    /// Key-value storage functions implementation
    pub fn add_storage_functions(
        linker: &mut Linker<crate::wasm::runtime::WasiState>,
        storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    ) -> Result<()> {
        // Storage GET function
        // Returns: length of value (or -1 if not found)
        let storage_get = storage.clone();
        linker.func_wrap(
            "storage",
            "get",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>,
                  key_ptr: i32,
                  key_len: i32,
                  out_ptr: i32,
                  out_max_len: i32| -> Result<i32> {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => anyhow::bail!("Failed to find memory export"),
                };
                
                // Read key from Wasm memory
                let key_bytes = memory
                    .data(&caller)
                    .get(key_ptr as usize..(key_ptr + key_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for key"))?;
                let key = String::from_utf8_lossy(key_bytes).to_string();
                
                debug!("Storage GET: {}", key);
                
                // Get value from storage
                let storage_clone = storage_get.clone();
                let value = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let store = storage_clone.read().await;
                        store.get(&key).cloned()
                    })
                });
                
                match value {
                    Some(val) => {
                        // Write value to Wasm memory
                        let write_len = std::cmp::min(val.len(), out_max_len as usize);
                        let mem_data = memory.data_mut(&mut caller);
                        let out_slice = mem_data
                            .get_mut(out_ptr as usize..(out_ptr as usize + write_len))
                            .ok_or_else(|| anyhow::anyhow!("Invalid memory access for output"))?;
                        
                        out_slice.copy_from_slice(&val[..write_len]);
                        Ok(write_len as i32)
                    }
                    None => Ok(-1),
                }
            },
        )?;
        
        // Storage SET function
        let storage_set = storage.clone();
        linker.func_wrap(
            "storage",
            "set",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>,
                  key_ptr: i32,
                  key_len: i32,
                  value_ptr: i32,
                  value_len: i32| -> Result<i32> {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => anyhow::bail!("Failed to find memory export"),
                };
                
                // Read key
                let key_bytes = memory
                    .data(&caller)
                    .get(key_ptr as usize..(key_ptr + key_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for key"))?;
                let key = String::from_utf8_lossy(key_bytes).to_string();
                
                // Read value
                let value_bytes = memory
                    .data(&caller)
                    .get(value_ptr as usize..(value_ptr + value_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for value"))?
                    .to_vec();
                
                debug!("Storage SET: {} ({} bytes)", key, value_bytes.len());
                
                // Store in storage
                let storage_clone = storage_set.clone();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let mut store = storage_clone.write().await;
                        store.insert(key, value_bytes);
                    })
                });
                
                Ok(0)
            },
        )?;
        
        // Storage DELETE function
        let storage_delete = storage.clone();
        linker.func_wrap(
            "storage",
            "delete",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>,
                  key_ptr: i32,
                  key_len: i32| -> Result<i32> {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => anyhow::bail!("Failed to find memory export"),
                };
                
                // Read key
                let key_bytes = memory
                    .data(&caller)
                    .get(key_ptr as usize..(key_ptr + key_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for key"))?;
                let key = String::from_utf8_lossy(key_bytes).to_string();
                
                debug!("Storage DELETE: {}", key);
                
                // Delete from storage
                let storage_clone = storage_delete.clone();
                let existed = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let mut store = storage_clone.write().await;
                        store.remove(&key).is_some()
                    })
                });
                
                Ok(if existed { 1 } else { 0 })
            },
        )?;
        
        // Storage LIST function (returns count of keys)
        let storage_list = storage.clone();
        linker.func_wrap(
            "storage",
            "list_count",
            move |_caller: Caller<'_, crate::wasm::runtime::WasiState>| -> Result<i32> {
                let storage_clone = storage_list.clone();
                let count = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let store = storage_clone.read().await;
                        store.len()
                    })
                });
                
                Ok(count as i32)
            },
        )?;
        
        debug!("Added storage functions to linker");
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
