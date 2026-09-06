# Check PNG alpha and linear colour on the native GPU

This short functional diagnostic uses D3D12 on Windows, Metal on macOS and
Vulkan on Linux. It requires a real GPU and does not require an idle machine.
It is not a timing benchmark, streaming preview, golden comparison or phase gate.

On the Windows reference checkout, fetch the prepared slice and switch to it
(use your remote's name if it is not `origin`):

```powershell
git fetch origin
git switch phase-1/colour-alpha-check
git pull --ff-only
cargo run --locked -p rezie-gpu --bin rezie-colour-check -- --output docs/testing/phase-1-colour16-windows-x86_64
```

The pinned toolchain and MSVC setup are the same as for `rezie-pool-check`.
The output directory must **not** exist; its parent must exist. For a repeat,
use a fresh suffix such as `-v2`. The command never replaces earlier evidence.

Expected output files:

- `report.json`: adapter, backend, source hashes, per-case numerical errors
  and pass/fail. Windows reference identity should be AMD Radeon RX 6800 XT / Dx12.
- `input-alpha.png`: code-generated RGBA8 sRGB fixture, decoded before ingest.
- `black.png`, `white.png`, `colour.png`, `translucent.png`, `transparent.png`:
  actual GPU results exported directly as **16-bit-per-channel** sRGB RGBA.
- `black.rgba16f.le`, `white.rgba16f.le`, `colour.rgba16f.le`,
  `translucent.rgba16f.le`, `transparent.rgba16f.le`: exact working binary16
  readbacks, 133,640 bytes each. Layout and hashes are recorded in the JSON.

Please return the whole output directory (or commit it on the slice branch),
including PNGs and raw `.rgba16f.le` files, and the terminal error if the command fails. The JSON records
the compiled shader/probe/checker hashes, so results can be matched to source.
Failure writes case metrics and output images where readback completed; adapter
or GPU validation failures instead return an error. A leftover output directory
after such a failure is intentionally preserved.

Five backgrounds each check 257×65 pixels against a double-precision oracle.
Raw premultiplied linear channels must have absolute error ≤0.002. GPU egress
must differ from sRGB encoding of those **observed** channels by ≤2 16-bit
code values. The error against the ideal complete pipeline is recorded as a
separate statistic, including intermediate half-float rounding. These diagnostic tolerances do not replace
the perceptual golden thresholds in SPEC §7.6. The odd dimensions exercise row
padding and workgroup bounds. All working GPU textures are Rgba16Float;
CPU readback is confined to this control-side diagnostic and PNG egress.

No reference images are installed by this command. After the Windows numerical
check passes, the next step is production golden candidates for human review.
The committed M4 images are development evidence only. Do not run
`cargo xtask golden --update` to turn these outputs into references.

## Independent audit and raw layout

Optional on Windows (Python 3.9+); also usable later on any platform without a GPU:

```powershell
python tools/audit-colour16.py docs/testing/phase-1-colour16-windows-x86_64 --output docs/testing/phase-1-colour16-windows-x86_64/audit.json
```

The auditor uses only Python's standard library, verifies the source/PNG/raw
hashes, parses the PNG at full 16-bit precision, and recomputes both numerical
checks independently. It recognizes LF and CRLF source checkouts. It rejects
changed summaries and raw samples. The optional output path must be new.

Raw files have no header: 257×65 pixels, top-to-bottom rows and left-to-right
pixels, four IEEE 754 binary16 values in RGBA order, each little-endian, eight
bytes per pixel, row stride 2056 bytes, no padding. For example,
`struct.iter_unpack('<4e', raw_bytes)` decodes the samples in Python. These are
actual GPU bits, not values inferred from PNGs or generated from expected colours.
The report's `linear_readback_layout` and per-case `linear_readback` identify
this encoding, length and checksum. Preserve both PNG and raw files with the
report when returning evidence.

The older 8-bit directories are historical and should remain unchanged.
The owner has approved the Windows 16-bit PNG/raw hashes bound to `c9397f0`;
byte-for-byte references are now installed. See [golden-tests.md](golden-tests.md)
for the fresh-render comparison command. Approval alone is not a passing run.
