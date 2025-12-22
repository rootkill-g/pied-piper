use anyhow::{Context, Result};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wasmtime::*;
use wasmtime::component::{Component, Linker as ComponentLinker, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView, WasiCtxView};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::{self, pipe::{MemoryInputPipe, MemoryOutputPipe}, bindings::Command};
use super::host::HostFunctions;

/// WASI state for the store implementing WasiView
pub struct WasiState {
    /// WASI context
    wasi_ctx: WasiCtx,

    /// Resource table for component model
    resource_table: ResourceTable,

    /// WASI Preview 1 context (core modules)
    wasi_p1: WasiP1Ctx,
    
    /// Output buffer (stdout) - for capturing output
    pub stdout_buffer: Arc<Mutex<Vec<u8>>>,
    
    /// Error buffer (stderr) - for capturing output
    pub stderr_buffer: Arc<Mutex<Vec<u8>>>,
    
    /// Stdout pipe for reading output
    pub stdout_pipe: Arc<MemoryOutputPipe>,
    
    /// Stderr pipe for reading output
    pub stderr_pipe: Arc<MemoryOutputPipe>,
}

impl WasiState {
    /// Create a new WASI state with custom stdin data
    pub fn with_stdin(stdin_data: Vec<u8>) -> Self {
        let stdout_buffer = Arc::new(Mutex::new(Vec::new()));
        let stderr_buffer = Arc::new(Mutex::new(Vec::new()));
        
        let stdout_pipe = Arc::new(MemoryOutputPipe::new(4096));
        let stderr_pipe = Arc::new(MemoryOutputPipe::new(4096));
        let stdin_pipe = MemoryInputPipe::new(stdin_data.clone());
        
        let wasi_ctx = WasiCtxBuilder::new()
            .stdin(stdin_pipe)
            .stdout(stdout_pipe.clone())
            .stderr(stderr_pipe.clone())
            .build();

        let wasi_p1 = WasiCtxBuilder::new()
            .stdin(MemoryInputPipe::new(stdin_data))
            .stdout(stdout_pipe.clone())
            .stderr(stderr_pipe.clone())
            .build_p1();
        
        Self {
            wasi_ctx,
            resource_table: ResourceTable::new(),
            wasi_p1,
            stdout_buffer,
            stderr_buffer,
            stdout_pipe,
            stderr_pipe,
        }
    }
    
    /// Create a new WASI state with empty buffers
    pub fn new() -> Self {
        Self::with_stdin(Vec::new())
    }
    
    /// Get stdout contents
    pub fn get_stdout(&self) -> Vec<u8> {
        // Try to get contents from the pipe
        self.stdout_pipe.contents().to_vec()
    }
    
    /// Get stderr contents
    pub fn get_stderr(&self) -> Vec<u8> {
        // Try to get contents from the pipe
        self.stderr_pipe.contents().to_vec()
    }
    
    /// Clear output buffers
    pub fn clear_output(&self) {
        self.stdout_buffer.lock().unwrap().clear();
        self.stderr_buffer.lock().unwrap().clear()
    }
}

impl Default for WasiState {
    fn default() -> Self {
        Self::new()
    }
}

impl WasiView for WasiState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resource_table,
        }
    }
}

impl ResourceLimiter for WasiState {
    fn memory_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool, Error> {
        // Default limit: 512MB
        Ok(desired <= 512 * 1024 * 1024)
    }
    
    fn table_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool, Error> {
        // Default limit: 10000 elements
        Ok(desired <= 10000)
    }
}

/// Configuration for the Wasm runtime
#[derive(Debug, Clone)]
pub struct WasmRuntimeConfig {
    /// Maximum memory in bytes (default: 128MB)
    pub max_memory_bytes: usize,
    
    /// Maximum execution time
    pub max_execution_time: Duration,
    
    /// Enable async support
    pub enable_async: bool,
    
    /// Enable WASI
    pub enable_wasi: bool,
    
    /// Enable fuel metering for CPU limits
    pub enable_fuel: bool,
    
    /// Initial fuel amount (if fuel enabled)
    pub initial_fuel: u64,
}

impl Default for WasmRuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 128 * 1024 * 1024, // 128MB
            max_execution_time: Duration::from_secs(30),
            enable_async: true,
            enable_wasi: true,
            enable_fuel: true,
            initial_fuel: 10_000_000, // 10M instructions
        }
    }
}

