// use metrics::{counter, gauge, histogram};
// use metrics_exporter_prometheus::PrometheusBuilder;
// use lazy_static::lazy_static;
// use std::time::Instant;
//
// lazy_static! {
//     static ref START_TIME: Instant = Instant::now();
// }
//
// pub fn setup_monitoring() -> Result<(), Box<dyn std::error::Error>> {
//     let builder = PrometheusBuilder::new();
//     builder.install()?;
//     Ok(())
// }
//
// pub fn record_signing_request() {
//     counter!("tss_signing_requests_total").increment(1);
// }
//
// pub fn record_signing_result(success: bool) {
//     if success {
//         counter!("tss_signing_results_success").increment(1);
//     } else {
//         counter!("tss_signing_results_failure").increment(1);
//     }
// }
//
// pub fn record_signing_duration(duration_ms: u64) {
//     histogram!("tss_signing_duration_ms").record(duration_ms as f64);
// }
//
// pub fn update_active_signers(count: usize) {
//     gauge!("tss_active_signers").set(count as f64);
// }
//
// pub fn record_uptime() {
//     gauge!("tss_uptime_seconds").set(START_TIME.elapsed().as_secs() as f64);
// }
//
// pub fn record_queue_depth(depth: usize) {
//     gauge!("tss_queue_depth").set(depth as f64);
// }
//
// pub fn record_storage_operation(operation: &str, success: bool) {
//     let label = if success { "success" } else { "failure" };
//     counter!("tss_storage_operations_total", "operation" => operation.to_string(), "result" => label.to_string()).increment(1);
// }

use lazy_static::lazy_static;
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::time::Instant;

lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

pub fn setup_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let builder = PrometheusBuilder::new();
    builder.install()?;
    Ok(())
}

pub fn record_signing_request() {
    counter!("tss_signing_requests_total").increment(1);
}

pub fn record_signing_result(success: bool) {
    if success {
        counter!("tss_signing_results_success").increment(1);
    } else {
        counter!("tss_signing_results_failure").increment(1);
    }
}

pub fn record_signing_duration(duration_ms: u64) {
    histogram!("tss_signing_duration_ms").record(duration_ms as f64);
}

pub fn update_active_signers(count: usize) {
    gauge!("tss_active_signers").set(count as f64);
}

pub fn record_uptime() {
    gauge!("tss_uptime_seconds").set(START_TIME.elapsed().as_secs() as f64);
}

pub fn record_queue_depth(depth: usize) {
    gauge!("tss_queue_depth").set(depth as f64);
}
