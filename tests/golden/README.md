# Approved golden references

`phase-1/colour-alpha/` contains five 16-bit PNGs and five raw linear readbacks
from Windows 11 / RX 6800 XT / D3D12. The owner approved their exact hashes in
proposal `c9397f0` under ADR 0030. They were copied byte-for-byte; `approved.json`
retains the full provenance and integrity hashes.

Run `cargo xtask gen-assets`, then `cargo xtask golden` on the reference
machine. M4 may run `golden --development`, explicitly non-normative. See
[golden-tests.md](../../docs/user/golden-tests.md) for metrics and failure outputs.
No reference mutation is automatic. New hashes require human approval.

Phase 0's historical pixel-free inventory is retained in its phase summary.
