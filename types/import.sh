#!/bin/sh
cd "$(dirname "$0")"
# Generate types from schemas for Rust
quicktype -s schema --lang rs --visibility public --derive-debug --derive-clone --derive-partial-eq --skip-serializing-none -o src/lib.rs --src ../docs/v2/schemas/
# Generate types from schemas for TypeScript
quicktype -s schema --lang ts --prefer-unions --prefer-const-values --just-types --no-date-times -o src/types.ts --src ../docs/v2/schemas/
# Replace '[property: string]: any;' with 'unknown' in TypeScript types
sed -i 's/\[property: string\]: any;/\[property: string\]: unknown;/g' src/types.ts