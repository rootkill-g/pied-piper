# Pied Piper Performance Benchmarks

This directory contains comprehensive performance benchmarks for critical operations in Pied Piper.

## Overview

Benchmarks are implemented using [Criterion.rs](https://github.com/bheisler/criterion.rs), which provides:
- Statistical analysis of performance
- Detection of performance regressions
- HTML reports with charts
- Comparison between benchmark runs

## Running Benchmarks

### Run All Benchmarks
```bash
cargo bench
```

### Run Specific Benchmark Suite
```bash
cargo bench --bench crypto_bench
cargo bench --bench package_bench
cargo bench --bench wasm_bench
```

### Run Specific Benchmark
```bash
cargo bench --bench crypto_bench -- encryption
cargo bench --bench package_bench -- serialization
```

## Benchmark Suites

### 1. Crypto Benchmarks (`crypto_bench.rs`)

Tests cryptographic operation performance:

- **Encryption**: AES-256-GCM encryption with various data sizes (1KB to 10MB)
- **Decryption**: Decryption performance benchmarks
- **Encryption Roundtrip**: Combined encrypt + decrypt operations
- **Network Key Derivation**: Key generation from peer IDs
- **Hashing**: Blake3 hash performance
- **Parallel Encryption**: Multi-file encryption scenarios

**Key Metrics:**
- Throughput (bytes/sec)
- Latency (ms)
- Scalability with data size

### 2. Package Benchmarks (`package_bench.rs`)

Tests .pn package operations:

- **Package Serialization**: Converting packages to encrypted bytes
- **Package Deserialization**: Loading packages from bytes
- **Package Roundtrip**: Full serialize + deserialize cycle
- **Assets Handling**: Performance with multiple assets (10-200 files)
- **Manifest Operations**: TOML parsing and generation
- **Compression**: Zstd compression ratio and speed

**Key Metrics:**
- Serialization time
- Compression ratio
- Memory efficiency
- Asset count impact

### 3. WASM Benchmarks (`wasm_bench.rs`)

Tests WebAssembly runtime performance:

- **CID Generation**: Module content hashing (Blake3)
- **Loader Creation**: Module loader initialization
- **Module Caching**: Cache insertion and retrieval
- **Runtime Creation**: WASM runtime initialization
- **Load from Bytes**: Module loading and validation
- **Concurrent Loading**: Parallel module loading (5-50 modules)
- **Cache Tiers**: Memory vs disk cache performance

**Key Metrics:**
- Module load time
- Cache hit latency
- Concurrent throughput
- Memory overhead

## Viewing Results

After running benchmarks, reports are generated in:
```
target/criterion/
```

Open the HTML report:
```bash
open target/criterion/report/index.html
```

## Performance Targets

### Crypto Operations
- Encryption: >100 MB/s for 1MB+ files
- Decryption: >100 MB/s for 1MB+ files
- Network key derivation: <1ms per key

### Package Operations
- Serialization: >50 MB/s for 1MB packages
- Deserialization: >50 MB/s for 1MB packages
- Compression ratio: >60% for typical web assets

### WASM Operations
- CID generation: <10ms for 1MB module
- Module caching: <5ms for memory cache hit
- Concurrent loading: Linear scaling up to CPU cores

## Continuous Benchmarking

To track performance over time:

1. **Baseline**: Run benchmarks on main branch
   ```bash
   git checkout main
   cargo bench
   ```

2. **Compare**: Run benchmarks on feature branch
   ```bash
   git checkout feature-branch
   cargo bench
   ```

3. **Review**: Criterion automatically compares with baseline
   - Green: Performance improved
   - Red: Performance regressed
   - Gray: No significant change

## Integration with CI/CD

Add to CI pipeline:
```yaml
- name: Run Benchmarks
  run: cargo bench --no-fail-fast
  
- name: Upload Benchmark Results
  uses: actions/upload-artifact@v2
  with:
    name: benchmark-results
    path: target/criterion
```

## Profiling

For detailed profiling, use:

### CPU Profiling
```bash
cargo bench --bench crypto_bench --profile-time 10
```

### Flamegraphs
```bash
cargo install flamegraph
cargo flamegraph --bench crypto_bench
```

### Memory Profiling
```bash
cargo bench --features criterion/mem
```

## Adding New Benchmarks

1. Create a new benchmark file in `benches/`:
   ```rust
   use criterion::{criterion_group, criterion_main, Criterion};
   
   fn my_benchmark(c: &mut Criterion) {
       c.bench_function("my_op", |b| {
           b.iter(|| {
               // Operation to benchmark
           });
       });
   }
   
   criterion_group!(benches, my_benchmark);
   criterion_main!(benches);
   ```

2. Register in `Cargo.toml`:
   ```toml
   [[bench]]
   name = "my_bench"
   harness = false
   ```

3. Run with `cargo bench --bench my_bench`

## Best Practices

1. **Use `black_box`**: Prevent compiler optimizations
   ```rust
   b.iter(|| {
       let result = expensive_operation(black_box(&input));
       black_box(result);
   });
   ```

2. **Set Throughput**: For size-dependent operations
   ```rust
   group.throughput(Throughput::Bytes(data.len() as u64));
   ```

3. **Warm-up**: Criterion automatically handles warm-up periods

4. **Measurement Time**: Adjust for slow operations
   ```rust
   group.measurement_time(Duration::from_secs(30));
   ```

## Troubleshooting

### Benchmarks Too Slow
- Reduce sample size: `group.sample_size(10);`
- Reduce measurement time: `group.measurement_time(Duration::from_secs(5));`

### High Variance
- Disable CPU frequency scaling
- Close background applications
- Run on dedicated benchmark machine

### Out of Memory
- Reduce data sizes
- Use sampling for large datasets

## Resources

- [Criterion.rs Book](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [The Rust Programming Language - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
