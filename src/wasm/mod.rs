//! WebAssembly runtime and execution environment
//!
//! This module provides:
//! - Wasm module loading and execution
//! - WASI support for system access
//! - Resource limits (CPU, memory, time)
//! - Host function bindings
//! - Sandboxed execution environment

mod host;
pub mod loader; // Make loader public for content module
mod runtime;
mod sandbox;

pub use host::HostFunctions;
pub use loader::{ModuleCid, ModuleInfo, ModuleLoader};
pub use runtime::{WasmRuntime, WasmRuntimeConfig};
pub use sandbox::{ExecutionContext, ExecutionResult, ResourceLimits, Sandbox};
