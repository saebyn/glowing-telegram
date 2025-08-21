use serde::Deserialize;
use std::time::Duration;

/// Configuration for integration tests
#[derive(Debug, Deserialize, Clone)]
pub struct TestConfig {
    /// How long to wait for LocalStack services to be ready
    pub localstack_startup_timeout: Duration,

    /// How long to wait for DynamoDB table creation
    pub dynamodb_table_creation_timeout: Duration,

    /// Container build timeout
    pub container_build_timeout: Duration,

    /// Container run timeout
    pub container_run_timeout: Duration,

    /// Test bucket name
    pub test_bucket: String,

    /// Test DynamoDB table name
    pub test_table: String,

    /// Whether to cleanup resources after test
    pub cleanup_after_test: bool,

    /// Whether to keep containers running for debugging
    pub keep_containers_for_debug: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            localstack_startup_timeout: Duration::from_secs(30),
            dynamodb_table_creation_timeout: Duration::from_secs(10),
            container_build_timeout: Duration::from_secs(600), // 10 minutes
            container_run_timeout: Duration::from_secs(300),   // 5 minutes
            test_bucket: "test-input-bucket".to_string(),
            test_table: "test-table".to_string(),
            cleanup_after_test: true,
            keep_containers_for_debug: false,
        }
    }
}

impl TestConfig {
    /// Load configuration from environment variables with defaults
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("TEST_LOCALSTACK_TIMEOUT") {
            if let Ok(seconds) = val.parse::<u64>() {
                config.localstack_startup_timeout =
                    Duration::from_secs(seconds);
            }
        }

        if let Ok(val) = std::env::var("TEST_BUILD_TIMEOUT") {
            if let Ok(seconds) = val.parse::<u64>() {
                config.container_build_timeout = Duration::from_secs(seconds);
            }
        }

        if let Ok(val) = std::env::var("TEST_RUN_TIMEOUT") {
            if let Ok(seconds) = val.parse::<u64>() {
                config.container_run_timeout = Duration::from_secs(seconds);
            }
        }

        if let Ok(val) = std::env::var("TEST_BUCKET") {
            config.test_bucket = val;
        }

        if let Ok(val) = std::env::var("TEST_TABLE") {
            config.test_table = val;
        }

        if let Ok(val) = std::env::var("TEST_CLEANUP") {
            config.cleanup_after_test = val.to_lowercase() != "false";
        }

        if let Ok(val) = std::env::var("TEST_KEEP_CONTAINERS") {
            config.keep_containers_for_debug = val.to_lowercase() == "true";
        }

        config
    }
}
