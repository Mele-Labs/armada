// What Fleet is holding disk for: the slots of each repository's worktree pool.
// `crates/ipc/src/holding.rs`.
//
// **Nothing says how large a worktree is, deliberately.** Bytes are not the
// decision. Which commits go, whether anything else has them, and which
// uncommitted files exist nowhere but that checkout is.

/** What was committed to a slot's branch so that freeing the slot loses nothing. Nothing is pushed. */
export type ReclaimedSaved = { commit: string; files: string[] };

export type WorktreesHeld = {
  /**
   * Every slot of each served repository's worktree pool, as `armada worktree
   * --status` reads it. Absent where Fleet serves no repository. Since 23.16.
   */
  slots?: WorktreeSlot[];
};

/** One slot of a repository's worktree pool. */
export type WorktreeSlot = {
  /** The Manifest whose repository the pool belongs to. */
  manifest_id: string;
  slot: number;
  path: string;
  held: SlotHolding;
  /** The branch `behind` is counted against. */
  base: string;
  /** The branch it is on. Absent for a free slot, which sits detached. */
  branch?: string;
  /** When its holder took it, where that was recorded. */
  since?: string;
  /** Every `setup.seed.paths` entry is a directory in the slot. */
  warm: boolean;
  /** Commits on the base its checkout does not have. Absent where git could not count them. */
  behind?: number;
  /** Commits on its checkout the base does not have. Absent where git could not count them, and from a Fleet before 30 Oct 2026. */
  ahead?: number;
  /**
   * A person closed it: no lease takes it until it is reopened, and a holder
   * keeps it until its lease ends. Since 23.17; absent from an older Fleet,
   * which closes nothing.
   */
  closed?: boolean;
  /** What a stranded slot holds, which a Scrap would lose. Since 23.29. */
  stranded?: SlotStranded;
  /** What a rescue Scout read of a stranded slot, while it reads and after. Since 23.29. */
  rescue?: SlotFinding;
};

/** The Session that held a slot before it stranded. `open` is false once it is closed or ended. */
export type SlotLastHolder = {
  kind: "session";
  id: string;
  title?: string;
  open: boolean;
};

/** The work a stranded slot holds. */
export type SlotStranded = {
  /** Every path `git status` reports, untracked included. */
  uncommitted: string[];
  /** Commits the base does not have, newest first. */
  commits: SlotCommit[];
  /** Of those, how many are on neither the remote nor the base. */
  unpushed: number;
  /** Who held the slot last. Absent where Fleet did not record one. */
  last_holder?: SlotLastHolder;
  /**
   * When Fleet told the last holder to commit, push and release, once. Absent
   * until it has, and where there was nobody to tell.
   */
  told?: string;
};

/** One commit on a stranded slot's branch. */
export type SlotCommit = {
  sha: string;
  subject: string;
  /** Whether it exists anywhere but this slot. Since 23.29. */
  home: CommitHome;
};

/**
 * Where else a commit on a stranded slot exists: `only_here` on no remote branch
 * and not on the local base, `on_remote` on a remote branch, `on_main` on the
 * local base.
 */
export type CommitHome = "only_here" | "on_remote" | "on_main";

/** Where a rescue Scout is. */
export type SlotFindingState = "reading" | "answered" | "stopped" | "failed";

/** What a rescue Scout concluded: `unfinished` has a part left to do, `scraps` needs no more work. */
export type SlotVerdict = "unfinished" | "scraps";

/** What a rescue Scout read of a stranded slot, kept against the slot. Since 23.29. */
export type SlotFinding = {
  state: SlotFindingState;
  /** The commit the slot was at when the Scout read it. */
  commit: string;
  /** Whether uncommitted changes were on top of it. */
  uncommitted: boolean;
  /** Characters of the change dropped before the Scout was handed it. */
  cut?: number;
  read: string[];
  searched: string[];
  /**
   * Whether the work has a part left to do, or is leftovers. Absent until the
   * Scout answers in the shape asked for. Since 23.29.
   */
  verdict?: SlotVerdict;
  /**
   * Under `unfinished`, what is left to do, a line each. Under `scraps`, one
   * line saying what the leftovers are. Since 23.29.
   */
  items?: string[];
  /** What it said last, where that was not the shape asked for. */
  summary?: string;
  /** Why it failed, where it did. */
  why?: string;
  cost_micros?: number;
};

/** What a person does with a stranded slot. Since 23.29. */
export type RescueAct = "start" | "stop" | "scrap" | "stash" | "pick_up";

/** `rescue_slot`'s body, `POST /worktrees/slots/rescue?manifest_id=`. */
export type RescueSlot = { act: RescueAct; slot: number };

/** What `rescue_slot` did. */
export type SlotRescued = {
  manifest_id: string;
  slot: number;
  /** The branch the slot was on, for a Scrap or a Stash. */
  branch?: string;
  /** A Scrap kept the branch, because it holds commits nothing else has. */
  branch_kept?: boolean;
  /** The commit a Stash made of the uncommitted work. */
  committed?: string;
};

/** What a person does to the pool from Cleanup's bay grid. Since 23.17. */
export type SlotAct = "add" | "remove" | "close" | "open" | "release";

/**
 * `change_slot_pool`'s body, `POST /worktrees/slots?manifest_id=`. `slot` names
 * the slot for all but `add`, which picks its own. On this machine only.
 */
export type ChangeSlotPool = {
  act: SlotAct;
  slot?: number;
  /** For `release`: the holder the person was shown, as the slot named it. Since 23.37. */
  holder?: string;
};

/** The slot `change_slot_pool` changed: the new one, for `add`. */
export type SlotPoolChanged = {
  manifest_id: string;
  slot: number;
  /** What a `release` did to the branch. Absent for every other act. Since 23.37. */
  released?: SlotReleased | null;
};

/** The branch a released slot's work is on, and what was committed to it. */
export type SlotReleased = { branch: string; saved?: ReclaimedSaved | null };

/**
 * Who holds a slot, or why nothing can. Discriminated on `state`, and matched
 * rather than rendered, so widening it is a major bump.
 */
export type SlotHolding =
  /** Never made. The next lease makes it. */
  | { state: "unmade" }
  /** A directory that is not a checkout. Nothing leases it until a person removes it. */
  | { state: "not_a_checkout" }
  /** A take or a release is under way. */
  | { state: "busy" }
  | { state: "free" }
  /** A process outside Fleet, as `ps` names it: `zsh (pid 4120)`. */
  | {
      state: "session";
      holder: string;
      /** The hosted Session whose agent, or something it started, this process is. Absent for a terminal Fleet does not host. */
      session_id?: string;
      session_title?: string;
    }
  /** A Session Fleet hosts, holding the slot under its own id. */
  | { state: "hosted_session"; session_id: string; session_title?: string }
  /** Its holder is gone and it still holds work. */
  | { state: "stranded"; why: string };

/** A bay an agent session holds, from a terminal or hosted by Armada: both are released the same way. */
export function releasable(slot: WorktreeSlot): boolean {
  return slot.held.state === "session" || slot.held.state === "hosted_session";
}

/** The holder a release names: the terminal's process as the pool read it, or a hosted Session's id. */
export function releaseHolder(slot: WorktreeSlot): string | undefined {
  if (slot.held.state === "session") return slot.held.holder;
  return slot.held.state === "hosted_session" ? slot.held.session_id : undefined;
}
