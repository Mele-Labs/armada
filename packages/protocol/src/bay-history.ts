// What happened to one slot of a repository's pool: who held it, and what was checked out in it —
// `crates/ipc/src/bay_history.rs`, for the side panel's Bay.
//
// Read off `get_slot_history` (`GET /slot_history?manifest_id=&slot=`). The header rules in
// `protocol.ts` hold here: hand-written, and the sets Fleet closes are written out.

/** What held a slot. `agent` is a process that took the lease itself, with `armada worktree lease`. */
export type BayHolderKind = "session" | "agent";

/** Who held the slot. */
export type BayHolder = {
  kind: BayHolderKind;
  /** The Session's id. For an `agent`, the id of the agent Session its lease was taken for, else the process's pid. */
  id: string;
  /** What it is called now, where Fleet still has it. */
  title?: string;
};

/**
 * One thing that happened to the slot, tagged by `kind`. A `released` after a lease taken from a
 * holder that had gone is dated when the next lease took the slot.
 */
export type BayHistoryEntry =
  | { kind: "leased"; at: string; holder: BayHolder; branch?: string }
  | { kind: "released"; at: string; holder: BayHolder }
  | {
      kind: "checkout";
      at: string;
      /** A branch, or the commit a detached head stood at. Absent where the reflog names none. */
      from?: string;
      /** A branch, or the commit it was detached at. */
      to: string;
      /** The commit `HEAD` stood at once it moved. */
      sha: string;
    };

/** `get_slot_history`: the slot's entries of the last thirty days, newest first, at most 300. */
export type BayHistory = {
  manifest_id: string;
  slot: number;
  entries: BayHistoryEntry[];
  /** Older entries inside the thirty days were left out to keep to the limit. Absent when none were. */
  cut?: boolean;
};

/** What `get_slot_history` is asked. */
export type BayHistoryAsk = { manifest_id: string; slot: number };
