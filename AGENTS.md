# Agent Instructions

## Validation

- For Rust changes, run `cargo fmt`, `cargo test --workspace`, and
  `cargo clippy --workspace --all-features --all-targets`. Use the stable Rust
  toolchain and allow at least 10 minutes for workspace commands.
- For CDK changes, run `npm run build` and `npm test` from `cdk/`. The tests
  synthesize stacks, may invoke Docker, and can take several minutes.
- For changes that affect full CDK synthesis, also run
  `AWS_DEFAULT_REGION=us-east-1 AWS_REGION=us-east-1 SKIP_GITHUB_ENV=true npm run cdk synth`
  from `cdk/` when Docker is available.

## Safety

- Do not deploy CDK stacks, push container images, modify remote AWS resources,
  or run AWS-backed service binaries unless the user explicitly requests it
  and the required credentials and configuration are available.
