#!/bin/sh
cd "$(dirname "$0")"

# Generate types from schemas for Rust
./node_modules/.bin/quicktype -s schema --lang rs --visibility public --derive-debug --derive-clone --derive-partial-eq --skip-serializing-none -o src/types.rs --src ../docs/v2/schemas/

# quicktype does not emit serde's deny_unknown_fields for closed schemas.
# Preserve the closed CutList, output-track, and source-route contracts.
node - <<'NODE'
const fs = require('node:fs');
const path = 'src/types.rs';
let source = fs.readFileSync(path, 'utf8');
for (const name of ['CutList', 'CutListClass', 'OutputTrack', 'SourceAudioRoute', 'SourceAudioPoint']) {
  const marker = `pub struct ${name} {`;
  if (source.split(marker).length !== 2) throw new Error(`expected exactly one ${name}`);
  source = source.replace(marker, `#[serde(deny_unknown_fields)]\n${marker}`);
}
for (const field of [
  'pub source_audio: Option<Vec<SourceAudioRoute>>,',
  'pub keyframes: Option<Vec<AudioChannelKeyframe>>,',
  'pub points: Option<Vec<SourceAudioPoint>>,'
]) {
  // keyframes also occurs on legacy AudioChannelMixing, which must stay permissive.
  const name = field.includes('source_audio:') ? 'OutputTrack' : 'SourceAudioRoute';
  const start = source.indexOf(`pub struct ${name} {`);
  const end = source.indexOf('\n}', start);
  if (start < 0 || end < 0) throw new Error(`missing struct ${name}`);
  let section = source.slice(start, end);
  const marker = `    #[serde(skip_serializing_if = "Option::is_none")]\n    ${field}`;
  if (section.split(marker).length !== 2) throw new Error(`expected one ${name}.${field}`);
  section = section.replace(marker, `    #[serde(default, deserialize_with = "crate::deserialize_present")]\n${marker}`);
  source = source.slice(0, start) + section + source.slice(end);
}
fs.writeFileSync(path, source);
NODE

# Generate types from schemas for TypeScript
./node_modules/.bin/quicktype -s schema --lang ts --prefer-unions --prefer-const-values --just-types --no-date-times -o src/types.ts --src ../docs/v2/schemas/

# Replace '[property: string]: any;' with 'unknown' in TypeScript types
sed -i 's/\[property: string\]: any;/\[property: string\]: unknown;/g' src/types.ts
