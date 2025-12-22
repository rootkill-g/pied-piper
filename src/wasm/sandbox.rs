use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Resource limits for Wasm execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in bytes
    pub max_memory_bytes: usize,

    /// Maximum execution time
    pub max_execution_time: Duration,

    /// Maximum fuel (instruction count)
    pub max_fuel: u64,

    /// Maximum stack depth
    pub max_stack_depth: usize,

    /// Maximum number of table elements
    pub max_table_elements: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 128 * 1024 * 1024, // 128MB
            max_execution_time: Duration::from_secs(30),
            max_fuel: 10_000_000, // 10M instructions
            max_stack_depth: 1024,
            max_table_elements: 10_000,
        }
    }
}

impl ResourceLimits {
    /// Create conservative limits (for untrusted code)
    pub fn conservative() -> Self {
        Self {
            max_memory_bytes: 16 * 1024 * 1024, // 16MB
            max_execution_time: Duration::from_secs(5),
            max_fuel: 1_000_000, // 1M instructions
            max_stack_depth: 256,
            max_table_elements: 1_000,
        }
    }

    /// Create permissive limits (for trusted code)
    pub fn permissive() -> Self {
        Self {
            max_memory_bytes: 512 * 1024 * 1024, // 512MB
            max_execution_time: Duration::from_secs(120),
            max_fuel: 100_000_000, // 100M instructions
            max_stack_depth: 4096,
            max_table_elements: 100_000,
        }
    }
}

/// Execution context with resource tracking
pub struct ExecutionContext {
    /// Resource limits
    pub limits: ResourceLimits,

    /// Start time of execution
    start_time: Instant,

    /// Initial fuel amount
    initial_fuel: u64,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            initial_fuel: limits.max_fuel,
            limits,
            start_time: Instant::now(),
        }
    }

    /// Check if execution time limit is exceeded
    pub fn is_time_exceeded(&self) -> bool {
        self.start_time.elapsed() > self.limits.max_execution_time
    }

    /// Get elapsed time
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Calculate fuel consumed
    pub fn fuel_consumed(&self, remaining_fuel: u64) -> u64 {
        self.initial_fuel.saturating_sub(remaining_fuel)
    }
}

/// Result of Wasm execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether execution succeeded
    pub success: bool,

    /// Return value (if any)
    pub return_value: Option<ExecutionValue>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Execution statistics
    pub stats: ExecutionStats,
}

/// Statistics from execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Time taken
    pub duration: Duration,

    /// Memory used (peak)
    pub memory_used: usize,

    /// Fuel consumed (instructions executed)
    pub fuel_consumed: u64,

    /// Whether any limits were hit
    pub limits_hit: Vec<String>,
}

impl ExecutionStats {
    pub fn new() -> Self {
        Self {
            duration: Duration::from_secs(0),
            memory_used: 0,
            fuel_consumed: 0,
            limits_hit: Vec::new(),
        }
    }
}

impl Default for ExecutionStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ExecutionValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    None,
}

impl From<wasmtime::Val> for ExecutionValue {
    fn from(val: wasmtime::Val) -> Self {
        match val {
            wasmtime::Val::I32(v) => ExecutionValue::I32(v),
            wasmtime::Val::I64(v) => ExecutionValue::I64(v),
            wasmtime::Val::F32(v) => ExecutionValue::F32(f32::from_bits(v)),
            wasmtime::Val::F64(v) => ExecutionValue::F64(f64::from_bits(v)),
            _ => ExecutionValue::None,
        }
    }
}

/// Sandbox environment for safe Wasm execution
pub struct Sandbox {
    /// Resource limits
    limits: ResourceLimits,
}

impl Sandbox {
    /// Create a new sandbox with the given limits
    pub fn new(limits: ResourceLimits) -> Self {
        Self { limits }
    }

    /// Create a sandbox with default limits
    pub fn with_defaults() -> Self {
        Self::new(ResourceLimits::default())
    }

    /// Create a conservative sandbox for untrusted code
    pub fn conservative() -> Self {
        Self::new(ResourceLimits::conservative())
    }

    /// Create a permissive sandbox for trusted code
    pub fn permissive() -> Self {
        Self::new(ResourceLimits::permissive())
    }

    /// Get the resource limits
    pub fn limits(&self) -> &ResourceLimits {
        &self.limits
    }

    /// Validate that a module doesn't exceed resource requirements
    pub fn validate_module(&self, module: &wasmtime::Module) -> Result<()> {
        // Check memory limits
        for memory_type in module.exports().filter_map(|e| {
            if let wasmtime::ExternType::Memory(m) = e.ty() {
                Some(m)
            } else {
                None
            }
        }) {
            let min_pages = memory_type.minimum();
            let min_bytes = min_pages as usize * 65536; // 64KB per page

            if min_bytes > self.limits.max_memory_bytes {
                anyhow::bail!(
                    "Module requires {} bytes of memory, but limit is {}",
                    min_bytes,
                    self.limits.max_memory_bytes
                );
            }
        }

        // Check table limits
        for table_type in module.exports().filter_map(|e| {
            if let wasmtime::ExternType::Table(t) = e.ty() {
                Some(t)
            } else {
                None
            }
        }) {
            let min_elements = table_type.minimum();

            if min_elements > self.limits.max_table_elements {
                anyhow::bail!(
                    "Module requires {} table elements, but limit is {}",
                    min_elements,
                    self.limits.max_table_elements
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_memory_bytes, 128 * 1024 * 1024);
        assert_eq!(limits.max_execution_time, Duration::from_secs(30));
    }

    #[test]
    fn test_resource_limits_conservative() {
        let limits = ResourceLimits::conservative();
        assert_eq!(limits.max_memory_bytes, 16 * 1024 * 1024);
        assert_eq!(limits.max_execution_time, Duration::from_secs(5));
    }

    #[test]
    fn test_execution_context() {
        let limits = ResourceLimits::default();
        let ctx = ExecutionContext::new(limits);
        assert!(!ctx.is_time_exceeded());
    }

    #[test]
    fn test_sandbox_creation() {
        let sandbox = Sandbox::with_defaults();
        assert_eq!(sandbox.limits().max_memory_bytes, 128 * 1024 * 1024);

        let conservative = Sandbox::conservative();
        assert_eq!(conservative.limits().max_memory_bytes, 16 * 1024 * 1024);
    }
}
