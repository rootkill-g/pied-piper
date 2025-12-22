//! Metrics collection and monitoring for Pied Piper
//!
//! This module provides comprehensive metrics collection using Prometheus,
//! tracking network health, WASM execution, gateway performance, and more.

use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, IntCounter,
    IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};
use std::sync::Arc;
use std::time::Instant;

/// Global metrics registry for the Pied Piper node
#[derive(Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    
    // Network metrics
    pub network_peers_connected: IntGauge,
    pub network_peers_discovered: IntCounter,
    pub network_messages_sent: IntCounterVec,
    pub network_messages_received: IntCounterVec,
    pub network_bytes_sent: Counter,
    pub network_bytes_received: Counter,
    
    // DHT metrics
    pub dht_records_stored: IntGauge,
    pub dht_queries_total: IntCounterVec,
    pub dht_query_duration: HistogramVec,
    
    // Content metrics
    pub content_modules_cached: IntGauge,
    pub content_cache_hits: IntCounter,
    pub content_cache_misses: IntCounter,
    pub content_fetches_total: IntCounterVec,
    pub content_fetch_duration: Histogram,
    
    // Gateway metrics
    pub http_requests_total: IntCounterVec,
    pub http_request_duration: HistogramVec,
    pub http_response_size: Histogram,
    pub websocket_connections: IntGauge,
    pub websocket_messages_sent: IntCounter,
    pub websocket_messages_received: IntCounter,
    
    // WASM execution metrics
    pub wasm_executions_total: IntCounterVec,
    pub wasm_execution_duration: HistogramVec,
    pub wasm_memory_usage: GaugeVec,
    pub wasm_fuel_consumed: Histogram,
    pub wasm_host_function_calls: IntCounterVec,
    
    // CRDT metrics
    pub crdt_operations_total: IntCounterVec,
    pub crdt_merge_operations: IntCounter,
    pub crdt_sync_messages_sent: IntCounter,
    pub crdt_sync_messages_received: IntCounter,
    pub crdt_maps_count: IntGauge,
    pub crdt_sets_count: IntGauge,
    
    // System metrics
    pub system_uptime: Gauge,
    pub system_cpu_usage: Gauge,
    pub system_memory_usage: Gauge,
    pub system_disk_usage: Gauge,
}

impl Metrics {
    /// Create a new metrics registry with all metrics initialized
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();
        
        // Network metrics
        let network_peers_connected = IntGauge::new(
            "pied_piper_network_peers_connected",
            "Number of currently connected peers"
        )?;
        registry.register(Box::new(network_peers_connected.clone()))?;
        
        let network_peers_discovered = IntCounter::new(
            "pied_piper_network_peers_discovered_total",
            "Total number of peers discovered"
        )?;
        registry.register(Box::new(network_peers_discovered.clone()))?;
        
        let network_messages_sent = IntCounterVec::new(
            Opts::new(
                "pied_piper_network_messages_sent_total",
                "Total number of network messages sent by protocol"
            ),
            &["protocol"]
        )?;
        registry.register(Box::new(network_messages_sent.clone()))?;
        
        let network_messages_received = IntCounterVec::new(
            Opts::new(
                "pied_piper_network_messages_received_total",
                "Total number of network messages received by protocol"
            ),
            &["protocol"]
        )?;
        registry.register(Box::new(network_messages_received.clone()))?;
        
        let network_bytes_sent = Counter::new(
            "pied_piper_network_bytes_sent_total",
            "Total bytes sent over the network"
        )?;
        registry.register(Box::new(network_bytes_sent.clone()))?;
        
        let network_bytes_received = Counter::new(
            "pied_piper_network_bytes_received_total",
            "Total bytes received from the network"
        )?;
        registry.register(Box::new(network_bytes_received.clone()))?;
        
        // DHT metrics
        let dht_records_stored = IntGauge::new(
            "pied_piper_dht_records_stored",
            "Number of records currently stored in DHT"
        )?;
        registry.register(Box::new(dht_records_stored.clone()))?;
        
        let dht_queries_total = IntCounterVec::new(
            Opts::new(
                "pied_piper_dht_queries_total",
                "Total DHT queries by type"
            ),
            &["query_type", "status"]
        )?;
        registry.register(Box::new(dht_queries_total.clone()))?;
        
