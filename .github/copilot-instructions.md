# Glowing Telegram - Stream Recording Management System

Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.

## Working Effectively

### Bootstrap and Build (CRITICAL TIMING)
- Rust workspace check: `cargo check --workspace` -- takes 4-5 minutes on first run. NEVER CANCEL. Set timeout to 10+ minutes.
- Rust workspace build (debug): `cargo build --workspace` -- takes 2-3 minutes incrementally. NEVER CANCEL. Set timeout to 10+ minutes.
- Rust workspace build (release): `cargo build --release --workspace` -- takes 5-6 minutes. NEVER CANCEL. Set timeout to 15+ minutes.
- Rust tests: `cargo test --workspace` -- takes 10-15 seconds (minimal tests exist). Set timeout to 5+ minutes.
- CDK dependencies: `cd cdk && npm ci` -- takes 30-45 seconds. Set timeout to 5+ minutes.
- CDK build: `cd cdk && npm run build` -- takes 30-45 seconds. Set timeout to 5+ minutes.
- CDK tests: `cd cdk && npm test` -- takes 10-15 seconds. Set timeout to 5+ minutes.

### Linting and Formatting
- Rust linting: `cargo clippy --workspace --all-targets` -- takes 30-45 seconds. Set timeout to 10+ minutes.
- Rust formatting check: `cargo fmt --check` -- **WILL FAIL** due to formatting issues. Run `cargo fmt` to fix.
- TypeScript linting: `cd cdk && npx biome check .` -- takes 25-30 seconds. Set timeout to 5+ minutes.
- **WARNING**: rustfmt.toml uses unstable features requiring nightly Rust, but stable toolchain is used.

### Running Applications
- **CRITICAL**: All CLI tools require extensive AWS environment configuration.
- Required environment variables: `INPUT_BUCKET`, `OUTPUT_BUCKET`, `DYNAMODB_TABLE`, `AWS_REGION`, etc.
- Example: `./target/debug/video_ingestor <input_key>` (requires AWS credentials and S3 bucket config).
- Example: `./target/debug/audio_transcriber <item_key> <input_key> <initial_prompt> <language>` (requires AWS setup).
- **DO NOT** attempt to run CLI tools without proper AWS configuration - they will panic.

### Docker and Deployment  
- **WARNING**: Docker builds may fail in sandboxed environments due to network/SSL certificate issues.
- Build single image: `docker buildx bake <service_name>` where service_name is one of: ai_chat_lambda, audio_transcriber, crud_api, etc.
- Build all images: `docker buildx bake all` -- takes 15-30 minutes. NEVER CANCEL. Set timeout to 60+ minutes.
- Deploy single service: `./scripts/push_image.sh <service_name>` (requires AWS credentials).
- Deploy all services: `./scripts/push_all.sh` (requires AWS credentials).

## Validation
- Always run `cargo clippy --workspace --all-targets` after making Rust changes.
- Always run `cargo fmt` after making Rust changes (ignore warnings about unstable features).
- Always run `cd cdk && npm run build && npm test` after making CDK changes.
- NEVER attempt to run `npx cdk synth` or `npx cdk deploy` - these fail due to Docker dependency issues.
- **Manual Testing**: Applications require AWS infrastructure and cannot be fully tested locally without proper setup.

## Common Tasks

### Repository Structure Overview
```
├── Cargo.toml                 # Rust workspace with 18 member crates
├── cdk/                       # AWS CDK infrastructure (TypeScript)
├── docs/                      # Documentation and JSON schemas
├── scripts/                   # Deployment and utility scripts
├── .github/workflows/         # CI/CD pipelines (rust.yml, cdk.yml)
├── docker-bake.hcl           # Multi-service Docker build configuration
├── Dockerfile                # Multi-stage Docker builds for all services
├── biome.json                # TypeScript/JS linting and formatting config
├── rustfmt.toml              # Rust formatting config (uses unstable features)
└── [18 Rust crates]          # Individual service directories
```

### Key Rust Crates
- `video_ingestor` - Analyzes video files, extracts metadata, detects silence
- `audio_transcriber` - Transcribes audio using OpenAI Whisper
- `ai_chat_lambda` - OpenAI API wrapper for chat completion
- `crud_api` - DynamoDB CRUD operations
- `gt_ffmpeg` - FFmpeg interaction library
- `types` - Shared types generated from JSON schemas
- `gt_app`, `gt_axum`, `gt_secrets` - Shared utility libraries

### Key Commands Reference
```bash
# Development commands (timing critical)
cargo check --workspace                    # 4-5 min, NEVER CANCEL
cargo build --workspace                    # 2-3 min, NEVER CANCEL  
cargo build --release --workspace          # 5-6 min, NEVER CANCEL
cargo test --workspace                     # 10-15 sec
cargo clippy --workspace --all-targets     # 30-45 sec
cargo fmt                                  # Fix formatting issues

# CDK commands
cd cdk && npm ci                           # 30-45 sec
cd cdk && npm run build                    # 8-10 sec
cd cdk && npm test                         # 10-15 sec
cd cdk && npx biome check .               # 25-30 sec

# Type generation
./types/import.sh                          # Generate types from JSON schemas

# Docker (may fail in sandboxed environments)
docker buildx bake <service>              # 15-30 min, NEVER CANCEL
docker buildx bake all                    # 15-30 min, NEVER CANCEL
```

### VS Code Integration
- Rust-analyzer configured for workspace
- Build tasks available via Command Palette
- Tasks include: CDK deploy, container builds, cargo watch
- Uses AWS profile "glowing-telegram-admin" for deployment tasks

### GitHub Actions CI
- **rust.yml**: Runs `cargo build`, `cargo test`, and `cargo clippy` on PRs/pushes
- **cdk.yml**: Runs `npm ci`, `npm run build`, `npm test`, and `npx cdk synth` 
- **Dependencies**: Node.js 20, Rust stable toolchain

### Critical Warnings
- **NEVER CANCEL** long-running builds - they require significant time to complete
- Set timeouts of 10+ minutes for Rust builds, 60+ minutes for Docker builds
- Rust formatting config requires nightly but stable is used - expect warnings
- CLI applications require AWS infrastructure and cannot run without proper configuration
- Docker operations may fail due to network restrictions in sandboxed environments
- CDK synth fails due to Python Lambda build dependencies

### Schema and Type Management
- JSON schemas located in `docs/v2/schemas/`
- Types auto-generated for both Rust and TypeScript via quicktype
- Run `./types/import.sh` after schema changes
- Generated files: `types/src/types.rs` and `types/src/types.ts`

Always validate your changes by running the appropriate linting and build commands before committing. The CI system will catch issues, but local validation saves time.