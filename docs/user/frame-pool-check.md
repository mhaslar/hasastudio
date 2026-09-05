# Phase 1 GPU pool ownership check

This developer check exercises real GPU texture ownership. It does not display
video or satisfy a phase's compositor/performance gate. Use an actual supported
GPU; a CPU software adapter is rejected.

```sh
cargo run -p rezie-gpu --bin rezie-pool-check -- --output target/pool-check.json
```

The report records adapter/backend, working format, two-worker reuse, shared
lease retention, exhaustion, growth and byte-budget checks. Counters measure
texture/view creation calls, not driver-private memory allocations. No CPU
pixel/frame type is created. M4 results are development evidence only.

FramePool belongs to the control side and cannot move to a worker thread.
Publish a returned FrameReader to workers after reserving capacity. Keep the
pool owner alive until all workers stop. A Frame lease must survive **GPU
completion**, not merely command recording; dropping it earlier would let its
slot be reused while GPU commands still need the content. Collect retired
buckets on the control side after replacing readers and releasing old frames.

Engine dispatch, shared-device GUI rendering, decoders and NDI output are
still being implemented in Phase 1. Follow docs/phases/01-progress.md for the
remaining scope and the outstanding Windows clock obligation.
