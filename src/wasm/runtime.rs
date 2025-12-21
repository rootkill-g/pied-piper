use anyhow::{Context, Result};
use std::time::Duration;
use wasmtime::*;

/// WASI state for the store - simplified for now
#[derive(Default)]
pub struct WasiState {
    // Placeholder for WASI context
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
    
    /// Load a Wasm module from bytes
    pub fn load_module(&self, wasm_bytes: &[u8]) -> Result<Module> {
        Module::new(&self.engine, wasm_bytes)
            .context("Failed to load Wasm module")
    }
    
    /// Instantiate a module with WASI
    pub async fn instantiate_with_wasi(
        &self,
        store: &mut Store<WasiState>,
        module: &Module,
    ) -> Result<Instance> {
        // Create a linker
        let linker = Linker::new(&self.engine);
        
        // TODO: Add WASI functions when we set up proper WASI support
        // For now, instantiate without WASI
        
        // Instantiate the module
        let instance = linker
            .instantiate_async(store, module)
            .await
            .context("Failed to instantiate module")?;
        
        Ok(instance)
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
        
        // Call the function
        func.call_async(&mut *store, args, &mut results)
            .await
            .context(format!("Failed to execute function '{}'", function_name))?;
        
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
