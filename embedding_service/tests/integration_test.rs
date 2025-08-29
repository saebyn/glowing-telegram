use aws_sdk_dynamodb::types::AttributeValue;
use std::collections::HashMap;
use std::process::Stdio;
use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::localstack::LocalStack;
use testcontainers_modules::postgres::Postgres;
use tokio::process::Command;
use tokio::time::{sleep, timeout};
use tokio_postgres::NoTls;

mod test_config;
use test_config::TestConfig;

/// Integration test for embedding service with transcription data
#[tokio::test]
#[ignore]
async fn test_embedding_service_with_transcription_data() {
    let config = TestConfig::from_env();

    println!("🚀 Starting embedding_service integration test");
    println!("📋 Test configuration: {:?}", config);

    // Start LocalStack container for AWS services
    println!("🐳 Starting LocalStack container...");
    let localstack = timeout(
        config.localstack_startup_timeout,
        LocalStack::default()
            .with_env_var("SERVICES", "dynamodb,secretsmanager")
            .start(),
    )
    .await
    .expect("LocalStack startup timed out")
    .expect("Failed to start LocalStack container");

    let localstack_port = localstack
        .get_host_port_ipv4(4566)
        .await
        .expect("Failed to get LocalStack port");

    println!("✅ LocalStack started on port {}", localstack_port);

    // Start PostgreSQL container with pgvector extension
    println!("🐳 Starting PostgreSQL container with pgvector...");
    let postgres = timeout(
        config.postgres_startup_timeout,
        Postgres::default()
            .with_env_var("POSTGRES_DB", &config.test_database)
            .with_env_var("POSTGRES_USER", &config.test_postgres_user)
            .with_env_var("POSTGRES_PASSWORD", &config.test_postgres_password)
            .with_tag("pg16")
            .with_name("pgvector/pgvector")
            .start(),
    )
    .await
    .expect("PostgreSQL startup timed out")
    .expect("Failed to start PostgreSQL container");

    let postgres_port = postgres
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get PostgreSQL port");

    println!("✅ PostgreSQL started on port {}", postgres_port);

    // Start mock OpenAI server if needed
    let mock_openai_port = if config.use_mock_openai {
        let port = start_mock_openai_server().await;
        println!("✅ Started mock OpenAI server on port {}", port);
        Some(port)
    } else {
        println!("📡 Using real OpenAI API");
        None
    };

    // Set up AWS clients for LocalStack
    let localstack_endpoint = format!("http://localhost:{}", localstack_port);
    let aws_config =
        aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&localstack_endpoint)
            .region("us-east-1")
            .credentials_provider(aws_sdk_dynamodb::config::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

    let dynamodb_client = aws_sdk_dynamodb::Client::new(&aws_config);
    let secrets_client = aws_sdk_secretsmanager::Client::new(&aws_config);

    // Test identifiers
    let test_item_key =
        format!("test-embedding-{}", chrono::Utc::now().timestamp());
    let test_stream_id =
        format!("test-stream-{}", chrono::Utc::now().timestamp());

    println!("🗄️ Setting up test infrastructure...");

    // Create DynamoDB table
    create_dynamodb_table(&dynamodb_client, &config).await;
    println!("✅ Created DynamoDB table: {}", config.test_table);

    // Create secrets in SecretsManager
    create_test_secrets(&secrets_client, &config).await;
    println!("✅ Created test secrets in SecretsManager");

    // Insert test data with transcriptions
    setup_test_dynamodb_data(
        &dynamodb_client,
        &config.test_table,
        &test_item_key,
        &test_stream_id,
    )
    .await;
    println!("✅ Inserted test data with transcriptions into DynamoDB");

    // Verify LocalStack is accessible before running container
    println!("🔍 Verifying LocalStack accessibility...");
    let health_check = tokio::process::Command::new("curl")
        .args(&[
            "-s",
            &format!("http://localhost:{}/health", localstack_port),
        ])
        .output()
        .await;

    match health_check {
        Ok(output) if output.status.success() => {
            println!("✅ LocalStack health check passed");
            println!(
                "   Response: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        Ok(output) => {
            println!(
                "⚠️ LocalStack health check failed with status: {:?}",
                output.status
            );
            println!("   Stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            println!("⚠️ Could not perform LocalStack health check: {}", e);
        }
    }

    // Test processing a single video clip
    println!("🏃 Running embedding_service container for single video...");
    let run_result = timeout(
        config.container_run_timeout,
        run_embedding_service_container(
            &config,
            &localstack_endpoint,
            &test_item_key,
            postgres_port,
            mock_openai_port,
            "process",
        ),
    )
    .await;

    let container_output = match run_result {
        Ok(Ok(output)) => {
            println!("✅ Container completed successfully");
            output
        }
        Ok(Err(e)) => panic!("❌ Container run failed: {}", e),
        Err(_) => panic!(
            "❌ Container run timed out after {:?}",
            config.container_run_timeout
        ),
    };

    // Verify results in PostgreSQL
    println!("🔍 Verifying embedding results in PostgreSQL...");
    verify_embedding_results(
        &config,
        postgres_port,
        &test_item_key,
        &test_stream_id,
        &container_output,
    )
    .await;

    // Test scanning a stream
    println!("🏃 Running embedding_service container for stream scan...");
    let scan_result = timeout(
        config.container_run_timeout,
        run_embedding_service_container(
            &config,
            &localstack_endpoint,
            &test_stream_id,
            postgres_port,
            mock_openai_port,
            "scan-stream",
        ),
    )
    .await;

    match scan_result {
        Ok(Ok(_)) => println!("✅ Stream scan completed successfully"),
        Ok(Err(e)) => panic!("❌ Stream scan failed: {}", e),
        Err(_) => panic!(
            "❌ Stream scan timed out after {:?}",
            config.container_run_timeout
        ),
    };

    println!("🎉 Embedding service integration test completed successfully!");

    if config.keep_containers_for_debug {
        println!(
            "🐛 Keeping containers running for debugging (TEST_KEEP_CONTAINERS=true)"
        );
        println!("   LocalStack endpoint: {}", localstack_endpoint);
        println!("   PostgreSQL port: {}", postgres_port);
        if let Some(port) = mock_openai_port {
            println!("   Mock OpenAI port: {}", port);
        }
        println!("   Press Ctrl+C to stop the test and clean up containers");
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl+c");
    }
}

async fn create_dynamodb_table(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    config: &TestConfig,
) {
    dynamodb_client
        .create_table()
        .table_name(&config.test_table)
        .key_schema(
            aws_sdk_dynamodb::types::KeySchemaElement::builder()
                .attribute_name("key")
                .key_type(aws_sdk_dynamodb::types::KeyType::Hash)
                .build()
                .expect("Failed to build key schema"),
        )
        .attribute_definitions(
            aws_sdk_dynamodb::types::AttributeDefinition::builder()
                .attribute_name("key")
                .attribute_type(
                    aws_sdk_dynamodb::types::ScalarAttributeType::S,
                )
                .build()
                .expect("Failed to build attribute definition"),
        )
        .attribute_definitions(
            aws_sdk_dynamodb::types::AttributeDefinition::builder()
                .attribute_name("stream_id")
                .attribute_type(
                    aws_sdk_dynamodb::types::ScalarAttributeType::S,
                )
                .build()
                .expect("Failed to build stream_id attribute definition"),
        )
        .global_secondary_indexes(
            aws_sdk_dynamodb::types::GlobalSecondaryIndex::builder()
                .index_name("stream_id-index")
                .key_schema(
                    aws_sdk_dynamodb::types::KeySchemaElement::builder()
                        .attribute_name("stream_id")
                        .key_type(aws_sdk_dynamodb::types::KeyType::Hash)
                        .build()
                        .expect("Failed to build GSI key schema"),
                )
                .projection(
                    aws_sdk_dynamodb::types::Projection::builder()
                        .projection_type(
                            aws_sdk_dynamodb::types::ProjectionType::All,
                        )
                        .build(),
                )
                .build()
                .expect("Failed to build GSI"),
        )
        .billing_mode(aws_sdk_dynamodb::types::BillingMode::PayPerRequest)
        .send()
        .await
        .expect("Failed to create DynamoDB table");

    // Wait for table to be ready
    sleep(config.dynamodb_table_creation_timeout).await;
}

async fn create_test_secrets(
    secrets_client: &aws_sdk_secretsmanager::Client,
    config: &TestConfig,
) {
    // Create database credentials secret
    let db_credentials = serde_json::json!({
        "username": config.test_postgres_user,
        "password": config.test_postgres_password
    });

    secrets_client
        .create_secret()
        .name("test/database/credentials")
        .description("Test database credentials for integration tests")
        .secret_string(db_credentials.to_string())
        .send()
        .await
        .expect("Failed to create database credentials secret");

    // Create OpenAI API key secret
    let openai_key = if config.use_mock_openai {
        "test-api-key".to_string()
    } else {
        config
            .openai_api_key
            .clone()
            .expect("OPENAI_API_KEY required when not using mock")
    };

    secrets_client
        .create_secret()
        .name("test/openai/api-key")
        .description("Test OpenAI API key for integration tests")
        .secret_string(openai_key)
        .send()
        .await
        .expect("Failed to create OpenAI API key secret");
}

async fn setup_test_dynamodb_data(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    table_name: &str,
    item_key: &str,
    stream_id: &str,
) {
    let mut item = HashMap::new();
    item.insert("key".to_string(), AttributeValue::S(item_key.to_string()));
    item.insert(
        "stream_id".to_string(),
        AttributeValue::S(stream_id.to_string()),
    );

    // Add transcription data
    let transcription_data = HashMap::from([
        ("text".to_string(), AttributeValue::S("This is a test transcription of a video clip. It contains some sample text that should be converted into embeddings for vector search functionality.".to_string())),
        ("language".to_string(), AttributeValue::S("en".to_string())),
        ("confidence".to_string(), AttributeValue::N("0.95".to_string())),
    ]);
    item.insert(
        "transcription".to_string(),
        AttributeValue::M(transcription_data),
    );

    // Add summary data
    let summary_data = HashMap::from([
        ("summary_main_discussion".to_string(), AttributeValue::S("Test video summary discussing various topics related to streaming and content creation.".to_string())),
        ("title".to_string(), AttributeValue::S("Test Video Title".to_string())),
    ]);
    item.insert("summary".to_string(), AttributeValue::M(summary_data));

    // Add metadata
    let format_data = HashMap::from([
        (
            "format_name".to_string(),
            AttributeValue::S("mp4".to_string()),
        ),
        (
            "duration".to_string(),
            AttributeValue::N("120.5".to_string()),
        ),
    ]);
    let metadata = HashMap::from([(
        "format".to_string(),
        AttributeValue::M(format_data),
    )]);
    item.insert("metadata".to_string(), AttributeValue::M(metadata));

    dynamodb_client
        .put_item()
        .table_name(table_name)
        .set_item(Some(item))
        .send()
        .await
        .expect("Failed to insert test data into DynamoDB");
}

async fn start_mock_openai_server() -> u16 {
    // Simple mock server that returns dummy embeddings
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{Body, Method, Request, Response, Server, StatusCode};
    use std::convert::Infallible;
    use std::net::SocketAddr;
    use tokio::task;

    async fn mock_openai_handler(
        req: Request<Body>,
    ) -> Result<Response<Body>, Infallible> {
        // Debug: log all incoming requests
        println!(
            "🔍 Mock OpenAI server received request: {} {}",
            req.method(),
            req.uri().path()
        );

        match (req.method(), req.uri().path()) {
            (&Method::POST, "/v1/embeddings")
            | (&Method::POST, "/embeddings") => {
                println!("✅ Mock OpenAI server: Handling embeddings request");
                let mock_response = serde_json::json!({
                    "object": "list",
                    "data": [{
                        "object": "embedding",
                        "index": 0,
                        "embedding": vec![0.1; 1536] // Mock 1536-dimensional embedding
                    }],
                    "model": "text-embedding-3-small",
                    "usage": {
                        "prompt_tokens": 10,
                        "total_tokens": 10
                    }
                });

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(Body::from(mock_response.to_string()))
                    .unwrap())
            }
            _ => {
                println!(
                    "❌ Mock OpenAI server: No handler for {} {}",
                    req.method(),
                    req.uri().path()
                );
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Not Found"))
                    .unwrap())
            }
        }
    }

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(mock_openai_handler))
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let server = Server::bind(&addr).serve(make_svc);
    let port = server.local_addr().port();

    task::spawn(async move {
        if let Err(e) = server.await {
            eprintln!("Mock OpenAI server error: {}", e);
        }
    });

    // Give server time to start
    sleep(std::time::Duration::from_millis(100)).await;
    port
}

