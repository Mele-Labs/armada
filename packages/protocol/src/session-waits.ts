// A Session that waits on another and starts on its own. The mirror of `crates/ipc/src/session_waits.rs`.
// `docs/concepts/session.md`, *A Session that waits on another*.

/** What starts a waiting Session. `ends` is the default. */
export type StartCondition =
  | { kind: "ends" }
  | { kind: "group_done"; group: string }
  | { kind: "task_done"; task: string }
  | { kind: "pr_pushed" }
  | { kind: "pr_lands" };

/** Where a waiting Session starts from: the default branch, or the branch of the pull request of the Session waited on. */
export type StartBase = "main" | "branch";

/** Who last set a wait's condition. */
export type WaitBy = "you" | "agent";

/**
 * On `start_session` and `start_another_session`: the Session the new one waits on. With it `request` is the brief and
 * nothing is sent; the Session holds no slot and no process until the condition is met.
 */
export type StartAfter = {
  session: string;
  /** Absent is `ends`. */
  condition?: StartCondition;
  /** Absent is `main`. */
  base?: StartBase;
  /** What the new Session needs of the one it waits on, read from the brief. The Session waited on is told. */
  needs?: string;
};

/** The pull request a `pr_pushed` or `pr_lands` wait is on, once the Session waited on has one. */
export type WaitPull = { number: number; branch: string };

/** On a `SessionRecord` while it waits: absent once it has started. */
export type SessionWait = {
  on: { id: string; title: string };
  condition: StartCondition;
  base: StartBase;
  needs?: string;
  /** The words it was dispatched with, which it starts on. */
  brief: string;
  pull?: WaitPull;
  /** The title of the group or task a `group_done` or `task_done` condition names, as the plan of the Session waited on has it now. */
  part_title?: string;
  by: WaitBy;
};

/** `POST /sessions/read_wait`: the proposer reads a brief that tags `session_id` for the wait it names. Answers `WaitReading`. */
export type ReadSessionWait = { session_id: string; request: string };

/**
 * What the proposer read. A reading that could not be had is `ends`, from `main`, asking nothing. `plain` is set where the brief
 * tags the Session and names no condition: the new Session does not wait on it, and the tag is a reference.
 */
export type WaitReading = { condition: StartCondition; base: StartBase; needs?: string; plain?: boolean };

/**
 * `POST /sessions/change_waiting`: when a waiting Session starts, or where from. The agent of the Session waited on
 * changes a Session that waits on it; Bridge's call changes any. Answers the `SessionWait` as it now stands.
 */
export type ChangeWaiting = { waiting_session: string; condition?: StartCondition; base?: StartBase };

/** `POST /sessions/start_waiting`: start a waiting Session at once, whatever its condition. Answers `SessionRecord`. */
export type StartWaitingNow = { session_id: string };

/** What a `waiter` row says: a Session was dispatched to wait on this one, the condition changed, or it started. */
export type WaiterEvent = "waits" | "changed" | "started";

/** Which thread a `waiter` row is in: that of the Session waited on, or that of the waiting Session. Absent is `waited`. */
export type WaiterSide = "waited" | "waiting";
