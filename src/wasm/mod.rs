//! WebAssembly runtime and execution environment
//!
//! This module provides:
//! - Wasm module loading and execution
//! - WASI support for system access
//! - Resource limits (CPU, memory, time)
//! - Host function bindings
//! - Sandboxed execution environment

mod runtime;
mod loader;
mod sandbox;
mod host;

pub use runtime::{WasmRuntime, WasmRuntimeConfig};
pub use loader::{ModuleLoader, ModuleInfo, ModuleCid};
pub use sandbox::{ResourceLimits, ExecutionContext, ExecutionResult, Sandbox};
pub use host::HostFunctions;