/// Main WebAssembly runtime
pub struct WasmRuntime {
    engine: Engine,
    config: WasmRuntimeConfig,
}

impl WasmRuntime {
    /// Create a new Wasm runtime with the given configuration
    pub fn new(config: WasmRuntimeConfig) -> Result<Self> {
        let mut engine_config = Config::new();
        
        // Enable async support if configured
        if config.enable_async {
            engine_config.async_support(true);
        }
        
        // Enable fuel metering for CPU limits
        if config.enable_fuel {
            engine_config.consume_fuel(true);
        }
        
        // Enable Cranelift optimizations
        engine_config.cranelift_opt_level(OptLevel::Speed);
        
        // Enable debug info for better error messages
        engine_config.debug_info(true);
        
        // Enable parallel compilation
        engine_config.parallel_compilation(true);
        
        let engine = Engine::new(&engine_config)
            .context("Failed to create Wasmtime engine")?;
        
        Ok(Self { engine, config })
    }
    
    /// Create a new store with resource limits
    pub fn create_store(&self) -> Result<Store<WasiState>> {
        let state = WasiState::default();
        let mut store = Store::new(&self.engine, state);
        
        // Set fuel limit if enabled
        if self.config.enable_fuel {
            store.set_fuel(self.config.initial_fuel)
                .context("Failed to set fuel limit")?;
        }
        
        // Set resource limits
        store.limiter(|state| state);
        
        Ok(store)
    }
    
    /// Create a new store with stdin data pre-loaded
    pub fn create_store_with_stdin(&self, stdin_data: Vec<u8>) -> Result<Store<WasiState>> {
        let state = WasiState::with_stdin(stdin_data);
        let mut store = Store::new(&self.engine, state);
        
        // Set fuel limit if enabled
        if self.config.enable_fuel {
            store.set_fuel(self.config.initial_fuel)
                .context("Failed to set fuel limit")?;
        }
        
        // Set resource limits
        store.limiter(|state| state);
        
        Ok(store)
    }
    
    /// Get stdout data from a store
    pub fn get_stdout(&self, store: &Store<WasiState>) -> Vec<u8> {
        store.data().get_stdout()
    }
    
    /// Get stderr data from a store
    pub fn get_stderr(&self, store: &Store<WasiState>) -> Vec<u8> {
        store.data().get_stderr()
    }
    
    /// Load a Wasm module from bytes
    pub fn load_module(&self, wasm_bytes: &[u8]) -> Result<Module> {
        Module::new(&self.engine, wasm_bytes)
            .context("Failed to load Wasm module")
    }
    
    /// Load a Wasm component from bytes (WASI Preview 2)
    pub fn load_component(&self, wasm_bytes: &[u8]) -> Result<Component> {
        Component::new(&self.engine, wasm_bytes)
            .context(format!("Failed to load Wasm component (size: {} bytes)", wasm_bytes.len()))
    }
    
    /// Instantiate a module with WASI  (Legacy - for backward compat with P1)
    pub async fn instantiate_with_wasi(
        &self,
        store: &mut Store<WasiState>,
        module: &Module,
    ) -> Result<Instance> {
        // Create a linker for core modules
        let mut linker = Linker::new(&self.engine);

        // Wire host functions for core modules (HTTP, storage, crypto, etc.)
        let host_functions = HostFunctions::new();
        host_functions.add_to_linker(&mut linker)?;

        // Wire WASI Preview 1 for core modules
        wasmtime_wasi::p1::add_to_linker_async(&mut linker, |state| &mut state.wasi_p1)
            .context("Failed to add WASI P1 to core linker")?;

        // Instantiate the module
        let instance = linker
            .instantiate_async(store, module)
            .await
            .context("Failed to instantiate module")?;
        
        Ok(instance)
    }
    
    /// Instantiate a component with WASI Preview 2 (Primary method)
    pub async fn instantiate_component_with_wasi(
        &self,
        store: &mut Store<WasiState>,
        component: &Component,
    ) -> Result<wasmtime::component::Instance> {
        // Create a component linker
        let mut linker = ComponentLinker::new(&self.engine);
        
        // Add WASI Preview 2 interfaces to the linker
        p2::add_to_linker_async(&mut linker)
            .context("Failed to add WASI P2 to component linker")?;

        // Add custom host functions for components
        let host_functions = HostFunctions::new();
        host_functions.add_to_component_linker(&mut linker)?;
        
        // Instantiate the component
        let instance = linker
            .instantiate_async(store, component)
            .await
            .context("Failed to instantiate component")?;
        
        Ok(instance)
    }
    
