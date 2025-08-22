use aws_sdk_dynamodb::types::AttributeValue;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::localstack::LocalStack;
use tokio::process::Command;
use tokio::time::{sleep, timeout};

mod test_config;
use test_config::TestConfig;

/// Integration test using real audio fixture file
#[tokio::test]
#[ignore]
async fn test_audio_transcriber_with_real_audio() {
    let config = TestConfig::from_env();

    println!("🚀 Starting audio_transcriber real audio integration test");
    println!("📋 Test configuration: {:?}", config);

    // Check if fixture file exists
    let fixture_path = "tests/fixtures/test_speech.wav";
    if !Path::new(fixture_path).exists() {
        panic!(
            "❌ Fixture file not found: {}\n\
            Please record the audio file as described in tests/fixtures/README.md",
            fixture_path
        );
    }

    // Start LocalStack container with timeout
    println!("🐳 Starting LocalStack container...");
    let localstack = timeout(
        config.localstack_startup_timeout,
        LocalStack::default()
            .with_env_var("SERVICES", "s3,dynamodb")
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

    // Set up AWS clients for LocalStack
    let endpoint_url = format!("http://localhost:{}", localstack_port);
    let aws_config =
        aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&endpoint_url)
            .region("us-east-1")
            .credentials_provider(aws_sdk_dynamodb::config::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

    let s3_client = aws_sdk_s3::Client::from_conf(
        aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .force_path_style(true)
            .endpoint_url(&endpoint_url)
            .region(aws_sdk_s3::config::Region::new("us-east-1"))
            .credentials_provider(aws_sdk_dynamodb::config::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .build(),
    );
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&aws_config);

    // Test identifiers
    let test_item_key =
        format!("test-real-audio-{}", chrono::Utc::now().timestamp());
    let test_audio_key =
        format!("real-test-audio-{}.wav", chrono::Utc::now().timestamp());

    println!("🗄️ Setting up test infrastructure...");

    // Create S3 bucket
    s3_client
        .create_bucket()
        .bucket(&config.test_bucket)
        .send()
        .await
        .expect("Failed to create S3 bucket");
    println!("✅ Created S3 bucket: {}", config.test_bucket);

    // Upload real audio fixture file
    println!("📁 Loading audio fixture: {}", fixture_path);
    let audio_content =
        fs::read(fixture_path).expect("Failed to read audio fixture file");

    s3_client
        .put_object()
        .bucket(&config.test_bucket)
        .key(&test_audio_key)
        .body(audio_content.into())
        .content_type("audio/wav")
        .send()
        .await
        .expect("Failed to upload real audio file");
    println!("✅ Uploaded real audio file: {}", test_audio_key);

    // Create DynamoDB table
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
        .billing_mode(aws_sdk_dynamodb::types::BillingMode::PayPerRequest)
        .send()
        .await
        .expect("Failed to create DynamoDB table");

    // Wait for table to be ready
    sleep(config.dynamodb_table_creation_timeout).await;
    println!("✅ Created DynamoDB table: {}", config.test_table);

    // Insert minimal test data (no pre-existing silence data for real audio)
    setup_minimal_test_dynamodb_data(
        &dynamodb_client,
        &config.test_table,
        &test_item_key,
    )
    .await;
    println!("✅ Inserted minimal test data into DynamoDB");

    // Run the container with real audio
    println!("🏃 Running audio_transcriber container with real audio...");
    let run_result = timeout(
        config.container_run_timeout,
        run_real_audio_transcriber_container(
            &config,
            &endpoint_url,
            &config.test_bucket,
            &config.test_table,
            &test_item_key,
            &test_audio_key,
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

    // Verify results with expected content
    println!("🔍 Verifying real audio transcription results...");
    verify_real_audio_transcription_results(
        &dynamodb_client,
        &config.test_table,
        &test_item_key,
        &container_output,
    )
    .await;

    println!("🎉 Real audio integration test completed successfully!");

    if config.keep_containers_for_debug {
        println!(
            "🐛 Keeping containers running for debugging (TEST_KEEP_CONTAINERS=true)"
        );
        println!("   LocalStack endpoint: {}", endpoint_url);
        println!("   Press Ctrl+C to stop the test and clean up containers");
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl+c");
    }
}

async fn setup_minimal_test_dynamodb_data(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    table_name: &str,
    item_key: &str,
) {
    let mut item = HashMap::new();
    item.insert("key".to_string(), AttributeValue::S(item_key.to_string()));

    // Add minimal metadata (real audio will be analyzed by the transcriber)
    let format_data = HashMap::from([(
        "format_name".to_string(),
        AttributeValue::S("wav".to_string()),
    )]);
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
        .expect("Failed to insert minimal test data into DynamoDB");
}

async fn run_real_audio_transcriber_container(
    config: &TestConfig,
    endpoint_url: &str,
    bucket: &str,
    table: &str,
    item_key: &str,
    audio_key: &str,
) -> Result<String, String> {
    let container_name = format!(
        "test-real-audio-transcriber-{}",
        chrono::Utc::now().timestamp()
    );

    let output = Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "--name",
            &container_name,
            "--network",
            "host", // Use host network to access LocalStack
            "-e",
            &format!("INPUT_BUCKET={}", bucket),
            "-e",
            &format!("DYNAMODB_TABLE={}", table),
            "-e",
            &format!("AWS_ENDPOINT_URL={}", endpoint_url),
            "-e",
            "DEVICE=cpu", // Use CPU for testing
            "-e",
            "AWS_ACCESS_KEY_ID=test",
            "-e",
            "AWS_SECRET_ACCESS_KEY=test",
            "-e",
            "AWS_REGION=us-east-1",
            "-e",
            "RUST_LOG=info", // Enable logging
            &config.image_name,
            item_key,
            audio_key,
            "",   // No initial prompt for real audio test
            "en", // English language
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to execute docker run: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("📋 Container stdout:\n{}", stdout);
    if !stderr.is_empty() {
        println!("📋 Container stderr:\n{}", stderr);
    }

    if !output.status.success() {
        return Err(format!(
            "Container failed with exit code {}\nSTDOUT:\n{}\nSTDERR:\n{}",
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        ));
    }

    Ok(stdout.to_string())
}

async fn verify_real_audio_transcription_results(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    table_name: &str,
    item_key: &str,
    container_output: &str,
) {
    // Get the updated item from DynamoDB
    let result = dynamodb_client
        .get_item()
        .table_name(table_name)
        .key("key", AttributeValue::S(item_key.to_string()))
        .send()
        .await
        .expect("Failed to get updated item from DynamoDB");

    let item = result
        .item
        .expect("Item not found in DynamoDB after transcription");

    // Verify transcription field exists
    let transcription_attr = item
        .get("transcription")
        .expect("Transcription field not found in DynamoDB item");

    println!(
        "📝 Real audio transcription result: {:?}",
        transcription_attr
    );

    // Verify transcription structure and content
    match transcription_attr {
        AttributeValue::M(transcription_map) => {
            // Check for expected fields in transcription
            assert!(
                transcription_map.contains_key("segments"),
                "Transcription should contain segments"
            );

            if let Some(AttributeValue::L(segments)) =
                transcription_map.get("segments")
            {
                println!("✅ Found {} transcription segments", segments.len());

                // Verify at least one segment exists
                assert!(
                    !segments.is_empty(),
                    "At least one transcription segment should exist"
                );

                // Extract all transcribed text for content verification
                let mut full_text = String::new();
                for segment in segments {
                    if let AttributeValue::M(segment_map) = segment {
                        if let Some(AttributeValue::S(text)) =
                            segment_map.get("text")
                        {
                            full_text.push_str(text);
                            full_text.push(' ');
                        }
                    }
                }

                let full_text = full_text.to_lowercase();
                println!("📝 Full transcribed text: {}", full_text);

                // Verify expected key phrases from the test recording
                let expected_phrases = [
                    "test recording",
                    "audio transcriber",
                    "quick brown fox",
                    "lazy dog",
                    "testing",
                ];

                let mut found_phrases = 0;
                for phrase in &expected_phrases {
                    if full_text.contains(phrase) {
                        println!("✅ Found expected phrase: '{}'", phrase);
                        found_phrases += 1;
                    } else {
                        println!(
                            "⚠️  Expected phrase not found: '{}'",
                            phrase
                        );
                    }
                }

                // Require at least 3 out of 5 expected phrases to account for transcription variations
                assert!(
                    found_phrases >= 3,
                    "Expected at least 3 out of 5 key phrases, found {} phrases in: {}",
                    found_phrases,
                    full_text
                );

                println!(
                    "✅ Found {}/{} expected phrases in transcription",
                    found_phrases,
                    expected_phrases.len()
                );
            }

            // Check for language detection
            if let Some(AttributeValue::S(language)) =
                transcription_map.get("language")
            {
                println!("✅ Detected language: {}", language);
                assert_eq!(
                    language, "en",
                    "Expected English language detection"
                );
            }
        }
        _ => panic!(
            "Transcription should be a map structure, got: {:?}",
            transcription_attr
        ),
    }

    // Verify container logs show expected behavior
    assert!(
        container_output.contains("Retrieved")
            || container_output.contains("Processing")
            || container_output.contains("Transcribing"),
        "Container output should show processing activity"
    );

    println!("✅ All real audio transcription verification checks passed!");
}
