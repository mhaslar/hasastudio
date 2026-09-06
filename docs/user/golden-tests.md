# Colour/alpha golden tests

The ten files under `tests/golden/phase-1/colour-alpha/` were explicitly
approved under ADR 0030, bound to proposal `c9397f0`, and installed byte-for-byte.
`approved.json` records their hashes, lengths, original Windows evidence and
measured source. The harness verifies them before rendering. Approval of these
files is distinct from a passing golden run.

## Windows reference run

On the Windows 11 / RX 6800 XT checkout, pull `phase-1/colour-alpha-check`, then:

```powershell
git pull --ff-only
cargo xtask gen-assets
cargo xtask golden --output docs/testing/phase-1-golden-windows-x86_64
```

No idle preparation is needed: this is correctness testing, not a performance
benchmark. The default command checks Windows 11 and RX hardware metadata,
then requires the renderer's actual selected adapter to be RX 6800 XT / D3D12.
No self-hosted runner is needed for the manual run.

Commit the **entire new output directory**, including `report.json` and the
`render/` subdirectory with PNGs, raw readbacks and renderer report. That provides
fresh rendering evidence and the full comparison metrics. If a run fails,
return its terminal error and `target/golden-failures/<run>/` artifacts as well.
Output directories must be new; use a `-v2` suffix for a repeat. Omitting
`--output` creates a unique directory under `target/golden-runs/`.

## M4 development check

```sh
cargo xtask gen-assets
cargo xtask golden --development --output docs/testing/phase-1-golden-macos-aarch64-v2
```

`--development` explicitly permits a native non-reference GPU. Its report says
`normative_reference_result: false`; it cannot satisfy the reference gate.
Hosted CI tests the comparator and stored evidence without executing a GPU.

## What is compared

`gen-assets` builds the approved 257×65 RGBA8 stimulus from code and writes it
to `tests/assets/phase-1/`. The golden command passes that file to the renderer
explicitly. Its decoded hash must match the approved stimulus. Each case
performs one alpha-over operation in linear premultiplied Rgba16Float and
exports a 16-bit sRGB PNG plus exact raw binary16 readback.

For each of five scenes, the comparator must pass all these checks:

- Mean CIEDE2000 **<1** and maximum **<3**, independently for appearance over
  linear black and white backgrounds. Conversion uses CIELAB with D65 white
  and unit CIEDE2000 parametric factors (ADR 0031).
- Alpha mean/max are reported separately; maximum normalized error **≤0.002**.
- Maximum raw linear channel difference against the approved readback **≤0.002**.
- The renderer's separate numerical checks against ideal linear results and
  16-bit encoding of observed working values also pass.

Numerical and perceptual checks are cumulative. Fully transparent hidden RGB
is not treated as visible colour. Alpha errors are not silently discarded.
D3D12/Metal byte equality is not required, and SHA-256 validates reference
integrity, not equality of a new rendering to the reference.

Failures write the actual 16-bit image, a colour-difference image and a
separate alpha-difference image under `target/golden-failures/<run>/`. Colour
heat is the larger ΔE00 over the two backgrounds, red at ΔE=3 or higher;
alpha heat is white at normalized error 0.002 or higher. Both heatmaps are
opaque so viewer backgrounds cannot hide them. Actual raw files are preserved.
If setup or malformed data prevents comparison, available actual artifacts and
an error JSON are retained; no valid difference image can be fabricated then.

## Auditing numerical readbacks later

```sh
python tools/audit-colour16.py docs/testing/phase-1-golden-windows-x86_64/render
```

This Python stdlib auditor checks full-precision PNG/raw values independently.
For older evidence, use `--source-revision <measured-commit>` to verify source
hashes against that explicit local git revision rather than the current files.
For the first approved 16-bit evidence the measured revision is `79113fd`.

The command never regenerates references. `golden --update` refuses mutation
and directs the operator to the required review. A future change starts with
new candidate files and explicit hash-bound human approval. Phase 4 will add
the repeated-blend scene; this harness does not claim stacked-overlay coverage.