        let dht_query_duration = HistogramVec::new(
            HistogramOpts::new(
                "pied_piper_dht_query_duration_seconds",
                "DHT query duration in seconds"
            ).buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
            &["query_type"]
        )?;
        registry.register(Box::new(dht_query_duration.clone()))?;
        
        // Content metrics
        let content_modules_cached = IntGauge::new(
            "pied_piper_content_modules_cached",
            "Number of WASM modules in cache"
        )?;
        registry.register(Box::new(content_modules_cached.clone()))?;
        
        let content_cache_hits = IntCounter::new(
            "pied_piper_content_cache_hits_total",
            "Total cache hits"
        )?;
        registry.register(Box::new(content_cache_hits.clone()))?;
        
        let content_cache_misses = IntCounter::new(
            "pied_piper_content_cache_misses_total",
            "Total cache misses"
        )?;
        registry.register(Box::new(content_cache_misses.clone()))?;
        
        let content_fetches_total = IntCounterVec::new(
            Opts::new(
                "pied_piper_content_fetches_total",
                "Total content fetches by status"
            ),
            &["status"]
        )?;
        registry.register(Box::new(content_fetches_total.clone()))?;
        
        let content_fetch_duration = Histogram::with_opts(
            HistogramOpts::new(
                "pied_piper_content_fetch_duration_seconds",
                "Content fetch duration in seconds"
            ).buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0])
        )?;
        registry.register(Box::new(content_fetch_duration.clone()))?;
        
        // Gateway metrics
        let http_requests_total = IntCounterVec::new(
            Opts::new(
                "pied_piper_http_requests_total",
                "Total HTTP requests by method and status"
            ),
            &["method", "path", "status"]
        )?;
        registry.register(Box::new(http_requests_total.clone()))?;
        
        let http_request_duration = HistogramVec::new(
            HistogramOpts::new(
                "pied_piper_http_request_duration_seconds",
                "HTTP request duration in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
            &["method", "path"]
        )?;
        registry.register(Box::new(http_request_duration.clone()))?;
        
        let http_response_size = Histogram::with_opts(
            HistogramOpts::new(
                "pied_piper_http_response_size_bytes",
                "HTTP response size in bytes"
            ).buckets(vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0])
        )?;
        registry.register(Box::new(http_response_size.clone()))?;
        
        let websocket_connections = IntGauge::new(
            "pied_piper_websocket_connections",
            "Number of active WebSocket connections"
        )?;
        registry.register(Box::new(websocket_connections.clone()))?;
        
        let websocket_messages_sent = IntCounter::new(
            "pied_piper_websocket_messages_sent_total",
            "Total WebSocket messages sent"
        )?;
        registry.register(Box::new(websocket_messages_sent.clone()))?;
        
        let websocket_messages_received = IntCounter::new(
            "pied_piper_websocket_messages_received_total",
            "Total WebSocket messages received"
        )?;
        registry.register(Box::new(websocket_messages_received.clone()))?;
        
        // WASM execution metrics
        let wasm_executions_total = IntCounterVec::new(
            Opts::new(
                "pied_piper_wasm_executions_total",
                "Total WASM executions by module and status"
            ),
            &["module_cid", "status"]
        )?;
        registry.register(Box::new(wasm_executions_total.clone()))?;
        
        let wasm_execution_duration = HistogramVec::new(
            HistogramOpts::new(
                "pied_piper_wasm_execution_duration_seconds",
                "WASM execution duration in seconds"
            ).buckets(vec![0.001, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
            &["module_cid"]
        )?;
        registry.register(Box::new(wasm_execution_duration.clone()))?;
        
        let wasm_memory_usage = GaugeVec::new(
            Opts::new(
                "pied_piper_wasm_memory_usage_bytes",
                "WASM module memory usage in bytes"
            ),
            &["module_cid"]
        )?;
        registry.register(Box::new(wasm_memory_usage.clone()))?;
        
        let wasm_fuel_consumed = Histogram::with_opts(
            HistogramOpts::new(
                "pied_piper_wasm_fuel_consumed",
                "WASM fuel consumed per execution"
            ).buckets(vec![1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0])
        )?;
        registry.register(Box::new(wasm_fuel_consumed.clone()))?;
        
        let wasm_host_function_calls = IntCounterVec::new(
            Opts::new(
                "pied_piper_wasm_host_function_calls_total",
                "Total host function calls by function name"
            ),
            &["function"]
        )?;
        registry.register(Box::new(wasm_host_function_calls.clone()))?;
        
        // CRDT metrics
        let crdt_operations_total = IntCounterVec::new(
            Opts::new(
                "pied_piper_crdt_operations_total",
                "Total CRDT operations by type"
            ),
            &["crdt_type", "operation"]
        )?;
        registry.register(Box::new(crdt_operations_total.clone()))?;
        
        let crdt_merge_operations = IntCounter::new(
            "pied_piper_crdt_merge_operations_total",
            "Total CRDT merge operations"
        )?;
        registry.register(Box::new(crdt_merge_operations.clone()))?;
        
        let crdt_sync_messages_sent = IntCounter::new(
            "pied_piper_crdt_sync_messages_sent_total",
            "Total CRDT sync messages sent"
        )?;
        registry.register(Box::new(crdt_sync_messages_sent.clone()))?;
        
        let crdt_sync_messages_received = IntCounter::new(
            "pied_piper_crdt_sync_messages_received_total",
            "Total CRDT sync messages received"
        )?;
        registry.register(Box::new(crdt_sync_messages_received.clone()))?;
        
        let crdt_maps_count = IntGauge::new(
            "pied_piper_crdt_maps_count",
            "Number of LWW-Maps"
        )?;
        registry.register(Box::new(crdt_maps_count.clone()))?;
        
        let crdt_sets_count = IntGauge::new(
            "pied_piper_crdt_sets_count",
            "Number of OR-Sets"
        )?;
        registry.register(Box::new(crdt_sets_count.clone()))?;
        
        // System metrics
        let system_uptime = Gauge::new(
            "pied_piper_system_uptime_seconds",
            "System uptime in seconds"
        )?;
        registry.register(Box::new(system_uptime.clone()))?;
        
        let system_cpu_usage = Gauge::new(
            "pied_piper_system_cpu_usage_percent",
            "System CPU usage percentage"
        )?;
        registry.register(Box::new(system_cpu_usage.clone()))?;
        
        let system_memory_usage = Gauge::new(
            "pied_piper_system_memory_usage_bytes",
            "System memory usage in bytes"
        )?;
        registry.register(Box::new(system_memory_usage.clone()))?;
        
        let system_disk_usage = Gauge::new(
            "pied_piper_system_disk_usage_bytes",
            "System disk usage in bytes"
        )?;
        registry.register(Box::new(system_disk_usage.clone()))?;
        
        Ok(Self {
            registry: Arc::new(registry),
            network_peers_connected,
            network_peers_discovered,
            network_messages_sent,
            network_messages_received,
            network_bytes_sent,
            network_bytes_received,
            dht_records_stored,
            dht_queries_total,
            dht_query_duration,
            content_modules_cached,
            content_cache_hits,
            content_cache_misses,
            content_fetches_total,
            content_fetch_duration,
            http_requests_total,
            http_request_duration,
            http_response_size,
            websocket_connections,
            websocket_messages_sent,
            websocket_messages_received,
            wasm_executions_total,
            wasm_execution_duration,
            wasm_memory_usage,
            wasm_fuel_consumed,
            wasm_host_function_calls,
            crdt_operations_total,
            crdt_merge_operations,
            crdt_sync_messages_sent,
            crdt_sync_messages_received,
            crdt_maps_count,
            crdt_sets_count,
            system_uptime,
            system_cpu_usage,
            system_memory_usage,
            system_disk_usage,
        })
    }
    
    /// Get the Prometheus registry
    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
    
    /// Export metrics in Prometheus text format
    pub fn export(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

/// Helper struct for timing operations
pub struct TimingGuard {
    start: Instant,
    histogram: Histogram,
}

impl TimingGuard {
    pub fn new(histogram: Histogram) -> Self {
        Self {
            start: Instant::now(),
            histogram,
        }
    }
}

impl Drop for TimingGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        self.histogram.observe(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new().unwrap();
        assert_eq!(metrics.network_peers_connected.get(), 0);
        assert_eq!(metrics.content_cache_hits.get(), 0);
    }
    
    #[test]
    fn test_metrics_increment() {
        let metrics = Metrics::new().unwrap();
        metrics.network_peers_connected.inc();
        assert_eq!(metrics.network_peers_connected.get(), 1);
    }
    
    #[test]
    fn test_metrics_export() {
        let metrics = Metrics::new().unwrap();
        metrics.network_peers_connected.set(5);
        let export = metrics.export();
        assert!(export.contains("pied_piper_network_peers_connected 5"));
    }
}