async fn run_embedding_service_container(
    config: &TestConfig,
    localstack_endpoint: &str,
    identifier: &str,
    postgres_port: u16,
    mock_openai_port: Option<u16>,
    command: &str,
) -> Result<String, String> {
    let container_name = format!(
        "test-embedding-service-{}-{}",
        command,
        chrono::Utc::now().timestamp()
    );

    // Store formatted strings to avoid temporary value issues
    let dynamodb_table_env = format!("DYNAMODB_TABLE={}", config.test_table);
    let database_port_env = format!("DATABASE_PORT={}", postgres_port);
    let database_name_env = format!("DATABASE_NAME={}", config.test_database);
    let aws_endpoint_env = format!("AWS_ENDPOINT_URL={}", localstack_endpoint);

    // Create OpenAI base URL env var if needed
    let openai_base_url_env = mock_openai_port
        .map(|port| format!("OPENAI_BASE_URL=http://localhost:{}", port));

    let mut docker_args = vec![
        "run",
        "--rm",
        "--name",
        &container_name,
        "--network",
        "host", // Use host network to access LocalStack and PostgreSQL
        "-e",
        &dynamodb_table_env,
        "-e",
        "DATABASE_SECRET_ARN=test/database/credentials",
        "-e",
        "DATABASE_ENDPOINT=localhost",
        "-e",
        &database_port_env,
        "-e",
        &database_name_env,
        "-e",
        "OPENAI_SECRET_ARN=test/openai/api-key",
        "-e",
        "OPENAI_MODEL=text-embedding-3-small",
        "-e",
        &aws_endpoint_env,
        "-e",
        "AWS_ACCESS_KEY_ID=test",
        "-e",
        "AWS_SECRET_ACCESS_KEY=test",
        "-e",
        "AWS_DEFAULT_REGION=us-east-1",
        "-e",
        "RUST_LOG=debug",
    ];

    // Add OpenAI base URL if using mock
    if let Some(ref env_var) = openai_base_url_env {
        docker_args.extend(&["-e", env_var]);
    }

    // Add image and command
    docker_args.push(&config.image_name);
    docker_args.push(command);
    docker_args.push(identifier);

    // Debug: print the docker command being executed
    println!(
        "🐳 Executing docker command: docker {}",
        docker_args.join(" ")
    );

    let output = Command::new("docker")
        .args(&docker_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to execute docker command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Container execution failed with exit code {:?}.\nStdout: {}\nStderr: {}",
            output.status.code(),
            stdout,
            stderr
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn verify_embedding_results(
    config: &TestConfig,
    postgres_port: u16,
    item_key: &str,
    stream_id: &str,
    container_output: &str,
) {
    println!("Container output: {}", container_output);

    // Connect to PostgreSQL
    let connection_string = format!(
        "host=localhost port={} dbname={} user={} password={}",
        postgres_port,
        config.test_database,
        config.test_postgres_user,
        config.test_postgres_password
    );

    let (client, connection) =
        tokio_postgres::connect(&connection_string, NoTls)
            .await
            .expect("Failed to connect to PostgreSQL");

    // Spawn the connection task
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    // Check that embeddings table exists
    let table_exists = client
        .query_one(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'embeddings')",
            &[],
        )
        .await
        .expect("Failed to check if embeddings table exists");

    let exists: bool = table_exists.get(0);
    assert!(exists, "Embeddings table should exist");
    println!("✅ Embeddings table exists");

    // Check that pgvector extension is installed
    let extension_exists = client
        .query_one(
            "SELECT EXISTS (SELECT FROM pg_extension WHERE extname = 'vector')",
            &[],
        )
        .await
        .expect("Failed to check if vector extension exists");

    let ext_exists: bool = extension_exists.get(0);
    assert!(ext_exists, "Vector extension should be installed");
    println!("✅ pgvector extension is installed");

    // Check that embeddings were created for the test item
    let embedding_count = client
        .query_one(
            "SELECT COUNT(*) FROM embeddings WHERE video_key = $1",
            &[&item_key],
        )
        .await
        .expect("Failed to count embeddings");

    let count: i64 = embedding_count.get(0);
    assert!(count > 0, "Should have created at least one embedding");
    println!("✅ Found {} embeddings for video key {}", count, item_key);

    // Check embedding content types
    let embeddings = client
        .query(
            "SELECT content_type, content, embedding FROM embeddings WHERE video_key = $1",
            &[&item_key],
        )
        .await
        .expect("Failed to fetch embeddings");

    let mut found_transcription = false;
    let mut found_summary = false;

    for row in embeddings {
        let content_type: String = row.get(0);
        let content: String = row.get(1);
        let embedding: pgvector::Vector = row.get(2);
        let embedding_vec: Vec<f32> = embedding.to_vec();

        match content_type.as_str() {
            "transcription" => {
                found_transcription = true;
                assert!(
                    content.contains("test transcription"),
                    "Transcription content should match"
                );
                assert_eq!(
                    embedding_vec.len(),
                    1536,
                    "Embedding should have 1536 dimensions"
                );
                println!("✅ Transcription embedding verified");
            }
            "summary" => {
                found_summary = true;
                assert!(
                    content.contains("Test video summary")
                        || content.contains("Test Video Title"),
                    "Summary content should match"
                );
                assert_eq!(
                    embedding_vec.len(),
                    1536,
                    "Embedding should have 1536 dimensions"
                );
                println!("✅ Summary embedding verified");
            }
            _ => panic!("Unexpected content type: {}", content_type),
        }
    }

    assert!(
        found_transcription,
        "Should have created transcription embedding"
    );
    // Summary is optional based on data availability
    if found_summary {
        println!("✅ Summary embedding also created");
    }

    // Verify stream_id is correctly set
    let stream_check = client
        .query_one(
            "SELECT stream_id FROM embeddings WHERE video_key = $1 LIMIT 1",
            &[&item_key],
        )
        .await
        .expect("Failed to check stream_id");

    let stored_stream_id: String = stream_check.get(0);
    assert_eq!(stored_stream_id, stream_id, "Stream ID should match");
    println!("✅ Stream ID correctly stored: {}", stored_stream_id);

    // Check that vector index exists
    let index_exists = client
        .query_one(
            "SELECT EXISTS (SELECT FROM pg_indexes WHERE indexname = 'idx_embeddings_embedding_hnsw')",
            &[],
        )
        .await
        .expect("Failed to check if HNSW index exists");

    let idx_exists: bool = index_exists.get(0);
    assert!(idx_exists, "HNSW vector index should exist");
    println!("✅ HNSW vector index exists for similarity search");
}
