use opentelemetry::KeyValue;
use opentelemetry_otlp::{MetricExporter, WithExportConfig};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::Resource;
use std::time::Duration;

/// Initialize the OpenTelemetry MeterProvider.
///
/// If `endpoint` is provided, configures an OTLP/HTTP exporter with a periodic
/// reader at the given `push_interval`. Otherwise, creates a no-op provider.
///
/// The returned `SdkMeterProvider` must be kept alive for the lifetime of the
/// application and `.shutdown()` called on graceful exit to flush final metrics.
pub fn init_meter_provider(
    endpoint: Option<&str>,
    service_name: &str,
    push_interval: Duration,
) -> SdkMeterProvider {
    let resource = Resource::builder()
        .with_attribute(KeyValue::new("service.name", service_name.to_string()))
        .build();

    let provider = if let Some(endpoint) = endpoint {
        let exporter = MetricExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .build()
            .expect("Failed to create OTLP metric exporter");

        let reader = PeriodicReader::builder(exporter)
            .with_interval(push_interval)
            .build();

        SdkMeterProvider::builder()
            .with_resource(resource)
            .with_reader(reader)
            .build()
    } else {
        SdkMeterProvider::builder()
            .with_resource(resource)
            .build()
    };

    opentelemetry::global::set_meter_provider(provider.clone());
    provider
}
