# Outstanding phase obligation

Exactly one open item. ADR 0023 authorizes conditional Phase 0 closure;
this item is not a claim of successful reference measurement.

- **Phase 0 — clock benchmark on Windows 11 / RX 6800 XT reference machine,
  including the slack sweep for that platform. Blocked on hardware availability.
  Due at the Phase 1 gate.**

Status: **OPEN**. The Windows calibrated slack is unset. Record the manual
sweep, reviewed/pinned Windows slack and passing ten-minute idle result with
all raw samples and host metadata before paying this item.

**If the Windows measurement fails, Phase 0 reopens and Phase 1 work stops
until rezie-rt is fixed.** Do not relax the maximum-lateness or percentile bounds.
Phase 1 cannot close until this item is paid. While it is open, no phase may
close conditionally; at most ONE outstanding item exists at any time.
