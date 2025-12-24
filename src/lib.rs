// Library interface for pied-piper
// This allows benchmarks and external crates to access public APIs

pub mod bundle;
pub mod package;
pub mod wasm;
pub mod network;
pub mod gateway;
pub mod content;
pub mod crdt;
pub mod security;
pub mod storage;
pub mod metrics;
pub mod config;

// Re-export commonly used types
pub use package::{PiperNetPackage, PackageManifest, PackageMetadata, PackageType};
pub use wasm::loader::{ModuleLoader, ModuleCid, ModuleInfo};
pub use wasm::runtime::{WasmRuntime, WasmRuntimeConfig};
