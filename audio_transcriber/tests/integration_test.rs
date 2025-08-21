use aws_sdk_dynamodb::types::AttributeValue;
use bytes::Bytes;
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

/// integration test for the audio_transcriber container
#[tokio::test]
async fn test_audio_transcriber_integration() {
    let config = TestConfig::from_env();

    println!("🚀 Starting audio_transcriber integration test");
    println!("📋 Test configuration: {:?}", config);

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
        format!("test-video-{}", chrono::Utc::now().timestamp());
    let test_audio_key =
        format!("test-audio-{}.wav", chrono::Utc::now().timestamp());

    println!("🗄️ Setting up test infrastructure...");

    // Create S3 bucket
    s3_client
        .create_bucket()
        .bucket(&config.test_bucket)
        .send()
        .await
        .expect("Failed to create S3 bucket");
    println!("✅ Created S3 bucket: {}", config.test_bucket);

    // Upload realistic test audio file
    let test_audio_content = create_realistic_test_audio();
    s3_client
        .put_object()
        .bucket(&config.test_bucket)
        .key(&test_audio_key)
        .body(test_audio_content.into())
        .content_type("audio/wav")
        .send()
        .await
        .expect("Failed to upload test audio file");
    println!("✅ Uploaded test audio file: {}", test_audio_key);

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

    // Insert comprehensive test data
    setup_test_dynamodb_data(
        &dynamodb_client,
        &config.test_table,
        &test_item_key,
    )
    .await;
    println!("✅ Inserted test data into DynamoDB");

    // Run the container
    println!("🏃 Running audio_transcriber container...");
    let run_result = timeout(
        config.container_run_timeout,
        run_audio_transcriber_container(
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

    // Verify results
    println!("🔍 Verifying transcription results...");
    verify_transcription_results(
        &dynamodb_client,
        &config.test_table,
        &test_item_key,
        &container_output,
    )
    .await;

    println!("🎉 Integration test completed successfully!");

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

/// Integration test using real audio fixture file
#[tokio::test]
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

async fn setup_test_dynamodb_data(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    table_name: &str,
    item_key: &str,
) {
    let mut item = HashMap::new();
    item.insert("key".to_string(), AttributeValue::S(item_key.to_string()));

    // Add realistic silence segments
    let silence_segments = vec![
        // Opening silence
        AttributeValue::M(HashMap::from([
            ("start".to_string(), AttributeValue::N("0.0".to_string())),
            ("end".to_string(), AttributeValue::N("0.5".to_string())),
        ])),
        // Mid silence
        AttributeValue::M(HashMap::from([
            ("start".to_string(), AttributeValue::N("3.2".to_string())),
            ("end".to_string(), AttributeValue::N("3.8".to_string())),
        ])),
        // Ending silence
        AttributeValue::M(HashMap::from([
            ("start".to_string(), AttributeValue::N("9.5".to_string())),
            ("end".to_string(), AttributeValue::N("10.0".to_string())),
        ])),
    ];
    item.insert("silence".to_string(), AttributeValue::L(silence_segments));

    // Add metadata with duration and other format info
    let format_data = HashMap::from([
        (
            "duration".to_string(),
            AttributeValue::N("10.0".to_string()),
        ),
        (
            "bit_rate".to_string(),
            AttributeValue::N("1411200".to_string()),
        ),
        (
            "format_name".to_string(),
            AttributeValue::S("wav".to_string()),
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

async fn run_audio_transcriber_container(
    endpoint_url: &str,
    bucket: &str,
    table: &str,
    item_key: &str,
    audio_key: &str,
) -> Result<String, String> {
    let container_name =
        format!("test-audio-transcriber-{}", chrono::Utc::now().timestamp());
    let image_name = "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/audio-transcription:latest";

    let output = Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "--name", &container_name,
            "--network", "host", // Use host network to access LocalStack
            "-e", &format!("INPUT_BUCKET={}", bucket),
            "-e", &format!("DYNAMODB_TABLE={}", table),
            "-e", &format!("AWS_ENDPOINT_URL={}", endpoint_url),
            "-e", "DEVICE=cpu", // Use CPU for testing
            "-e", "AWS_ACCESS_KEY_ID=test",
            "-e", "AWS_SECRET_ACCESS_KEY=test",
            "-e", "AWS_REGION=us-east-1",
            "-e", "RUST_LOG=info", // Enable logging
            image_name,
            item_key,
            audio_key,
            "This is a comprehensive test transcription with multiple speakers and background noise.",
            "en"
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

async fn verify_transcription_results(
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
        "📝 Transcription result structure: {:?}",
        transcription_attr
    );

    // Verify transcription structure
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

                // Check first segment structure
                if let Some(AttributeValue::M(first_segment)) =
                    segments.first()
                {
                    let expected_fields = ["start", "end", "text"];
                    for field in &expected_fields {
                        assert!(
                            first_segment.contains_key(*field),
                            "Segment should contain '{}' field",
                            field
                        );
                    }
                    println!("✅ Transcription segment structure is valid");
                }
            }

            // Check for additional metadata if present
            if transcription_map.contains_key("language") {
                println!("✅ Language detection included in transcription");
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
            || container_output.contains("Processing"),
        "Container output should show processing activity"
    );

    println!("✅ All transcription verification checks passed!");
}

/// Creates a more realistic test audio file with multiple tones
/// This simulates a more complex audio scenario for testing
fn create_realistic_test_audio() -> Bytes {
    let sample_rate = 44100u32;
    let duration_samples = sample_rate * 10; // 10 seconds

    let mut wav_data = Vec::new();

    // WAV header
    wav_data.extend_from_slice(b"RIFF");
    wav_data.extend_from_slice(&(36 + duration_samples * 2).to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");

    // Format chunk
    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&16u32.to_le_bytes());
    wav_data.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav_data.extend_from_slice(&1u16.to_le_bytes()); // Mono
    wav_data.extend_from_slice(&sample_rate.to_le_bytes());
    wav_data.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav_data.extend_from_slice(&2u16.to_le_bytes());
    wav_data.extend_from_slice(&16u16.to_le_bytes());

    // Data chunk
    wav_data.extend_from_slice(b"data");
    wav_data.extend_from_slice(&(duration_samples * 2).to_le_bytes());

    // Generate more complex audio with multiple frequencies and volume changes
    for i in 0..duration_samples {
        let t = i as f32 / sample_rate as f32;

        // Create a more complex waveform simulating speech-like patterns
        let base_freq = 200.0; // Base frequency
        let modulation = (t * 2.0).sin() * 50.0; // Frequency modulation
        let frequency = base_freq + modulation;

        // Volume envelope to simulate speech patterns
        let volume = if t < 0.5 {
            0.1
        }
        // Initial silence
        else if t < 3.0 {
            0.8
        }
        // Speaking
        else if t < 4.0 {
            0.1
        }
        // Pause
        else if t < 8.0 {
            0.7
        }
        // More speaking
        else {
            0.1
        }; // Final silence

        let sample =
            (t * frequency * 2.0 * std::f32::consts::PI).sin() * volume;
        let sample_i16 = (sample * 32767.0) as i16;
        wav_data.extend_from_slice(&sample_i16.to_le_bytes());
    }

    Bytes::from(wav_data)
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
    let image_name = "159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/audio-transcription:latest";

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
            image_name,
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

#[cfg(test)]
mod test_helpers {
    use super::*;

    #[test]
    fn test_realistic_audio_creation() {
        let wav_data = create_realistic_test_audio();

        // Validate WAV structure
        assert!(wav_data.len() > 44, "WAV file should be larger than header");
        assert_eq!(&wav_data[0..4], b"RIFF", "Should start with RIFF");
        assert_eq!(
            &wav_data[8..12],
            b"WAVE",
            "Should contain WAVE identifier"
        );

        // Check expected duration (10 seconds at 44.1kHz, 16-bit mono)
        let expected_size = 44 + (44100 * 10 * 2); // Header + data
        assert_eq!(
            wav_data.len(),
            expected_size,
            "WAV file should be expected size"
        );
    }

    #[test]
    fn test_config_from_env() {
        // Test default configuration
        let config = TestConfig::default();
        assert_eq!(config.test_bucket, "test-input-bucket");
        assert_eq!(config.test_table, "test-table");
        assert!(config.cleanup_after_test);

        // Test environment variable loading
        std::env::set_var("TEST_BUCKET", "custom-bucket");
        std::env::set_var("TEST_TABLE", "custom-table");

        let config = TestConfig::from_env();
        assert_eq!(config.test_bucket, "custom-bucket");
        assert_eq!(config.test_table, "custom-table");

        // Cleanup
        std::env::remove_var("TEST_BUCKET");
        std::env::remove_var("TEST_TABLE");
    }
}
