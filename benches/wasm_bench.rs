use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use pied_piper::wasm::loader::ModuleCid;
use pied_piper::wasm::runtime::{WasmRuntime, WasmRuntimeConfig};

/// Create a minimal valid WASM module for benchmarking
fn create_minimal_wasm() -> Vec<u8> {
    // Minimal WASM module: magic + version + empty sections
    vec![
        0x00, 0x61, 0x73, 0x6d, // magic: \0asm
        0x01, 0x00, 0x00, 0x00, // version: 1
    ]
}

/// Create a more realistic WASM module with functions
fn create_wasm_with_functions(num_functions: usize) -> Vec<u8> {
    let mut wasm = create_minimal_wasm();
    
    // Add some padding to simulate larger modules
    wasm.extend(vec![0u8; num_functions * 100]);
    
    wasm
}

/// Benchmark module CID generation
fn bench_cid_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cid_generation");
    
    let sizes = vec![
        ("small_1KB", 1024),
        ("medium_10KB", 10 * 1024),
        ("large_100KB", 100 * 1024),
        ("xlarge_1MB", 1024 * 1024),
    ];
    
    for (label, size) in sizes {
        let wasm_bytes = create_wasm_with_functions(size / 100);
        
        group.throughput(Throughput::Bytes(wasm_bytes.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), &wasm_bytes, |b, bytes| {
            b.iter(|| {
                let cid = ModuleCid::from_bytes(black_box(bytes));
                black_box(cid);
            });
        });
    }
    
    group.finish();
}

/// Benchmark WASM runtime creation
fn bench_runtime_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("runtime_creation");
    
    group.bench_function("default_runtime", |b| {
        b.iter(|| {
            let runtime = WasmRuntime::new(WasmRuntimeConfig::default()).unwrap();
            black_box(runtime);
        });
    });
    
    group.bench_function("custom_runtime", |b| {
        let config = WasmRuntimeConfig {
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            max_execution_time: std::time::Duration::from_secs(5),
            enable_async: true,
            enable_wasi: true,
            enable_fuel: true,
            initial_fuel: 10_000_000,
        };
        
        b.iter(|| {
            let runtime = WasmRuntime::new(black_box(config.clone())).unwrap();
            black_box(runtime);
        });
    });
    
    group.finish();
}

/// Benchmark CID generation for different data patterns
fn bench_cid_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("cid_patterns");
    
    // Compressible data (repeated pattern)
    let compressible: Vec<u8> = vec![42u8; 10240]; // 10KB
    
    // Random data (not compressible)
    let random: Vec<u8> = (0..10240).map(|_| rand::random()).collect();
    
    // Structured WASM-like data
    let wasm = create_wasm_with_functions(100);
    
    group.bench_function("compressible_data", |b| {
        b.iter(|| {
            let cid = ModuleCid::from_bytes(black_box(&compressible));
            black_box(cid);
        });
    });
    
    group.bench_function("random_data", |b| {
        b.iter(|| {
            let cid = ModuleCid::from_bytes(black_box(&random));
            black_box(cid);
        });
    });
    
    group.bench_function("wasm_data", |b| {
        b.iter(|| {
            let cid = ModuleCid::from_bytes(black_box(&wasm));
            black_box(cid);
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_cid_generation,
    bench_runtime_creation,
    bench_cid_patterns,
);

criterion_main!(benches);
