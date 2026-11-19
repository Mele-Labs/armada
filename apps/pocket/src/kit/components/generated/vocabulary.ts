// Copied from Bridge Classic's generated registry and trimmed to what Pocket draws. Pocket owns this copy and does not regenerate it.
//
// The verb, the glyph and the status token each variant renders as, from
// `crates/core-model/domain/enum-verbs.toml`; whether a Check outcome
// advances a step, from `check-outcomes.toml`. Nothing here is written by
// hand, which is the point: a state label typed into a component is a
// second vocabulary.
//
// A `null` verb, glyph or token is a gap in the registry, not a default. Every
// one is listed in `GAPS` so a surface can say what it could not render instead
// of inventing copy for it.

import { CircleDashed, CircleDot, Check, X, Minus, type LucideIcon } from "lucide-react";

/** How one variant reads. `null` where the registry carries no answer. */
export type Rendering = {
  readonly verb: string | null;
  readonly icon: LucideIcon | null;
  /** Badge's `status` prop: the token stem, without its `--status-` prefix. */
  readonly badgeStatus: string | null;
  readonly statusToken: string | null;
  /**
   * The sentence a tooltip reads for this variant, where the verb alone
   * gives no reasoning. `null` where the registry carries none — most
   * tables do not, and a surface renders no tooltip rather than one with
   * an empty label.
   */
  readonly hint: string | null;
};

/** `task_state`, keyed by the wire value. */
export const TASK_STATE: Readonly<Record<string, Rendering | undefined>> = {
  open: {
    verb: "open",
    icon: CircleDashed,
    badgeStatus: "not-started",
    statusToken: "--status-not-started",
    hint: null,
  },
  working: {
    verb: "working",
    icon: CircleDot,
    badgeStatus: "running",
    statusToken: "--status-running",
    hint: null,
  },
  handed_in: {
    verb: "done, no edit seen",
    icon: Check,
    badgeStatus: "completed-success",
    statusToken: "--status-completed-success",
    hint: null,
  },
  done: {
    verb: "done",
    icon: Check,
    badgeStatus: "completed-success",
    statusToken: "--status-completed-success",
    hint: null,
  },
  failed: {
    verb: "failed",
    icon: X,
    badgeStatus: "completed-failed",
    statusToken: "--status-completed-failed",
    hint: null,
  },
  dropped: {
    verb: "dropped",
    icon: Minus,
    badgeStatus: "killed",
    statusToken: "--status-killed",
    hint: null,
  },
};
