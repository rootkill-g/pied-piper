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
            "env",
            "host_log",
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
            "env",
            "host_now_millis",
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
            "env",
            "host_random_u32",
            |_caller: Caller<'_, crate::wasm::runtime::WasiState>| -> u32 {
                use rand::Rng;
                rand::rng().random()
            },
        )?;
        
        // Add HTTP client functions
        NetworkHostFunctions::add_http_functions(linker, self.http_client.clone())?;
        
        // Add new-style HTTP functions for core modules (with separate status/body)
        NetworkHostFunctions::add_http_functions_v2(linker, self.http_client.clone())?;
        
        // Add storage functions (both v1 and v2)
        StorageHostFunctions::add_storage_functions(linker, self.storage.clone())?;
        StorageHostFunctions::add_storage_functions_v2(linker, self.storage.clone())?;
        
        // Add crypto functions (both v1 and v2)
        CryptoHostFunctions::add_crypto_functions(linker)?;
        CryptoHostFunctions::add_crypto_functions_v2(linker)?;
        
        debug!("Added all host functions to linker");
        Ok(())
    }

    /// Add host functions to a component linker (WASI P2)
    pub fn add_to_component_linker(
        &self,
        linker: &mut ComponentLinker<crate::wasm::runtime::WasiState>,
    ) -> Result<()> {
        // For components, we need to provide interfaces under the package namespace
        // The WIT file defines: package component:api-client
        // So interfaces are: component:api-client/host, component:api-client/http, etc.
        
        let mut host_instance = linker.instance("component:api-client/host")?;
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

        host_instance.func_wrap("now-millis", |_store, (): ()| -> Result<(i64,)> {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            Ok((now,))
        })?;

        host_instance.func_wrap("random-u32", |_store, (): ()| -> Result<(u32,)> {
            use rand::Rng;
            Ok((rand::rng().random(),))
        })?;

        let mut http_instance = linker.instance("component:api-client/http")?;
        let http_client_get = self.http_client.clone();
        http_instance.func_wrap_async(
            "get",
            move |_store, (url,): (String,)| {
                let client = http_client_get.clone();
                Box::new(async move {
                    match client.get(&url).send().await {
                        Ok(response) => {
                            let status = response.status().as_u16() as u32;
                            match response.bytes().await {
                                Ok(bytes) => Ok((status, bytes.to_vec())),
                                Err(_) => Ok((500u32, Vec::new())),
                            }
                        }
                        Err(_) => Ok((0u32, Vec::new())),
                    }
                })
            },
        )?;

        let http_client_post = self.http_client.clone();
        http_instance.func_wrap_async(
            "post",
            move |_store, (url, body): (String, Vec<u8>)| {
                let client = http_client_post.clone();
                Box::new(async move {
                    match client.post(&url).body(body).send().await {
                        Ok(response) => {
                            let status = response.status().as_u16() as u32;
                            match response.bytes().await {
                                Ok(bytes) => Ok((status, bytes.to_vec())),
                                Err(_) => Ok((500u32, Vec::new())),
                            }
                        }
                        Err(_) => Ok((0u32, Vec::new())),
                    }
                })
            },
        )?;

        let mut storage_instance = linker.instance("component:api-client/storage")?;
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
        storage_instance.func_wrap_async("list-count", move |_store, (): ()| {
            let storage = storage_count.clone();
            Box::new(async move { Ok((storage.read().await.len() as u32,)) })
        })?;

        let mut crypto_instance = linker.instance("component:api-client/crypto")?;
        crypto_instance.func_wrap(
            "blake3-hash",
            |_store, (data,): (Vec<u8>,)| -> Result<(Vec<u8>,)> {
            let hash = blake3::hash(&data);
            Ok((hash.as_bytes().to_vec(),))
        })?;

        debug!("Added component host functions to linker (component:api-client namespace)");
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
    
    /// Add v2 HTTP functions with separate status return and body length pointer
    pub fn add_http_functions_v2(
        linker: &mut Linker<crate::wasm::runtime::WasiState>,
        http_client: reqwest::Client,
    ) -> Result<()> {
        // host_http_get(url_ptr, url_len, body_ptr, body_len_ptr) -> status
        let http_client_get = http_client.clone();
        linker.func_wrap(
            "env",
            "host_http_get",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>,
                  url_ptr: i32,
                  url_len: i32,
                  body_ptr: i32,
                  body_len_ptr: i32| -> Result<u32> {
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
                
                debug!("host_http_get: {}", url);
                
                // Make HTTP request
                let client = http_client_get.clone();
                let (status, body) = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        match client.get(&url).send().await {
                            Ok(response) => {
                                let status = response.status().as_u16() as u32;
                                match response.bytes().await {
                                    Ok(bytes) => (status, bytes.to_vec()),
                                    Err(_) => (500, vec![]),
                                }
                            }
                            Err(_) => (0, vec![]),
                        }
                    })
                });
                
                // Read the current body_len (max capacity)
                let mem_data = memory.data(&caller);
                let body_len_bytes = mem_data
                    .get(body_len_ptr as usize..(body_len_ptr as usize + 4))
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for body_len"))?;
                let max_body_len = u32::from_le_bytes([
                    body_len_bytes[0],
                    body_len_bytes[1],
                    body_len_bytes[2],
                    body_len_bytes[3],
                ]) as usize;
                
                // Write body to memory
                let write_len = std::cmp::min(body.len(), max_body_len);
                let mem_data_mut = memory.data_mut(&mut caller);
                let body_slice = mem_data_mut
                    .get_mut(body_ptr as usize..(body_ptr as usize + write_len))
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for body"))?;
                body_slice.copy_from_slice(&body[..write_len]);
                
                // Write actual body length
                let len_slice = mem_data_mut
                    .get_mut(body_len_ptr as usize..(body_len_ptr as usize + 4))
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for body_len write"))?;
                len_slice.copy_from_slice(&(write_len as u32).to_le_bytes());
                
                Ok(status)
            },
        )?;
        
        // host_http_post(url_ptr, url_len, body_in_ptr, body_in_len, body_out_ptr, body_out_len_ptr) -> status
        let http_client_post = http_client.clone();
        linker.func_wrap(
            "env",
            "host_http_post",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>,
                  url_ptr: i32,
                  url_len: i32,
                  body_in_ptr: i32,
                  body_in_len: i32,
                  body_out_ptr: i32,
                  body_out_len_ptr: i32| -> Result<u32> {
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
                let body_in_bytes = memory
                    .data(&caller)
                    .get(body_in_ptr as usize..(body_in_ptr + body_in_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for body_in"))?
                    .to_vec();
                
                debug!("host_http_post: {} ({} bytes)", url, body_in_bytes.len());
                
                // Make HTTP request
                let client = http_client_post.clone();
                let (status, body_out) = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        match client.post(&url).body(body_in_bytes).send().await {
                            Ok(response) => {
                                let status = response.status().as_u16() as u32;
                                match response.bytes().await {
                                    Ok(bytes) => (status, bytes.to_vec()),
                                    Err(_) => (500, vec![]),
                                }
                            }
                            Err(_) => (0, vec![]),
                        }
                    })
                });
                
                // Read max output length
                let mem_data = memory.data(&caller);
                let body_out_len_bytes = mem_data
                    .get(body_out_len_ptr as usize..(body_out_len_ptr as usize + 4))
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for body_out_len"))?;
                let max_body_out_len = u32::from_le_bytes([
                    body_out_len_bytes[0],
                    body_out_len_bytes[1],
                    body_out_len_bytes[2],
                    body_out_len_bytes[3],
                ]) as usize;
                
                // Write response body
                let write_len = std::cmp::min(body_out.len(), max_body_out_len);
                let mem_data_mut = memory.data_mut(&mut caller);
                let body_slice = mem_data_mut
                    .get_mut(body_out_ptr as usize..(body_out_ptr as usize + write_len))
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for body_out"))?;
                body_slice.copy_from_slice(&body_out[..write_len]);
                
                // Write actual output length
                let len_slice = mem_data_mut
                    .get_mut(body_out_len_ptr as usize..(body_out_len_ptr as usize + 4))
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for body_out_len write"))?;
                len_slice.copy_from_slice(&(write_len as u32).to_le_bytes());
                
                Ok(status)
            },
        )?;
        
        debug!("Added HTTP v2 functions to linker");
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
    
    /// Add v2 storage functions with length pointer pattern
    pub fn add_storage_functions_v2(
        linker: &mut Linker<crate::wasm::runtime::WasiState>,
        storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    ) -> Result<()> {
        // host_storage_get(key_ptr, key_len, val_ptr, val_len_ptr) -> 1 if found, 0 if not
        let storage_get = storage.clone();
        linker.func_wrap(
            "env",
            "host_storage_get",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>,
                  key_ptr: i32,
                  key_len: i32,
                  val_ptr: i32,
                  val_len_ptr: i32| -> Result<u32> {
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
                
                debug!("host_storage_get: {}", key);
                
                // Get from storage
                let storage_clone = storage_get.clone();
                let value = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let store = storage_clone.read().await;
                        store.get(&key).cloned()
                    })
                });
                
                match value {
                    Some(val) => {
                        // Read max value length
                        let mem_data = memory.data(&caller);
                        let val_len_bytes = mem_data
                            .get(val_len_ptr as usize..(val_len_ptr as usize + 4))
                            .ok_or_else(|| anyhow::anyhow!("Invalid memory access for val_len"))?;
                        let max_val_len = u32::from_le_bytes([
                            val_len_bytes[0],
                            val_len_bytes[1],
                            val_len_bytes[2],
                            val_len_bytes[3],
                        ]) as usize;
                        
                        // Write value to memory
                        let write_len = std::cmp::min(val.len(), max_val_len);
                        let mem_data_mut = memory.data_mut(&mut caller);
                        let val_slice = mem_data_mut
                            .get_mut(val_ptr as usize..(val_ptr as usize + write_len))
                            .ok_or_else(|| anyhow::anyhow!("Invalid memory access for val"))?;
                        val_slice.copy_from_slice(&val[..write_len]);
                        
                        // Write actual length
                        let len_slice = mem_data_mut
                            .get_mut(val_len_ptr as usize..(val_len_ptr as usize + 4))
                            .ok_or_else(|| anyhow::anyhow!("Invalid memory access for val_len write"))?;
                        len_slice.copy_from_slice(&(write_len as u32).to_le_bytes());
                        
                        Ok(1) // Found
                    }
                    None => Ok(0), // Not found
                }
            },
        )?;
        
        // host_storage_set(key_ptr, key_len, val_ptr, val_len) -> 1 on success
        let storage_set = storage.clone();
        linker.func_wrap(
            "env",
            "host_storage_set",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>,
                  key_ptr: i32,
                  key_len: i32,
                  val_ptr: i32,
                  val_len: i32| -> Result<u32> {
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
                let val_bytes = memory
                    .data(&caller)
                    .get(val_ptr as usize..(val_ptr + val_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for val"))?
                    .to_vec();
                
                debug!("host_storage_set: {} ({} bytes)", key, val_bytes.len());
                
                // Store
                let storage_clone = storage_set.clone();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let mut store = storage_clone.write().await;
                        store.insert(key, val_bytes);
                    })
                });
                
                Ok(1)
            },
        )?;
        
        // host_storage_delete(key_ptr, key_len) -> 1 if existed, 0 if not
        let storage_delete = storage.clone();
        linker.func_wrap(
            "env",
            "host_storage_delete",
            move |mut caller: Caller<'_, crate::wasm::runtime::WasiState>,
                  key_ptr: i32,
                  key_len: i32| -> Result<u32> {
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
                
                debug!("host_storage_delete: {}", key);
                
                // Delete
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
        
        // host_storage_count() -> count
        let storage_count = storage.clone();
        linker.func_wrap(
            "env",
            "host_storage_count",
            move |_caller: Caller<'_, crate::wasm::runtime::WasiState>| -> Result<u32> {
                let storage_clone = storage_count.clone();
                let count = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let store = storage_clone.read().await;
                        store.len()
                    })
                });
                
                Ok(count as u32)
            },
        )?;
        
        debug!("Added storage v2 functions to linker");
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
    
    /// Add v2 crypto functions for core modules
    pub fn add_crypto_functions_v2(linker: &mut Linker<crate::wasm::runtime::WasiState>) -> Result<()> {
        // host_blake3_hash(data_ptr, data_len, hash_ptr)
        linker.func_wrap(
            "env",
            "host_blake3_hash",
            |mut caller: Caller<'_, crate::wasm::runtime::WasiState>, 
             data_ptr: i32, 
             data_len: i32, 
             hash_ptr: i32| -> Result<()> {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => anyhow::bail!("Failed to find memory export"),
                };
                
                // Read input data
                let data = memory
                    .data(&caller)
                    .get(data_ptr as usize..(data_ptr + data_len) as usize)
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for input data"))?;
                
                debug!("host_blake3_hash: {} bytes", data.len());
                
                // Compute hash
                let hash = blake3::hash(data);
                let hash_bytes = hash.as_bytes();
                
                // Write hash to output (32 bytes)
                let mem_data = memory.data_mut(&mut caller);
                let hash_slice = mem_data
                    .get_mut(hash_ptr as usize..(hash_ptr as usize + 32))
                    .ok_or_else(|| anyhow::anyhow!("Invalid memory access for hash output"))?;
                hash_slice.copy_from_slice(hash_bytes);
                
                Ok(())
            },
        )?;
        
        debug!("Added crypto v2 functions to linker");
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
