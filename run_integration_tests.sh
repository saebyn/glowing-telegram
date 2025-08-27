#!/usr/bin/env bash

# Integration Test Runner
# This script provides an easy way to run the container integration tests for any service

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
SERVICE=""
IMAGE_NAME=""
DOCKER_PREFIX="159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/"

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
Integration Test Runner

USAGE:
    $0 <SERVICE> [OPTIONS]

ARGUMENTS:
    SERVICE                 Service directory to run tests for (e.g., audio_transcriber, video_ingestor)

OPTIONS:
    -v, --verbose           Enable verbose output
    -n, --no-cleanup        Don't cleanup resources after test (for debugging)
    --build-timeout SECS    Container build timeout in seconds (default: 600)
    --run-timeout SECS      Container run timeout in seconds (default: 300)
    --image-name NAME       Docker image name to test (default: auto-generated ECR image)
    --docker-prefix PREFIX  Docker image prefix (default: 159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/)
    --build                 Build the Docker image before running tests
    -h, --help              Show this help message

EXAMPLES:
    # Run integration tests for audio_transcriber
    $0 audio_transcriber

    # Run integration tests with verbose output
    $0 audio_transcriber --verbose

    # Run without cleanup for debugging
    $0 video_ingestor --no-cleanup

    # Run with extended timeouts for slow systems
    $0 audio_transcriber --build-timeout 1200 --run-timeout 600

    # Run with custom image
    $0 audio_transcriber --image-name my-custom-image:latest

    # Build and test
    $0 audio_transcriber --build

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
    - Rust development environment (for Rust services)
    - Network access for downloading dependencies

SUPPORTED SERVICES:
    - audio_transcriber
    - video_ingestor
    - embedding_service
    - crud_api
    - ai_chat_lambda
    - And any other service with integration tests

EOF
}

# Function to generate default image name
generate_image_name() {
    local service=$1
    local docker_prefix=$2
    # Convert service name to match docker-bake.hcl target naming
    local docker_service=$(echo "$service" | sed 's/_/-/g')
    echo "${docker_prefix}${docker_service}:latest"
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
        --docker-prefix)
            DOCKER_PREFIX="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        -*)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
        *)
            if [[ -z "$SERVICE" ]]; then
                SERVICE="$1"
            else
                print_error "Too many arguments. Only one service can be specified."
                show_usage
                exit 1
            fi
            shift
            ;;
    esac
done

# Check if service was provided
if [[ -z "$SERVICE" ]]; then
    print_error "Service argument is required"
    show_usage
    exit 1
fi

# Generate default image name if not provided
if [[ -z "$IMAGE_NAME" ]]; then
    IMAGE_NAME=$(generate_image_name "$SERVICE" "$DOCKER_PREFIX")
fi

print_info "Integration Test Runner for $SERVICE"
print_info "========================================"

# Check if service directory exists
if [[ ! -d "$SERVICE" ]]; then
    print_error "Service directory '$SERVICE' not found"
    exit 1
fi

# Check if service has tests directory (for Rust services)
if [[ -f "$SERVICE/Cargo.toml" ]] && [[ ! -d "$SERVICE/tests" ]]; then
    print_warning "Service '$SERVICE' doesn't appear to have integration tests (no tests/ directory)"
    print_info "Continuing anyway in case tests are defined elsewhere..."
fi

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
    print_info "Building Docker image for $SERVICE..."
    
    # Check if docker-bake.hcl exists
    if [[ ! -f "docker-bake.hcl" ]]; then
        print_error "docker-bake.hcl not found in root directory"
        exit 1
    fi

    # Use service name as-is for docker-bake target (most targets use underscores)
    DOCKER_SERVICE="$SERVICE"
    
    print_info "Building docker image for target: $DOCKER_SERVICE"
    if ! docker buildx bake --load --file docker-bake.hcl "$DOCKER_SERVICE"; then
        print_error "Failed to build Docker image for $DOCKER_SERVICE"
        print_info "Make sure the service '$DOCKER_SERVICE' is defined in docker-bake.hcl"
        exit 1
    fi

    print_success "Docker image built successfully"
else
    print_info "Skipping Docker image build (use --build flag to build)"
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
print_info "  Service: $SERVICE"
print_info "  Build timeout: ${BUILD_TIMEOUT}s"
print_info "  Run timeout: ${RUN_TIMEOUT}s"
print_info "  Cleanup after test: $CLEANUP"
print_info "  Image name: $IMAGE_NAME"

# Change to service directory
cd "$SERVICE"

# Run the tests
print_info "Starting integration tests for $SERVICE..."

# Check if this is a Rust service
if [[ -f "Cargo.toml" ]]; then
    print_info "Running Rust integration tests..."
    if cargo test ${VERBOSE} -- --ignored --nocapture; then
        print_success "Integration tests completed successfully!"
    else
        print_error "Integration test failed!"
        exit 1
    fi
elif [[ -f "package.json" ]]; then
    # For Node.js services
    print_info "Running Node.js integration tests..."
    if npm run test:integration; then
        print_success "Integration tests completed successfully!"
    else
        print_error "Integration test failed!"
        exit 1
    fi
elif [[ -f "requirements.txt" ]] || [[ -f "pyproject.toml" ]]; then
    # For Python services
    print_info "Running Python integration tests..."
    if python -m pytest tests/integration; then
        print_success "Integration tests completed successfully!"
    else
        print_error "Integration test failed!"
        exit 1
    fi
else
    print_warning "Unknown service type for '$SERVICE'"
    print_info "Attempting to run cargo test anyway..."
    if cargo test ${VERBOSE} -- --ignored --nocapture; then
        print_success "Integration tests completed successfully!"
    else
        print_error "Integration test failed!"
        exit 1
    fi
fi

if [[ "$CLEANUP" == "false" ]]; then
    print_warning "Resources were not cleaned up (--no-cleanup flag used)"
    print_info "You may need to manually stop containers when done debugging"
fi

print_success "Integration test run completed for $SERVICE! 🎉"
