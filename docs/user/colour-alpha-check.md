# Check PNG alpha and linear colour on the native GPU

This short functional diagnostic uses D3D12 on Windows, Metal on macOS and
Vulkan on Linux. It requires a real GPU and does not require an idle machine.
It is not a timing benchmark, streaming preview, golden comparison or phase gate.

On the Windows reference checkout, fetch the prepared slice and switch to it
(use your remote's name if it is not `origin`):

```powershell
git fetch origin
git switch phase-1/colour-alpha-check
cargo run --locked -p rezie-gpu --bin rezie-colour-check -- --output docs/testing/phase-1-colour-windows-x86_64
```

The pinned toolchain and MSVC setup are the same as for `rezie-pool-check`.
The output directory must **not** exist; its parent must exist. For a repeat,
use a fresh suffix such as `-v2`. The command never replaces earlier evidence.

Expected output files:

- `report.json`: adapter, backend, source hashes, per-case numerical errors
  and pass/fail. Windows reference identity should be AMD Radeon RX 6800 XT / Dx12.
- `input-alpha.png`: code-generated RGBA8 sRGB fixture, decoded before ingest.
- `black.png`, `white.png`, `colour.png`, `translucent.png`, `transparent.png`:
  actual GPU results exported at the PNG boundary.

Please return the whole output directory (or commit it on the slice branch),
including PNGs, and the terminal error if the command fails. The JSON records
the compiled shader/probe/checker hashes, so results can be matched to source.
Failure writes case metrics and output images where readback completed; adapter
or GPU validation failures instead return an error. A leftover output directory
after such a failure is intentionally preserved.

Five backgrounds each check 257×65 pixels against a double-precision oracle.
Raw premultiplied linear channels must have absolute error ≤0.002 and exported
RGBA8 channels error ≤2 code values. These diagnostic tolerances do not replace
the perceptual golden thresholds in SPEC §7.6. The odd dimensions exercise row
padding and workgroup bounds. All working GPU textures are Rgba16Float;
CPU readback is confined to this control-side diagnostic and PNG egress.

No reference images are installed by this command. After the Windows numerical
check passes, the next step is production golden candidates for human review.
The committed M4 images are development evidence only. Do not run
`cargo xtask golden --update` to turn these outputs into references.
