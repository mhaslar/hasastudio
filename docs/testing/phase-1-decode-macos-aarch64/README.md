# Native decode evidence — M4 / Metal development host

`auto/report.json` and `software/report.json` were produced against code
commit `8eed7414bcd5c0a934bf24ef38ae48bd11c0b78b`. The only worktree entry
was this evidence directory. Each report contains seven fixtures × 24
pictures, observed PTS/time base and canonical component hashes checked
against the independently authored fixture oracle. Both pass exactly.

Automatic mode observed actual VideoToolbox hardware sessions for H.264
and 8/10-bit HEVC; readback formats were NV12 and P010LE. VP9 and AV1 fall
back explicitly because the pinned FFmpeg has no matching VideoToolbox
configuration. The native log records dav1d 1.5.4. The software run verifies
`REZIE_DISABLE_HW_DECODE=1` and the absence of hardware contexts.

`native-guard-tests.json` retains the ten successful build/startup policy
checks run during implementation before the code commit; the guard source
was unchanged afterward. `system-ffmpeg-rejection.txt` is the relevant
literal diagnostic from the separate pre-commit negative build using the
real system GPL FFmpeg 9.0.1, libavcodec 63.1.101. It is deliberately an
excerpt, not a claim that all other build errors originated in the guard.

The two `unmodified-ffmpeg-*.json` files retain pre-commit replacement-library
experiments: compatible LGPL FFmpeg without the optional Mac accessor
decodes H.264 exactly in automatic software fallback; strict hardware fails
with the missing-accessor diagnostic. These experiments predate the final
source-bound reports and carry no reconstructed source hash.

`package-smoke.log` records the final package against `8eed741`: the relocated
GUI rendered with a live engine tick with loader overrides removed. It is
a startup/packaging check, not decoded-picture preview or performance evidence.

[Linux ci-fast](https://github.com/mhaslar/hasastudio/actions/runs/34036144315)
passed against the same commit, including native build, guard checks and
forced software decode. Windows execution and the full matrix are pending.
No performance or Phase 1 closure claim is made. `SHA256SUMS` covers the
retained measurement files, excluding this explanatory note and itself.
