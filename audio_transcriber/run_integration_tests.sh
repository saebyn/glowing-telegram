#!/usr/bin/env bash

# Audio Transcriber Integration Test Runner
# This script provides an easy way to run the container integration tests

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
VERBOSE=""
CLEANUP="true"
BUILD_TIMEOUT="600"
RUN_TIMEOUT="300"
BUILD="false"
IMAGE_NAME="159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/audio-transcription:latest"

# Function to print colored output
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Function to show usage
show_usage() {
    cat << EOF
Audio Transcriber Integration Test Runner

USAGE:
    $0 [OPTIONS]

OPTIONS:
    -v, --verbose           Enable verbose output
    -n, --no-cleanup        Don't cleanup resources after test (for debugging)
    --build-timeout SECS    Container build timeout in seconds (default: 600)
    --run-timeout SECS      Container run timeout in seconds (default: 300)
    --image-name NAME       Docker image name to test (default: ECR image)
    -h, --help              Show this help message
    --build                 Build the Docker image before running tests

EXAMPLES:
    # Run integration test
    $0

    # Run integration test with verbose output
    $0 --verbose

    # Run without cleanup for debugging
    $0 --no-cleanup

    # Run with extended timeouts for slow systems
    $0 --build-timeout 1200 --run-timeout 600

    # Run with custom image
    $0 --image-name my-custom-image:latest

ENVIRONMENT VARIABLES:
    You can also configure the tests using environment variables:
    
    TEST_BUILD_TIMEOUT      Container build timeout (seconds)
    TEST_RUN_TIMEOUT        Container run timeout (seconds)
    TEST_BUCKET             S3 bucket name for testing
    TEST_TABLE              DynamoDB table name for testing
    TEST_CLEANUP            Whether to cleanup after test (true/false)
    TEST_KEEP_CONTAINERS    Keep containers running for debugging (true/false)
    TEST_IMAGE_NAME         Docker image name to test

PREREQUISITES:
    - Docker must be installed and running
    - Rust development environment
    - Network access for downloading dependencies

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE="--verbose"
            shift
            ;;
        -n|--no-cleanup)
            CLEANUP="false"
            shift
            ;;
        --build-timeout)
            BUILD_TIMEOUT="$2"
            shift 2
            ;;
        --build)
            BUILD="true"
            shift
            ;;
        --run-timeout)
            RUN_TIMEOUT="$2"
            shift 2
            ;;
        --image-name)
            IMAGE_NAME="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

print_info "Audio Transcriber Integration Test Runner"
print_info "========================================"

# Check prerequisites
print_info "Checking prerequisites..."

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    print_error "Docker is not installed or not in PATH"
    exit 1
fi

# Check if Docker is running
if ! docker info &> /dev/null; then
    print_error "Docker is not running"
    exit 1
fi

print_success "Docker is available and running"

if [[ "$BUILD" == "true" ]]; then
    print_info "Building Docker image..."
    # Check if we're in the right directory
    if [[ ! -f "Cargo.toml" ]] || [[ ! -d "tests" ]]; then
        print_error "Must be run from the audio_transcriber directory"
        exit 1
    fi

    print_info "Building docker image..."
    # Build the Docker image with ../docker-bake.hcl

    # save current directory
    CURRENT_DIR=$(pwd)

    # Change to the parent directory to find docker-bake.hcl
    cd ..

    # Check if docker-bake.hcl exists
    if [[ ! -f "docker-bake.hcl" ]]; then
        print_error "docker-bake.hcl not found in parent directory"
        exit 1
    fi

    if ! docker buildx bake --load --file docker-bake.hcl audio_transcriber; then
        print_error "Failed to build Docker image"
        exit 1
    fi

    # Change back to the original directory
    cd "$CURRENT_DIR"

    print_success "Docker image built successfully"
else
    print_info "Skipping Docker image build as per --build flag"
fi

# Set environment variables
export TEST_BUILD_TIMEOUT="${BUILD_TIMEOUT}"
export TEST_RUN_TIMEOUT="${RUN_TIMEOUT}"
export TEST_CLEANUP="${CLEANUP}"
export TEST_IMAGE_NAME="${IMAGE_NAME}"

if [[ "$CLEANUP" == "false" ]]; then
    export TEST_KEEP_CONTAINERS="true"
fi

print_info "Configuration:"
print_info "  Build timeout: ${BUILD_TIMEOUT}s"
print_info "  Run timeout: ${RUN_TIMEOUT}s"
print_info "  Cleanup after test: $CLEANUP"
print_info "  Image name: $IMAGE_NAME"

# Run the tests
print_info "Starting integration tests..."

print_info "Running integration tests..."
if cargo test ${VERBOSE} --test integration_test -- --nocapture; then
    print_success "Integration tests completed successfully!"
else
    print_error "Integration test failed!"
    exit 1
fi

if [[ "$CLEANUP" == "false" ]]; then
    print_warning "Resources were not cleaned up (--no-cleanup flag used)"
    print_info "You may need to manually stop containers when done debugging"
fi

print_success "Integration test run completed! 🎉"
