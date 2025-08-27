#!/usr/bin/env bash
set -e

# Integration test runner with docker-compose
# This script provides better isolation and reliability than testcontainers

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
print_error() { echo -e "${RED}❌ $1${NC}"; }

# Configuration from environment
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-embedding-test}"
USE_REAL_OPENAI="${USE_REAL_OPENAI:-false}"
BUILD_IMAGE="${BUILD_IMAGE:-true}"
CLEANUP="${CLEANUP:-true}"
VERBOSE="${VERBOSE:-false}"

print_info "Starting embedding service integration tests"
print_info "=============================================="

# Check prerequisites
if ! command -v docker &> /dev/null; then
    print_error "Docker is required but not installed"
    exit 1
fi

# Check for Docker Compose (v1 or v2)
if command -v docker-compose &> /dev/null; then
    DOCKER_COMPOSE="docker-compose"
elif docker compose version &> /dev/null; then
    DOCKER_COMPOSE="docker compose"
else
    print_error "Docker Compose is required but not installed"
    exit 1
fi

print_success "Prerequisites check passed (using $DOCKER_COMPOSE)"

# Cleanup function
cleanup() {
    if [[ "$CLEANUP" == "true" ]]; then
        print_info "Cleaning up test environment..."
        $DOCKER_COMPOSE -f docker-compose.test.yml -p "$COMPOSE_PROJECT_NAME" down -v --remove-orphans || true
        print_success "Cleanup completed"
    else
        print_warning "Skipping cleanup (CLEANUP=false)"
    fi
}

# Set up trap for cleanup
trap cleanup EXIT

# Build embedding service image if needed
if [[ "$BUILD_IMAGE" == "true" ]]; then
    print_info "Building embedding service test image..."
    cd ../../  # Go to workspace root
    
    # Try building with docker-bake first (more efficient)
    if docker buildx bake embedding_service --load; then
        print_success "Built using docker-bake"
        # Tag it for our test use
        docker tag 159222827421.dkr.ecr.us-west-2.amazonaws.com/glowing-telegram/embedding-service:latest embedding-test_embedding-service-test:latest
    else
        print_info "docker-bake failed, trying test Dockerfile..."
        cd "$SCRIPT_DIR"
        # Build using the test Dockerfile directly
        if ! $DOCKER_COMPOSE -f docker-compose.test.yml -p "$COMPOSE_PROJECT_NAME" build embedding-service-test; then
            print_error "Failed to build embedding service test image"
            exit 1
        fi
    fi
    
    cd "$SCRIPT_DIR"
    print_success "Test image build completed"
fi

# Start test services
print_info "Starting test infrastructure..."
$DOCKER_COMPOSE -f docker-compose.test.yml -p "$COMPOSE_PROJECT_NAME" up -d

# Wait for services to be healthy
print_info "Waiting for services to be ready..."
timeout 60 bash -c '
until '"$DOCKER_COMPOSE"' -f docker-compose.test.yml -p '"$COMPOSE_PROJECT_NAME"' ps | grep -q "healthy"; do
    echo "Waiting for services..."
    sleep 2
done
'

print_success "Test infrastructure is ready"

# Get service ports
POSTGRES_PORT=$($DOCKER_COMPOSE -f docker-compose.test.yml -p "$COMPOSE_PROJECT_NAME" port postgres-test 5432 | cut -d: -f2)
LOCALSTACK_PORT=$($DOCKER_COMPOSE -f docker-compose.test.yml -p "$COMPOSE_PROJECT_NAME" port localstack-test 4566 | cut -d: -f2)
MOCK_OPENAI_PORT=$($DOCKER_COMPOSE -f docker-compose.test.yml -p "$COMPOSE_PROJECT_NAME" port mock-openai 8080 | cut -d: -f2)

print_info "Service ports:"
print_info "  PostgreSQL: $POSTGRES_PORT"
print_info "  LocalStack: $LOCALSTACK_PORT"
print_info "  Mock OpenAI: $MOCK_OPENAI_PORT"

# Set environment variables for the test
export TEST_POSTGRES_PORT="$POSTGRES_PORT"
export TEST_LOCALSTACK_PORT="$LOCALSTACK_PORT"
export TEST_MOCK_OPENAI_PORT="$MOCK_OPENAI_PORT"
export TEST_USE_DOCKER_COMPOSE="true"
export TEST_COMPOSE_PROJECT_NAME="$COMPOSE_PROJECT_NAME"

# Run the actual integration test
print_info "Running integration tests..."
cd ../../  # Go to workspace root

if [[ "$VERBOSE" == "true" ]]; then
    cargo test -p embedding_service --test integration_test -- --ignored --nocapture
else
    cargo test -p embedding_service --test integration_test -- --ignored
fi

print_success "Integration tests completed successfully! 🎉"