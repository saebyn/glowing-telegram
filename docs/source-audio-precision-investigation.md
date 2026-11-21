# Source-audio precision investigation

This records the original investigation, not a declaration of preview/render
parity. Since the experiments below, an experimental second-based `points`
route has been added; see `source-audio-routing.md` for its current contract.
Corrections must remain render-blocked until the full contract and
implementation are independently verified. Legacy frame `keyframes` still use
frame-stepped `volume=eval=frame`, and nonzero gain below the f32 range remains
rejected; it must not be clamped or discarded.

## Local evidence

Experiments used FFmpeg 8.1.3, synthetic local media, and the actual GT
`render_cut_list` command. A local wrapper substituted a lossless f64 PCM
Matroska output for AAC/MP4, without changing the generated filter graph. A
two-section fixture used 48 kHz constant stereo input `(0.25, 0.5)`, a section
boundary at output second 1, and source-route gains ramping over frames 60–61
from `(0, 1)` to `(1, 0)`. Two points at frame 61 checked that the last won.
For sample offset `i` from second 1, the independent expected values are
`(0.25 * i / 800, 0.5 * (1 - i / 800))` for `0 <= i <= 800`, then held.

| Offset | Expected left, right | Actual existing graph |
| --- | --- | --- |
| 200 | 0.0625, 0.375 | 0.04, 0.42 |
| 400 | 0.125, 0.25 | 0.04, 0.42 |
| 799 | 0.2496875, 0.000625 | 0.04, 0.42 |
| 800 | 0.25, 0 | 0.04, 0.42 |

The existing graph reached `(0.25, 0)` by offset 1200. This is not AAC
distortion: the capture is lossless. In an experimental copy of the generated
graph, replacing each route's `volume` filter with
`aeval=exprs='val(0)*(EXPRESSION)'` produced the expected values at offsets
0, 1, 200, 400, 799, 800, 801, and 1000. `aeval`'s `t` follows input PTS,
not simply sample count from zero: adding ten seconds to PTS caused an
envelope starting at zero to be held at its endpoint. Correctly rebased,
padded and trimmed route timestamps are therefore essential.

A second synthetic fixture used a one-second crossfade and a gain ramp of
`x = (t - 1)` on the incoming section. For identical constant sources the
continuous ideal was `source * (1 - x + x*x)`. The experimental `aeval`
graph produced 0.187494792 at `t=1.5` for a 0.25 source, versus 0.1875 in
that ideal. The observed difference was about one 48 kHz fade step; inspect
FFmpeg `acrossfade` endpoint convention against the actual preview rather
than assuming that replacing `volume` alone proves parity.

`aeval` and `amix` negotiated double precision in a local physical-channel
graph. A gain of `1e-50` on `(0.25, 0.5)` yielded `(2.5e-51, 5e-51)` in f64;
applying `1e50` afterward restored `(0.25, 0.5)`. Inserting an f32 conversion
between those operations instead yielded zeros. `Number.MAX_SAFE_INTEGER`
times `1e-17` gave `0.09007199254740991` in f64. The final AAC encoder and
other graph filters must be examined for format negotiation, numerical
rounding, input-dependent overflow and underflow; a double-precision filter
alone does not establish end-to-end precision or justify removing the block.

## Concrete integration and resource blockers

- Authored points in the API are arbitrary strictly increasing **seconds**;
  the current GT DTO/schema accepts integer output **frames**. Rounding to
  60 fps can merge distinct points and change a ramp. A coordinated,
  lossless seconds representation and compiler/preview tests are required.
- The current nested linear expression grows in depth with point count.
  A local FFmpeg expression with 100 nested points failed to parse, while 10
  worked. Merely replacing `volume` with `aeval` retains that failure and
  also moves expression evaluation from once per audio frame to every sample.
  A prototype balanced decision expression handled 1,000 segments in a
  two-second synthetic run, but at 3,000 segments its single filter argument
  failed with `E2BIG`. On this machine `ARG_MAX` is 2 MiB, but a single
  Linux argument has a substantially smaller limit. Expression complexity,
  graph transport, parsing, memory and per-sample cost need an explicit safe
  design and actionable technical failures, not an arbitrary point cap.
- Continue using argument-vector invocation, never shell interpolation.
  Numbers originate from validated numeric fields; any replacement transport
  must keep graph quoting safe and bound resource use.
- Full-stream timing scans remain costly. Earlier local experiments found
  interval seeking can miss B-frames and nominal frame-rate metadata can hide
  a dropped frame. Reducing the scans without a trusted frame index or another
  proof of ordinal/timestamp continuity risks silently shifting audio.

Local scratch fixtures are under `/tmp/opencode/gt-sample-probe-sp5b6d3q`
and `/tmp/opencode/gt-aeval-fade-4ny0rui5`; they are synthetic and disposable.
