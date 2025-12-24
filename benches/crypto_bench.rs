use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use pied_piper::package::crypto::{encrypt, decrypt, get_network_key};
use pied_piper::wasm::loader::ModuleCid;

/// Benchmark encryption performance with different data sizes
fn bench_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption");
    
    // Test different data sizes
    let sizes = vec![
        ("1KB", 1024),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
    ];
    
    let key = get_network_key();
    
    for (label, size) in sizes {
        let data: Vec<u8> = (0..size).map(|_| rand::random()).collect();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("encrypt", label), &data, |b, data| {
            b.iter(|| {
                let encrypted = encrypt(black_box(data), black_box(&key)).unwrap();
                black_box(encrypted);
            });
        });
    }
    
    group.finish();
}

/// Benchmark decryption performance
fn bench_decryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("decryption");
    
    let sizes = vec![
        ("1KB", 1024),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
    ];
    
    let key = get_network_key();
    
    for (label, size) in sizes {
        let data: Vec<u8> = (0..size).map(|_| rand::random()).collect();
        let encrypted = encrypt(&data, &key).unwrap();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("decrypt", label), &encrypted, |b, encrypted| {
            b.iter(|| {
                let decrypted = decrypt(black_box(encrypted), black_box(&key)).unwrap();
                black_box(decrypted);
            });
        });
    }
    
    group.finish();
}

/// Benchmark encryption roundtrip (encrypt + decrypt)
fn bench_encryption_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption_roundtrip");
    
    let sizes = vec![
        ("1KB", 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
    ];
    
    let key = get_network_key();
    
    for (label, size) in sizes {
        let data: Vec<u8> = (0..size).map(|_| rand::random()).collect();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), &data, |b, data| {
            b.iter(|| {
                let encrypted = encrypt(black_box(data), black_box(&key)).unwrap();
                let decrypted = decrypt(black_box(&encrypted), black_box(&key)).unwrap();
                black_box(decrypted);
            });
        });
    }
    
    group.finish();
}

/// Benchmark Blake3 hashing via CID generation
fn bench_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashing");
    
    let sizes = vec![
        ("1KB", 1024),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
    ];
    
    for (label, size) in sizes {
        let data: Vec<u8> = (0..size).map(|_| rand::random()).collect();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("blake3", label), &data, |b, data| {
            b.iter(|| {
                let cid = ModuleCid::from_bytes(black_box(data));
                black_box(cid);
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_encryption,
    bench_decryption,
    bench_encryption_roundtrip,
    bench_hashing,
);

criterion_main!(benches);
