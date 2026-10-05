// What Fleet is holding disk for, and the test each one did not pass.
// `crates/ipc/src/holding.rs`.
//
// **The reason is the payload, not a label on it.** Not-provably-safe is one
// word for several situations a person answers differently — a branch the base
// cannot reach, files nobody committed, a job still moving, a job something
// else is waiting on — so each arm carries what its own decision needs. A count
// of commits without the branch they are on, or a claim of uncommitted work
// without the filenames, is a row that asks somebody to guess.
//
// **A piloted job's checkout is not on this wire.** Fleet drops it before the
// answer is built, so there is no arm below for it and nothing here could draw
// one by mistake. `#367` is the reason: a person is at an unrestricted toolset
// in that directory.
//
// **Nothing says how large a worktree is, deliberately.** Bytes are not the
// decision. Which commits go, whether anything else has them, and which
// uncommitted files exist nowhere but that checkout is.

/**
 * Every worktree Fleet is holding disk for, ordered by job id.
 *
 * **Complete, including the ones the sweep is about to take on its own.** A
 * list filtered to the held ones would be a list the next sweep changes, and a
 * person who came looking for a worktree that is not on it could not tell
 * "already given back" from "held and not said".
 */
export type WorktreesHeld = {
  worktrees: WorktreeHeld[];
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
  /**
   * A person closed it: no lease takes it until it is reopened, and a holder
   * keeps it until its lease ends. Since 23.17; absent from an older Fleet,
   * which closes nothing.
   */
  closed?: boolean;
  /** What a stranded slot holds, which a Scrap would lose. Since 23.28. */
  stranded?: SlotStranded;
  /** What a rescue Scout read of a stranded slot, while it reads and after. Since 23.28. */
  rescue?: SlotFinding;
};

/** The work a stranded slot holds. */
export type SlotStranded = {
  /** Every path `git status` reports, untracked included. */
  uncommitted: string[];
  /** Commits the base does not have, newest first. */
  commits: SlotCommit[];
  /** Of those, how many are on neither the remote nor the base. */
  unpushed: number;
};

/** One commit on a stranded slot's branch. */
export type SlotCommit = {
  sha: string;
  subject: string;
  /** Whether it exists anywhere but this slot. Since 23.28. */
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

/** What a rescue Scout read of a stranded slot, kept against the slot. Since 23.28. */
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
   * Scout answers in the shape asked for. Since 23.28.
   */
  verdict?: SlotVerdict;
  /**
   * Under `unfinished`, what is left to do, a line each. Under `scraps`, one
   * line saying what the leftovers are. Since 23.28.
   */
  items?: string[];
  /** What it said last, where that was not the shape asked for. */
  summary?: string;
  /** Why it failed, where it did. */
  why?: string;
  cost_micros?: number;
};

/** What a person does with a stranded slot. Since 23.28. */
export type RescueAct = "start" | "stop" | "scrap" | "stash";

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
export type SlotAct = "add" | "remove" | "close" | "open";

/**
 * `change_slot_pool`'s body, `POST /worktrees/slots?manifest_id=`. `slot` names
 * the slot for all but `add`, which picks its own. On this machine only.
 */
export type ChangeSlotPool = { act: SlotAct; slot?: number };

/** The slot `change_slot_pool` changed: the new one, for `add`. */
export type SlotPoolChanged = { manifest_id: string; slot: number };

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
  /** One of Fleet's jobs. The title is absent where the store no longer has the job. */
  | {
      state: "job";
      job_id: string;
      job_title?: string;
      /** Where the Job ended, for a Job that has. Since 23.28. */
      job_status?: string;
      /** Why the Job's release was refused after it ended: its work is still in the slot. Since 23.28. */
      kept?: string;
      /** The Job completed and holds the slot until a person clears it. Since 23.28. */
      completed?: boolean;
    }
  /** A process outside Fleet, as `ps` names it: `zsh (pid 4120)`. */
  | { state: "session"; holder: string }
  /** Its holder is gone and it still holds work. */
  | { state: "stranded"; why: string };

/** One job's worktree, and every test it failed. */
export type WorktreeHeld = {
  job_id: string;
  job_title: string;
  status: string;
  /**
   * When armada last moved anything on this job.
   *
   * **Not when the files in the checkout were last written**, and nothing on
   * this seam can be: the dirty reading answers names and not times. It is a
   * floor — a checkout whose job stopped four days ago has been sitting at
   * least that long — and it is here because it is read against `uncommitted`,
   * the one reason where reclaiming ends something.
   */
  last_moved_at: string;
  /** The checkout on disk — what a person goes and looks at. */
  path: string;
  /**
   * Whether the checkout is still there. False on a job whose checkout is gone
   * and whose branch is still held, which `delete_branch` is for. Since 13.32.
   */
  on_disk: boolean;
  /** The branch the job derived, named even where it is already gone. */
  branch: string;
  /**
   * **Empty is the whole of the safety claim.** Fleet's own sweep takes
   * exactly the empty ones, so a row with nothing here is a row nobody has to
   * decide about.
   */
  held: HeldReason[];
};

/**
 * One test a worktree did not pass.
 *
 * Discriminated on `why`, and every arm is matched rather than rendered — which
 * is what makes a reason added to this set a **major** protocol bump rather
 * than a minor one. `docs/practices/protocol.md` has the row.
 */
export type HeldReason =
  /**
   * The job is still moving, so it may still need its worktree. **Nothing may
   * reclaim this one** — Fleet refuses a status that is not terminal, so a
   * surface offering it would be offering a 409.
   */
  | { why: "not_terminal"; status: string }
  /**
   * The branch holds commits the base cannot reach.
   *
   * **Reclaiming does not destroy them.** There is no force on this seam, so
   * the checkout goes and the branch stays exactly where it is — which is why
   * the tip travels: it is what the work is reachable from afterwards.
   */
  | { why: "unmerged"; base: string; commits: number; tip: string }
  /** Nothing could say what the branch would be merged into, so it is kept. */
  | { why: "base_unanswered"; detail: string }
  /**
   * Files written and committed nowhere.
   *
   * **The one reason where reclaiming destroys something.** No branch carries
   * these, so removing the directory is the end of them — which is why they are
   * named file by file and why the confirmation reads them out.
   */
  | { why: "uncommitted"; files: string[] }
  /** Somebody locked the checkout, which is a person saying not yet. */
  | { why: "locked"; reason: string }
  /** A job that depends on this one has not finished. */
  | { why: "depended_on"; by: string[] }
  /** Version control would not say what is in the checkout. */
  | { why: "unreadable"; detail: string };

/** Whether Fleet will give this one back without being asked. */
export function provablySafe(held: WorktreeHeld): boolean {
  return held.held.length === 0;
}

/**
 * Whether a person may choose this one.
 *
 * **A job that is not terminal is drawn and not offered.** Fleet refuses the
 * act with a 409 while a drone might still be writing, so a checkbox on that
 * row would be a control whose only outcome is a refusal — and #385's own table
 * says what a person decides about one: leave it alone.
 */
export function reclaimable(held: WorktreeHeld): boolean {
  return !held.held.some((reason) => reason.why === "not_terminal");
}
