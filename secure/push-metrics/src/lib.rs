// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

// Re-export counter types from prometheus crate
pub use prometheus::{
    register_histogram, register_histogram_vec, register_int_counter, register_int_counter_vec,
    register_int_gauge, register_int_gauge_vec, Histogram, HistogramVec, IntCounter, IntCounterVec,
    IntGauge, IntGaugeVec,
};

use libra_logger::{error, info};
use prometheus::{Encoder, TextEncoder};
use std::{env, thread, thread::JoinHandle, time::Duration};

const DEFAULT_PUSH_FREQUENCY_SECS: u64 = 15;

/// MetricsPusher provides a function to push a list of Metrics to a configurable
/// pushgateway endpoint.
pub struct MetricsPusher;

impl MetricsPusher {
    fn run(self, push_metrics_endpoint: String, push_metrics_frequency_secs: u64) {
        loop {
            let mut buffer = Vec::new();
            if let Err(e) = TextEncoder::new().encode(&prometheus::gather(), &mut buffer) {
                error!("Failed to encode push metrics: {}.", e.to_string());
            } else {
                let response = ureq::post(&push_metrics_endpoint)
                    .timeout_connect(10_000)
                    .send_bytes(&buffer);
                if let Some(error) = response.synthetic_error() {
                    error!(
                        "Failed to push metrics to {}. Error: {}",
                        push_metrics_endpoint, error
                    );
                }
            }
            thread::sleep(Duration::from_secs(push_metrics_frequency_secs));
        }
    }

    /// start starts a new thread and periodically pushes the metrics to a pushgateway endpoint
    pub fn start(self) -> Option<JoinHandle<()>> {
        // eg value for PUSH_METRICS_ENDPOINT: "http://pushgatewar.server.com:9091/metrics/job/safety_rules"
        let push_metrics_endpoint = match env::var("PUSH_METRICS_ENDPOINT") {
            Ok(s) => s,
            Err(_) => {
                info!("PUSH_METRICS_ENDPOINT env var is not set. Skipping sending metrics.");
                return None;
            }
        };
        let push_metrics_frequency_secs = match env::var("PUSH_METRICS_FREQUENCY_SECS") {
            Ok(s) => match s.parse::<u64>() {
                Ok(i) => i,
                Err(_) => {
                    error!("Invalid value for PUSH_METRICS_FREQUENCY_SECS: {}", s);
                    return None;
                }
            },
            Err(_) => DEFAULT_PUSH_FREQUENCY_SECS,
        };
        info!(
            "Starting push metrics loop. Sending metrics to {} with a frequency of {} seconds",
            push_metrics_endpoint, push_metrics_frequency_secs
        );
        Some(thread::spawn(move || {
            self.run(push_metrics_endpoint, push_metrics_frequency_secs)
        }))
    }
}
