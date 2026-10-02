// The merge line: the branches waiting to land on main, and the ones that just
// left it. Draft, for the Job record Fleet serves on detail, which is where
// `docs/capabilities/merge-line.md` (*Where each part goes in Fleet*) puts the
// outcome file and `--status`.
//
// Source of truth today: `armada land --status`, read off disk by
// `crates/armada/src/land/status.rs` from the queue and each branch's outcome
// (`crates/armada/src/land/outcome.rs`). **Nothing on the wire carries it**, so
// there is no derivation from today's Fleet beside the type: a real Bridge is
// handed none and Overview draws no panel.

import type { MergeLineEntry } from "@armada/components";

/** One repository's line. */
export type MergeLineView = {
  /** In place order: `--status`'s numbered rows. */
  line: readonly MergeLineEntry[];
  /** Off the line with an outcome on disk, newest first. */
  off: readonly MergeLineEntry[];
};