    /// Execute a WASI P2 component using the Command pattern
    pub async fn execute_component_command(
        &self,
        store: &mut Store<WasiState>,
        component: &Component,
    ) -> Result<()> {
        // Create a component linker
        let mut linker = ComponentLinker::new(&self.engine);
        
        // Add WASI Preview 2 interfaces to the linker
        p2::add_to_linker_async(&mut linker)
            .context("Failed to add WASI P2 to component linker")?;

        // Add custom host functions for components
        let host_functions = HostFunctions::new();
        host_functions.add_to_component_linker(&mut linker)?;
        
        // Instantiate the Command component
        let command = Command::instantiate_async(&mut *store, component, &linker)
            .await
            .context("Failed to instantiate Command component")?;
        
        // Execute the component's run function with timeout
        let result = self
            .run_with_timeout(
                async {
                    command
                        .wasi_cli_run()
                        .call_run(&mut *store)
                        .await
                        .context("Failed to call component run function")
                },
                "component command",
            )
            .await?;
        
        // Check the result
        match result {
            Ok(()) => Ok(()),
            Err(()) => Err(anyhow::anyhow!("Component returned error exit code")),
        }
    }
    
    /// Execute a WASI P2 component's exported function (typically for Command pattern)
    pub async fn execute_component_function(
        &self,
        store: &mut Store<WasiState>,
        instance: &wasmtime::component::Instance,
        function_name: &str,
    ) -> Result<()> {
        // Get the typed function from the component
        // For Command pattern, we typically call functions that take no args and return Result
        let func = instance
            .get_typed_func::<(), ()>(&mut *store, function_name)
            .context(format!("Failed to get function '{}' from component", function_name))?;
        
        // Call the function with timeout
        self.run_with_timeout(
            async {
                func.call_async(&mut *store, ())
                    .await
                    .context(format!(
                        "Failed to execute component function '{}'",
                        function_name
                    ))
            },
            &format!("component function '{}'", function_name),
        )
        .await?;
        
        Ok(())
    }
    
    /// Execute a function in the module
    pub async fn execute_function(
        &self,
        store: &mut Store<WasiState>,
        instance: &Instance,
        function_name: &str,
        args: &[Val],
    ) -> Result<Vec<Val>> {
        // Get the function
        let func = instance
            .get_func(&mut *store, function_name)
            .context(format!("Function '{}' not found", function_name))?;
        
        // Prepare results buffer
        let mut results = vec![Val::I32(0); func.ty(&*store).results().len()];
        
        // Call the function with timeout
        self.run_with_timeout(
            async {
                func.call_async(&mut *store, args, &mut results)
                    .await
                    .context(format!("Failed to execute function '{}'", function_name))
            },
            &format!("function '{}'", function_name),
        )
        .await?;
        
        Ok(results)
    }
    
    /// Get remaining fuel (if fuel metering is enabled)
    pub fn get_remaining_fuel(&self, store: &Store<WasiState>) -> Option<u64> {
        if self.config.enable_fuel {
            store.get_fuel().ok()
        } else {
            None
        }
    }

    async fn run_with_timeout<F, T>(&self, fut: F, action: &str) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        if self.config.max_execution_time == Duration::from_secs(0) {
            return fut.await;
        }

        match tokio::time::timeout(self.config.max_execution_time, fut).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!(
                "Execution timed out after {:?} during {}",
                self.config.max_execution_time,
                action
            )),
        }
    }
    
    /// Get the runtime configuration
    pub fn config(&self) -> &WasmRuntimeConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_runtime() {
        let config = WasmRuntimeConfig::default();
        let runtime = WasmRuntime::new(config);
        assert!(runtime.is_ok());
    }
    
    #[tokio::test]
    async fn test_create_store() {
        let config = WasmRuntimeConfig::default();
        let runtime = WasmRuntime::new(config).unwrap();
        let store = runtime.create_store();
        assert!(store.is_ok());
    }
}
