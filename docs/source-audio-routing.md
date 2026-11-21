# Per-section source audio routing

> **Work in progress; independent validation required before delivery.** The
> renderer now preserves tested late-start and early-end audio, but deliberately
> rejects some ambiguous or unsupported timelines rather than guessing.

`outputTrack[].sourceAudio` is an optional, **complete** list of physical
routes for that section. Omission preserves the original renderer behavior
(the first input audio stream). Supplying routes replaces that default; the
caller must include every intended contribution, including unaffected ones.
For example, to route a stereo stream unchanged:

```json
"sourceAudio": [
  { "streamIndex": 1, "sourceChannel": 0, "outputChannel": 0 },
  { "streamIndex": 1, "sourceChannel": 1, "outputChannel": 1 }
]
```

`streamIndex` counts **audio streams in that input**, starting at zero, not
all container streams and not channels in the HLS preview. `sourceChannel`
counts physical channels within that audio stream; `outputChannel` is 0 or 1
in the stereo output. Both output channels must be supplied. Multiple routes
to the same output channel are summed without normalization. The renderer
checks each source stream and channel against `ffprobe` before rendering. When
any explicit route is present it also validates every unchanged main-track
section, because a short intro/outro would otherwise shift later audio relative
to its output-time envelopes. Unchanged tracks must have complete original
audio coverage; otherwise rendering fails instead of modifying their defaults.
It decodes and drains each main-track video and selected audio stream to EOF,
validating timestamps through the last requested section and rejecting decoder errors.
This can add substantial latency and compute cost for long inputs and requires
performance review before delivery.

This first implementation requires an identifiable video and container start
at the same timestamp, video frames consistent with the output frame rate,
audio/video time bases no coarser than one millisecond, and finite, continuous
timestamped audio frames. It rejects video-late/audio-
leading media, missing timing metadata, decoding errors, video frame ranges
beyond the source, and internal audio discontinuities. These are technical safety restrictions,
not a decision that such media should remain unsupported. A common nonzero
input start is supported. Leading or trailing silence for a selected stream
whose available audio starts late or ends early is preserved.
The production callers invoke this check before constructing the command;
other `build_ffmpeg_command` callers must do the same with the identical frame
rate and input directory.

For new source corrections, optional `points` are a nonempty array of
`{"time": seconds, "gain": value}` with finite nonnegative, strictly
increasing absolute output-timeline seconds, including any intro and transition
overlap. Points are not rounded to video frames. The renderer evaluates their
linear interpolation per audio sample and holds endpoint values. Omit points
for unity. A route must not specify both `points` and legacy `keyframes`.
Omission is distinct from JSON or DynamoDB null: present `sourceAudio`,
`points`, or source-route `keyframes` must be arrays, never null. Null is
rejected before rendering; it cannot silently switch a correction to unity or
the legacy stream. This does not alter legacy top-level `audioMixing` null or
its route-keyframes compatibility.
The separate, older `keyframes` spelling continues to use output-timeline
video frames, last duplicate wins, and frame-stepped evaluation. The existing
`audioMixing` property also retains its frame-stepped behavior. Gains are
applied per source before crossfades and cuts,
so overlapping sections can use different mappings and envelopes. The existing
`audioMixing` property, if also supplied, still operates on the assembled
output afterward.

Authored gains must be finite and nonnegative; the nonzero gain must fit
FFmpeg's float volume processing (between the smallest positive f32 subnormal
and f32 maximum). This includes gains above 2 and JavaScript's
`Number.MAX_SAFE_INTEGER`; it is not a loudness limit. Values are not clamped.
Floating-point rounding, input-dependent overflow or underflow, and lossy output
encoding or clipping can still change the resulting samples. No limiter is added.
The double-precision per-sample path does not eliminate the eventual AAC/f32
format conversion. For now a positive `1e-50` gain is rejected, not rounded
to zero; the authored correction must remain saved and render-blocked.
Input-dependent signal underflow, overflow and encoding distortion are not
fully detectable from gain alone. Preview/render parity is not established.
On Linux, generated graphs that exceed the kernel's per-argument `execve`
limit are rejected before rendering; filter-script transport is not yet
implemented. This is a technical limit, not a product point-count limit.
Closed CutList, output-track, and route objects reject unknown fields so typos
cannot silently disable corrections. Keyframe and legacy mixing objects retain
their existing permissive extension behavior.

The caller is responsible for obtaining a trustworthy mapping for each input
and for choosing its complete mix. Do not infer physical indexes from names or
from the preview layout. This feature does not establish recording layouts,
alter tenant/input key selection, or perform final loudness normalization.
