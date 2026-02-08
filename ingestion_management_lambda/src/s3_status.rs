use aws_sdk_s3::Client as S3Client;
use std::collections::HashMap;

use crate::ApiError;
use crate::config::StorageCostConfig;

#[derive(Debug)]
pub struct S3ObjectInfo {
    pub exists: bool,
    pub storage_class: Option<String>,
    pub size_bytes: Option<i64>,
    pub retrieval_required: bool,
}

/// Result type for cost calculations
pub struct CostEstimate {
    pub retrieval_costs: Option<HashMap<String, f64>>,
    pub retrieval_times: Option<HashMap<String, f64>>,
    pub compute_cost: f64,
}

/// Get information about an S3 object using HeadObject
pub async fn get_s3_object_info(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<S3ObjectInfo, ApiError> {
    match s3_client.head_object().bucket(bucket).key(key).send().await {
        Ok(output) => {
            let storage_class = output
                .storage_class()
                .map(|sc| normalize_storage_class(sc.as_str()));

            let size_bytes = output.content_length();

            let retrieval_required = storage_class
                .as_ref()
                .map(|sc| requires_retrieval(sc))
                .unwrap_or(false);

            Ok(S3ObjectInfo {
                exists: true,
                storage_class,
                size_bytes,
                retrieval_required,
            })
        }
        Err(err) => {
            // Check if it's a 404 (object not found)
            if err.to_string().contains("NotFound")
                || err.to_string().contains("404")
            {
                Ok(S3ObjectInfo {
                    exists: false,
                    storage_class: None,
                    size_bytes: None,
                    retrieval_required: false,
                })
            } else {
                Err(ApiError::S3Error(err.to_string()))
            }
        }
    }
}

/// Normalize storage class names to match our config keys
fn normalize_storage_class(storage_class: &str) -> String {
    match storage_class {
        "STANDARD" => "STANDARD",
        "REDUCED_REDUNDANCY" => "STANDARD",
        "STANDARD_IA" => "STANDARD_IA",
        "ONEZONE_IA" => "ONEZONE_IA",
        "INTELLIGENT_TIERING" => "INTELLIGENT_TIERING",
        "GLACIER" => "GLACIER",
        "GLACIER_IR" => "GLACIER_IR",
        "DEEP_ARCHIVE" => "DEEP_ARCHIVE",
        _ => storage_class,
    }
    .to_string()
}

/// Check if a storage class requires retrieval before access
fn requires_retrieval(storage_class: &str) -> bool {
    matches!(storage_class, "GLACIER" | "DEEP_ARCHIVE")
}

/// Calculate retrieval costs and times based on storage class and size
pub fn calculate_costs(
    s3_info: &S3ObjectInfo,
    config: &StorageCostConfig,
) -> CostEstimate {
    let size_bytes = s3_info.size_bytes.unwrap_or(0);
    let size_gb = size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

    // Calculate compute cost (always calculated if size is available)
    let compute_cost = config.calculate_compute_cost(size_bytes);

    // Only calculate retrieval costs for classes that require retrieval
    if !s3_info.retrieval_required {
        return CostEstimate {
            retrieval_costs: None,
            retrieval_times: None,
            compute_cost,
        };
    }

    let storage_class = match &s3_info.storage_class {
        Some(sc) => sc,
        None => {
            return CostEstimate {
                retrieval_costs: None,
                retrieval_times: None,
                compute_cost,
            }
        }
    };

    // Calculate retrieval costs for bulk and standard tiers
    let mut retrieval_costs = HashMap::new();
    let mut retrieval_times = HashMap::new();

    for tier in ["bulk", "standard"] {
        if let Some(cost_per_gb) =
            config.get_retrieval_cost(storage_class, tier)
        {
            let total_cost = cost_per_gb * size_gb;
            retrieval_costs.insert(
                tier.to_string(),
                (total_cost * 100.0).round() / 100.0,
            );
        }

        if let Some(time_hours) =
            config.get_retrieval_time(storage_class, tier)
        {
            retrieval_times.insert(tier.to_string(), time_hours);
        }
    }

    let retrieval_costs = if retrieval_costs.is_empty() {
        None
    } else {
        Some(retrieval_costs)
    };

    let retrieval_times = if retrieval_times.is_empty() {
        None
    } else {
        Some(retrieval_times)
    };

    CostEstimate {
        retrieval_costs,
        retrieval_times,
        compute_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_storage_class() {
        assert_eq!(normalize_storage_class("STANDARD"), "STANDARD");
        assert_eq!(normalize_storage_class("GLACIER"), "GLACIER");
        assert_eq!(normalize_storage_class("DEEP_ARCHIVE"), "DEEP_ARCHIVE");
        assert_eq!(normalize_storage_class("GLACIER_IR"), "GLACIER_IR");
    }

    #[test]
    fn test_requires_retrieval() {
        assert!(!requires_retrieval("STANDARD"));
        assert!(!requires_retrieval("STANDARD_IA"));
        assert!(!requires_retrieval("GLACIER_IR"));
        assert!(requires_retrieval("GLACIER"));
        assert!(requires_retrieval("DEEP_ARCHIVE"));
    }

    #[test]
    fn test_calculate_costs_no_retrieval() {
        let s3_info = S3ObjectInfo {
            exists: true,
            storage_class: Some("STANDARD".to_string()),
            size_bytes: Some(1_073_741_824), // 1 GB
            retrieval_required: false,
        };

        let config = StorageCostConfig {
            storage_costs_per_gb_month: HashMap::new(),
            retrieval_costs_per_gb: HashMap::new(),
            retrieval_times_hours: HashMap::new(),
            compute_cost_per_hour: 0.50,
            compute_hours_per_video_gb: 0.015,
        };

        let costs = calculate_costs(&s3_info, &config);

        assert!(costs.retrieval_costs.is_none());
        assert!(costs.retrieval_times.is_none());
        assert_eq!(costs.compute_cost, 0.0075); // 1 GB * 0.015 hr/GB * $0.50/hr
    }

    #[test]
    fn test_calculate_costs_with_retrieval() {
        let s3_info = S3ObjectInfo {
            exists: true,
            storage_class: Some("GLACIER".to_string()),
            size_bytes: Some(52_428_800_000), // ~50 GB
            retrieval_required: true,
        };

        let mut retrieval_costs_per_gb = HashMap::new();
        let mut retrieval_times_hours = HashMap::new();

        retrieval_costs_per_gb.insert(
            "GLACIER".to_string(),
            crate::config::RetrievalOptions {
                bulk: 0.0025,
                standard: 0.01,
            },
        );

        retrieval_times_hours.insert(
            "GLACIER".to_string(),
            crate::config::RetrievalOptions {
                bulk: 8.0,
                standard: 4.0,
            },
        );

        let config = StorageCostConfig {
            storage_costs_per_gb_month: HashMap::new(),
            retrieval_costs_per_gb,
            retrieval_times_hours,
            compute_cost_per_hour: 0.50,
            compute_hours_per_video_gb: 0.015,
        };

        let costs = calculate_costs(&s3_info, &config);

        assert!(costs.retrieval_costs.is_some());
        let cost_map = costs.retrieval_costs.unwrap();
        // ~50 GB * $0.0025/GB = ~$0.12 bulk, ~50 GB * $0.01/GB = ~$0.49 standard
        assert!(cost_map.contains_key("bulk"));
        assert!(cost_map.contains_key("standard"));

        assert!(costs.retrieval_times.is_some());
        let times = costs.retrieval_times.unwrap();
        assert_eq!(times.get("bulk"), Some(&8.0));
        assert_eq!(times.get("standard"), Some(&4.0));

        // ~50 GB * 0.015 hr/GB * $0.50/hr = ~$0.37
        assert!((costs.compute_cost - 0.366).abs() < 0.01);
    }
}
