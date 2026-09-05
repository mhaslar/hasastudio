Phase 0 has no pixel payload or compositor paths (ADR 0001).
`cargo xtask golden` verifies this inventory and the generated Phase 0 asset
manifest, and reports zero frame comparisons explicitly. Tick correctness is
tested through the real engine integration harness. GPU image references and
their perceptual comparisons start with the consuming compositor phase.
