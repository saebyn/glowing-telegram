use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetrievalOptions {
    pub bulk: f64,
    pub standard: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageCostConfig {
    pub storage_costs_per_gb_month: HashMap<String, f64>,
    pub retrieval_costs_per_gb: HashMap<String, RetrievalOptions>,
    pub retrieval_times_hours: HashMap<String, RetrievalOptions>,
    pub compute_cost_per_hour: f64,
    pub compute_hours_per_video_gb: f64,
}

impl StorageCostConfig {
    /// Get retrieval cost for a storage class and tier (bulk or standard)
    pub fn get_retrieval_cost(&self, storage_class: &str, tier: &str) -> Option<f64> {
        self.retrieval_costs_per_gb
            .get(storage_class)
            .and_then(|opts| match tier {
                "bulk" => Some(opts.bulk),
                "standard" => Some(opts.standard),
                _ => None,
            })
    }

    /// Get retrieval time for a storage class and tier (bulk or standard)
    pub fn get_retrieval_time(&self, storage_class: &str, tier: &str) -> Option<f64> {
        self.retrieval_times_hours
            .get(storage_class)
            .and_then(|opts| match tier {
                "bulk" => Some(opts.bulk),
                "standard" => Some(opts.standard),
                _ => None,
            })
    }

    /// Calculate compute cost based on video size in bytes
    pub fn calculate_compute_cost(&self, size_bytes: i64) -> f64 {
        let size_gb = size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let compute_hours = size_gb * self.compute_hours_per_video_gb;
        compute_hours * self.compute_cost_per_hour
    }
}
