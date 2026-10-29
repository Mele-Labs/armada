// A Session's plan: what the agent records after planning with the person, what he approves and
// marks up, and how far Fleet has seen it carried out. `crates/ipc/src/session_plan.rs`,
// `docs/concepts/session.md` (*The plan*). Since 16 Oct 2026.
//
// The header rules in `protocol.ts` hold here. The plan is the agent's statement and progress is
// Fleet's observation: a task's state is read off the agent's task list and the edits Fleet saw under
// its group's paths.

import type { AdversarialReview } from "./adversarial-review";

/** Where one task stands. `handed_in` is done by the agent's word with no edit seen under its group's paths. */
export type PlanTaskState = "open" | "working" | "handed_in" | "done" | "failed";

/**
 * One task of the plan. `seen`: an edit was seen under its group's paths. Absent when none was. `marked`: Fleet set the
 * state itself, and this is its proof ("commit 1a2b3c4 names it"); absent where the agent's own list set it. `stale`: in
 * progress, and the work looks to be elsewhere; absent while it is not. `changed_at`: when it was put in progress or
 * completed, by the agent's list or Fleet's mark; absent while it is open.
 */
export type SessionPlanTask = {
  id: string;
  text: string;
  state: PlanTaskState;
  seen?: boolean;
  marked?: string;
  stale?: TaskStale;
  changed_at?: string;
};

/** Why a task in progress looks left behind (`why`, one line), and `since` when it was last touched. */
export type TaskStale = { why: string; since: string };

/** One group of the plan: the tasks a single subagent takes. */
export type SessionPlanGroup = {
  id: string;
  /** The type of subagent that takes it, free text such as `rust-engineer`. */
  agent: string;
  title: string;
  /** From the repository's root. */
  paths: string[];
  tasks: SessionPlanTask[];
  /** The issue the group works: the forge's number as digits (`7`), or a tracker's key (`ABC-12`). Absent where it names none. */
  issue?: string;
  /** One line of what the group gives the person. Absent where the agent wrote none. */
  gives?: string;
  /** When Fleet last saw an edit under one of the paths. Absent while it has seen none. */
  worked_at?: string;
  /** The pull request the group lands in. Always present: the label is `PR 1` where the agent named none. */
  lands_in: PlanLandsIn;
};

/** Where a group lands. Every group that shares a `label` reads the same `pull_request`. */
export type PlanLandsIn = {
  label: string;
  /** The pull request bound to the label, as the ledger's `pr` row last read it. Absent until one is. */
  pull_request?: PlanLandsInPr;
};

export type PlanLandsInPr = { number: number; title: string; state: "open" | "draft" | "merged" | "closed" };

/** What the agent changed when it recorded the plan again. */
export type PlanChanges = {
  /** Ids of groups and tasks that are new. */
  added: string[];
  renamed: { id: string; from: string }[];
  moved: { id: string; from: string }[];
  /** `group` is the group it was in, or its own id for a group. */
  removed: { group: string; text: string }[];
};

/** The sketch drawn with the plan. Its scene is the Session's ledger artifact `id`. */
export type PlanSketch = { id: string; title: string };

/** A Session's plan as it stands. Absent on the record until the agent records one. */
export type SessionPlan = {
  version: number;
  /** A few plain lines in the agent's words of what the person gets. Empty for a plan recorded before there was one. */
  summary: string;
  recorded_at: string;
  /** A version recorded again is pending, and a pending plan is one of the Session's calls. */
  approved: boolean;
  approved_at?: string;
  /**
   * When the person put this version away to keep talking. It holds nothing and calls for nothing
   * until the agent records a new version, which starts without it.
   */
  set_aside_at?: string;
  /** Every path a group names, once each. */
  scope: string[];
  groups: SessionPlanGroup[];
  /** Against the version before. Absent for the first. */
  changes?: PlanChanges;
  sketch?: PlanSketch;
  /** The windows that keep a pending plan from being put to the person. Absent when none does. */
  held_by?: PlanHold[];
  /** The latest adversarial review of this plan, whatever version it read. Out of date when its `subject.version` is not `version`. */
  adversarial?: AdversarialReview;
};

/** A window that holds a pending plan back, and why: what settles it. */
export type PlanHold = { url: string; why: "unapproved" | "changing" | "changed" | "walk" };

/**
 * One change the person marks on the plan, against the version he read. Nothing changes the plan until
 * the agent records it again. `group` and `task` are ids from that version.
 */
export type PlanMark =
  | { kind: "rename"; group: string; task?: string; from: string; to: string }
  | { kind: "remove"; group: string; task?: string }
  | { kind: "add_task"; group: string; id: string; text: string }
  | { kind: "add_group"; id: string; text: string }
  | { kind: "move_task"; task: string; from: string; to: string }
  | { kind: "subagent"; group: string; from: string; to: string }
  | { kind: "note"; id: string; group: string; task?: string; text: string };

/** `POST /sessions/plan/approve`: the person approves the plan. `version` is the one he read. */
export type ApprovePlan = { session_id: string; version: number };

/**
 * `POST /sessions/plan/changes`: the marks go to the agent as one message. `approve` approves the plan
 * with them.
 */
export type SendPlanChanges = { session_id: string; version: number; marks: PlanMark[]; approve?: boolean };

/** One line of the changes the person sent. `verb`: Renamed, Removed, Added, Moved, Subagent or Note. */
export type PlanChangeLine = { verb: string; place: string; from?: string; text?: string };

/**
 * `POST /sessions/plan`, the agent's. Recording again replaces the plan as a new version. A task or
 * group keeps its `id` through a rename; one without is new. `sketch.scene` is `show_sketch`'s format.
 */
export type RecordPlan = {
  /** A few plain lines in the person's own words: what he gets. No file names or types. A plan without one is refused. */
  summary: string;
  groups: {
    id?: string;
    title: string;
    agent: string;
    paths?: string[];
    issue?: string;
    /** One line, in the person's own words, of what this group gives him. */
    gives?: string;
    /** A short label for the pull request the group lands in, such as `PR 1`, at most 24 characters. Groups that ship together share one. Absent is `PR 1`. */
    lands_in?: string;
    tasks: { id?: string; title: string }[];
  }[];
  sketch?: { title: string; scene: unknown };
  /** The page the plan depends on: it is put to the person after he approves that walk. */
  walk?: string;
  session_id?: string;
};

/** What was done to a file. `write` made it or replaced it whole. */
export type TouchAct = "write" | "edit" | "remove";

/** One file touch Fleet saw, for the Field. `by` is absent where the Session's own agent touched it. */
export type FileTouch = {
  by?: { agent: string; id: string };
  path: string;
  act: TouchAct;
  at: string;
};
