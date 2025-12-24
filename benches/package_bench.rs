use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use pied_piper::package::{PiperNetPackage, PackageManifest, PackageMetadata, PackageType};
use std::collections::HashMap;

/// Helper to create a test manifest
fn create_test_manifest(name: &str) -> PackageManifest {
    PackageManifest {
        package_type: PackageType::Backend,
        entrypoint: "module.wasm".to_string(),
        assets: vec![],
        dependencies: HashMap::new(),
        metadata: PackageMetadata {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: Some("Benchmark test package".to_string()),
            author: Some("Benchmark Suite".to_string()),
            license: Some("MIT".to_string()),
            homepage: None,
            repository: None,
        },
    }
}

/// Benchmark package serialization with different sizes
fn bench_package_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_serialization");
    
    let key = [42u8; 32];
    let sizes = vec![
        ("small_10KB", 10 * 1024),
        ("medium_100KB", 100 * 1024),
        ("large_1MB", 1024 * 1024),
        ("xlarge_10MB", 10 * 1024 * 1024),
    ];
    
    for (label, size) in sizes {
        let manifest = create_test_manifest(label);
        let module: Vec<u8> = (0..size).map(|_| rand::random()).collect();
        let package = PiperNetPackage::new(manifest, module, HashMap::new(), HashMap::new());
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), &package, |b, pkg| {
            b.iter(|| {
                let bytes = pkg.to_bytes(black_box(&key)).unwrap();
                black_box(bytes);
            });
        });
    }
    
    group.finish();
}

/// Benchmark package deserialization
fn bench_package_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_deserialization");
    
    let key = [42u8; 32];
    let sizes = vec![
        ("small_10KB", 10 * 1024),
        ("medium_100KB", 100 * 1024),
        ("large_1MB", 1024 * 1024),
        ("xlarge_10MB", 10 * 1024 * 1024),
    ];
    
    for (label, size) in sizes {
        let manifest = create_test_manifest(label);
        let module: Vec<u8> = (0..size).map(|_| rand::random()).collect();
        let package = PiperNetPackage::new(manifest, module, HashMap::new(), HashMap::new());
        let bytes = package.to_bytes(&key).unwrap();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), &bytes, |b, bytes| {
            b.iter(|| {
                let pkg = PiperNetPackage::from_bytes(black_box(bytes), black_box(&key)).unwrap();
                black_box(pkg);
            });
        });
    }
    
    group.finish();
}

/// Benchmark package roundtrip (serialize + deserialize)
fn bench_package_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_roundtrip");
    
    let key = [42u8; 32];
    let sizes = vec![
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
    ];
    
    for (label, size) in sizes {
        let manifest = create_test_manifest(label);
        let module: Vec<u8> = (0..size).map(|_| rand::random()).collect();
        let package = PiperNetPackage::new(manifest, module, HashMap::new(), HashMap::new());
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), &package, |b, pkg| {
            b.iter(|| {
                let bytes = pkg.to_bytes(black_box(&key)).unwrap();
                let decoded = PiperNetPackage::from_bytes(black_box(&bytes), black_box(&key)).unwrap();
                black_box(decoded);
            });
        });
    }
    
    group.finish();
}

/// Benchmark package with multiple assets
fn bench_package_with_assets(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_with_assets");
    
    let key = [42u8; 32];
    let asset_counts = vec![10, 50, 100, 200];
    let asset_size = 10 * 1024; // 10KB per asset
    
    for count in asset_counts {
        let manifest = create_test_manifest(&format!("{}_assets", count));
        let module: Vec<u8> = (0..asset_size).map(|_| rand::random()).collect();
        
        // Create multiple assets
        let mut assets = HashMap::new();
        for i in 0..count {
            let asset_data: Vec<u8> = (0..asset_size).map(|_| rand::random()).collect();
            assets.insert(format!("asset_{}.txt", i), asset_data);
        }
        
        let package = PiperNetPackage::new(manifest, module, assets, HashMap::new());
        let total_size = (count as u64 + 1) * asset_size as u64;
        
        group.throughput(Throughput::Bytes(total_size));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_assets", count)), 
            &package, 
            |b, pkg| {
                b.iter(|| {
                    let bytes = pkg.to_bytes(black_box(&key)).unwrap();
                    black_box(bytes);
                });
            }
        );
    }
    
    group.finish();
}

/// Benchmark manifest serialization/deserialization
fn bench_manifest_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_operations");
    
    let manifest = create_test_manifest("test");
    
    group.bench_function("to_toml", |b| {
        b.iter(|| {
            let toml = manifest.to_toml().unwrap();
            black_box(toml);
        });
    });
    
    let toml = manifest.to_toml().unwrap();
    group.bench_function("from_toml", |b| {
        b.iter(|| {
            let parsed = PackageManifest::from_toml(black_box(&toml)).unwrap();
            black_box(parsed);
        });
    });
    
    group.finish();
}

/// Benchmark compression impact
fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");
    
    let key = [42u8; 32];
    
    // Highly compressible data (repeated pattern)
    let compressible: Vec<u8> = vec![42u8; 1024 * 1024]; // 1MB of same byte
    
    // Random data (not compressible)
    let random: Vec<u8> = (0..1024*1024).map(|_| rand::random()).collect();
    
    let manifest = create_test_manifest("compression_test");
    
    group.bench_function("compressible_data", |b| {
        let pkg = PiperNetPackage::new(
            manifest.clone(), 
            compressible.clone(), 
            HashMap::new(), 
            HashMap::new()
        );
        b.iter(|| {
            let bytes = pkg.to_bytes(black_box(&key)).unwrap();
            black_box(bytes);
        });
    });
    
    group.bench_function("random_data", |b| {
        let pkg = PiperNetPackage::new(
            manifest.clone(), 
            random.clone(), 
            HashMap::new(), 
            HashMap::new()
        );
        b.iter(|| {
            let bytes = pkg.to_bytes(black_box(&key)).unwrap();
            black_box(bytes);
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_package_serialization,
    bench_package_deserialization,
    bench_package_roundtrip,
    bench_package_with_assets,
    bench_manifest_operations,
    bench_compression_ratio,
);

criterion_main!(benches);
