use dashmap::DashMap;
use opentelemetry::metrics::{Counter, Histogram, UpDownCounter};
use std::sync::Arc;
use y_sweet_core::doc_sync::DocWithSyncKv;

/// Server-wide metrics backed by OpenTelemetry instruments.
///
/// All fields are OTel instruments (internally Arc-based), so `Metrics` is
/// cheap to clone and safe to share across tasks.
#[derive(Clone)]
pub struct Metrics {
    pub active_connections: UpDownCounter<i64>,
    pub websocket_failures: Counter<u64>,
    pub pong_timeouts: Counter<u64>,
    pub documents_loaded: Counter<u64>,
    pub documents_gc: Counter<u64>,
    pub sync_updates: Counter<u64>,
    pub http_updates: Counter<u64>,
    pub persistence_ops: Counter<u64>,
    pub persistence_errors: Counter<u64>,
    pub sync_latency_ms: Histogram<f64>,
    pub http_latency_ms: Histogram<f64>,
    pub persistence_latency_ms: Histogram<f64>,
}

impl Metrics {
    /// Create all OTel instruments using the global meter provider.
    ///
    /// The `docs` reference is captured by the ObservableGauge callback to
    /// report `y_sweet_active_documents` on each collection cycle.
    pub fn new(docs: Arc<DashMap<String, DocWithSyncKv>>) -> Self {
        let meter = opentelemetry::global::meter("y-sweet");

        // ObservableGauge: the SDK retains the callback registration internally,
        // so we don't need to store the returned handle.
        let _active_documents = meter
            .u64_observable_gauge("y_sweet_active_documents")
            .with_description("Number of documents loaded in memory")
            .with_callback(move |observer| {
                observer.observe(docs.len() as u64, &[]);
            })
            .build();

        Self {
            active_connections: meter
                .i64_up_down_counter("y_sweet_active_connections")
                .with_description("Current number of active WebSocket connections")
                .build(),
            websocket_failures: meter
                .u64_counter("y_sweet_websocket_failures")
                .with_description("WebSocket stream errors")
                .build(),
            pong_timeouts: meter
                .u64_counter("y_sweet_pong_timeouts")
                .with_description("Pong timeout disconnections")
                .build(),
            documents_loaded: meter
                .u64_counter("y_sweet_documents_loaded")
                .with_description("Documents loaded from store")
                .build(),
            documents_gc: meter
                .u64_counter("y_sweet_documents_gc")
                .with_description("Documents garbage collected")
                .build(),
            sync_updates: meter
                .u64_counter("y_sweet_sync_updates")
                .with_description("Sync updates via WebSocket")
                .build(),
            http_updates: meter
                .u64_counter("y_sweet_http_updates")
                .with_description("Updates via HTTP")
                .build(),
            persistence_ops: meter
                .u64_counter("y_sweet_persistence_ops")
                .with_description("Successful persistence operations")
                .build(),
            persistence_errors: meter
                .u64_counter("y_sweet_persistence_errors")
                .with_description("Failed persistence operations")
                .build(),
            sync_latency_ms: meter
                .f64_histogram("y_sweet_sync_latency")
                .with_description("Sync update processing latency in milliseconds")
                .with_unit("ms")
                .with_boundaries(vec![0.0, 0.001, 0.01, 0.1, 0.3, 0.5, 1.0, 5.0, 10.0, 50.0, 250.0, 1000.0])
                .build(),
            http_latency_ms: meter
                .f64_histogram("y_sweet_http_latency")
                .with_description("HTTP update processing latency in milliseconds")
                .with_unit("ms")
                .with_boundaries(vec![0.0, 0.001, 0.01, 0.1, 0.3, 0.5, 1.0, 5.0, 10.0, 50.0, 250.0, 1000.0])
                .build(),
            persistence_latency_ms: meter
                .f64_histogram("y_sweet_persistence_latency")
                .with_description("Persistence latency in milliseconds")
                .with_unit("ms")
                .with_boundaries(vec![0.0, 0.001, 0.01, 0.1, 0.3, 0.5, 1.0, 5.0, 10.0, 50.0, 250.0, 1000.0])
                .build(),
        }
    }
}
