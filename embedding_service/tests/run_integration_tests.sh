#!/usr/bin/env bash
set -e

# Simple integration test runner using testcontainers
# This script runs the embedding service integration tests with LocalStack and PostgreSQL

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
print_success() { echo -e "${GREEN}✅ $1${NC}"; }

print_info "Starting embedding service integration tests with testcontainers"
print_info "=============================================================="

# Go to workspace root
cd "$SCRIPT_DIR/../../"

# Build the embedding service image first
print_info "Building embedding service image..."
if ! docker buildx bake embedding_service --load; then
    print_error "Failed to build embedding service image"
    exit 1
fi
print_success "Embedding service image built"

# Run the integration test
print_info "Running integration tests..."
cargo test -p embedding_service --test integration_test -- --ignored --nocapture

print_success "Integration tests completed successfully! 🎉"