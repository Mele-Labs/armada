// How full a limit is, as a ring draws it. Mirrors `crates/ipc/src/ring_band.rs`. Carried on a plan window
// and on a Job's context, so a client reads the band and does not hold the two lines.

/** Normal below 80, warning from 80, danger from 95. `UsageLevel` in `@armada/components`, value for value. */
export type RingBand = "normal" | "warning" | "danger";
