// A Job's lanes as a screen draws them, worked out by Fleet so a client does not. Carried on `SessionRecord.lanes`. Mirrors
// `crates/ipc/src/job_lanes.rs`. Two layers: what a Job holds and what its rules did (`job`), and the lanes and edges drawn from it
// (`lanes`, `rules`). A time is an instant, never a clock: Fleet knows no time zone, so the client words it.

/** What a node is: its shape and its tint. A kind is never a status. `start` and `end` are the small circles at the ends of the workflow. */
export type LaneNodeKind = "step" | "tracker" | "forge" | "watch" | "job" | "start" | "end";

/** Whether a node is a step of the workflow or a state of a lifecycle. */
export type LaneNodeRole = "step" | "state";

/** What reads a lifecycle's states. */
export type LaneLifecycleKind = "tracker" | "forge" | "watch" | "job";

/** Whether a lane is the workflow's or an instance's. */
export type LaneKind = "workflow" | "lifecycle";

/** How far a rule goes on its own. */
export type LaneAutonomy = "acts" | "asks" | "off";

/** Whose rule it is: Armada's, the person's own (Kit's) or the repository's. */
export type LaneKept = "armada" | "yours" | "repository";

/** The moment a rule is for. Fleet sets `starts`, `ends` and `enters`; `leaves` is never set. */
export type LaneMoment = "starts" | "ends" | "enters" | "leaves";

/** How a rule that fired ended: its acts done, waiting on the person, still to be done, or set aside. */
export type LaneOutcome = "acted" | "asked" | "waiting" | "skipped";

/** A ticket a started Job holds for the Job that started it. `state` and `state_id` are empty where it stands in no state; `state` then keeps the word the source said, if any. */
export type LaneCarriedTicket = {
  label: string;
  title?: string;
  state: string;
  state_id: string;
};

/** One state of a lifecycle as a Job draws it, with the column it stands in. */
export type LaneLifecycleState = {
  id: string;
  label: string;
  col: number;
  /** Absent where false. */
  end?: boolean;
};

/** A lifecycle the Job holds an instance of. */
export type LaneLifecycle = {
  id: string;
  label: string;
  /** Where its states are read from, in a word under the lane's title: `Tracker`, `Forge`, `Watch` or `Fleet`. */
  from: string;
  kind: LaneLifecycleKind;
  states: LaneLifecycleState[];
  /** The ways a state is left, as pairs of state ids. */
  next: [
    string,
    string,
  ][];
};

/** A step of the Job's workflow, in the column its order gives it. */
export type LaneStep = {
  id: string;
  label: string;
  col: number;
};

/** Either end of a rule's edge: a lane (`workflow` or a lifecycle's id) and the step or state in it. `$step` as the item is the step the Job was on when the rule fired, else the one it is on. */
export type LaneRef = {
  lane: string;
  item: string;
};

/** A rule as a Job's lanes draw it. A rule a Job event or a gate sets off is not drawn, so it is not here. */
export type LaneRule = {
  id: string;
  label: string;
  from: LaneRef;
  at: LaneMoment;
  to: LaneRef;
  autonomy: LaneAutonomy;
  kept: LaneKept;
};

/** One thing a Job holds: a ticket, its pull request, a deploy, a Job it started. */
export type LaneInstance = {
  /** `<lifecycle>:<key, percent-encoded>`: a key can hold a slash, which a node id cannot. */
  id: string;
  lifecycle: string;
  label: string;
  /** A ticket's own title; `Unmapped: <word>` for a status no state lists. */
  title?: string;
  /** For a Job this one started: the ticket it was started for, read from there. */
  ticket?: LaneCarriedTicket;
  /** Empty where it stands in no state. */
  state: string;
  /** The word its source used, for a status no state lists. */
  unmapped?: string;
  /** Ending the Job ended it, standing where it was. Absent where false. */
  ended?: boolean;
  /** The states it went through, in order, the current one last. */
  visited: string[];
};

/** One act of a rule as the Job's rules kept it. `standing` is Fleet's word: `planned`, `asked`, `stuck`, `done`, `skipped`, `told` or `not_carried`. */
export type LaneFiredAct = {
  text: string;
  /** `acts` or `asks`. */
  course: LaneAutonomy;
  standing: string;
  said?: string;
  /** When it was settled, else when it was planned. */
  at: string;
};

/** A rule that fired for an instance, and how it ended. */
export type LaneFired = {
  rule: string;
  /** The instance it was done to, by its lane id; empty where a step set it off and it handed to a step. */
  instance: string;
  outcome: LaneOutcome;
  /** When its last act was settled, else planned. */
  at: string;
  /** What it did, a line for each act. */
  did: string[];
  /** The id of the step the Job was on when it was planned. */
  step?: string;
  acts: LaneFiredAct[];
};

/** A change made to the Job's own copy, marked on what it changes. `lane` is an instance's id, `workflow`, `rule`, or a lifecycle's id with no `item`. */
export type LaneChange = {
  id: string;
  lane: string;
  item: string;
  text: string;
  at: string;
  /** Absent where false. */
  dropped?: boolean;
  /** A changed rule's name, from any rule the Job or its repository holds, drawn or not. */
  name?: string;
};

/** The definitions the Job's lanes are drawn from. */
export type LaneCopy = {
  steps: LaneStep[];
  lifecycles: LaneLifecycle[];
  rules: LaneRule[];
};

/** What a Job holds and what its rules did. */
export type JobLanesHeld = {
  copy: LaneCopy;
  instances: LaneInstance[];
  fired: LaneFired[];
  changes: LaneChange[];
  /** What the step the Job is on waits for, a sentence for each end that has not come. Absent when empty. */
  waiting?: string[];
};

/** A node of a lane: a step or a state. Every flag is absent where false. */
export type LaneNode = {
  /** `<lane id>/<item>`. */
  id: string;
  label: string;
  col: number;
  role: LaneNodeRole;
  kind: LaneNodeKind;
  /** The step the agent is working on. */
  lit?: boolean;
  /** Where an instance stands, or the step a Job is at with nobody working on it. */
  here?: boolean;
  /** An earlier state the instance went through, or the start of a workflow that has begun. */
  passed?: boolean;
  /** One sentence for each end the step has not had. */
  waiting?: string[];
  end?: boolean;
  skipped?: boolean;
  /** Where an instance stood when ending the Job ended it. It takes the place of `here`. */
  ended?: boolean;
  /** The ticket the Job on this lane was started for, drawn on the node the Job stands at. */
  ticket?: LaneCarriedTicket;
  /** The Job's own copy changed here, and how. */
  changed?: string;
  /** A workflow's editor only: the Workspaces and repositories, by name, that name or read this state otherwise, add it, or leave it out. */
  differs?: string[];
  /** A workflow's editor only: the Workspaces whose pieces tie for this repository, so it reads the default's: `Web and Mobile differ`. */
  clash?: string;
};

/** What a piece's mark waits on in a workflow's editor: a pull request that removes it (Fleet sets none), or the owner's Approve of a draft that keeps it. */
export type LanePending = "removal" | "keep";

/** A lane: the workflow's steps, or one instance's states. */
export type LaneView = {
  id: string;
  kind: LaneKind;
  title: string;
  /** Under the title, quiet: where its states are read from. */
  source: string;
  /** What kind of thing the lane is, in its lifecycle's own name. */
  what?: string;
  /** The tooltip of the title: a ticket's own title. */
  note?: string;
  /** The tracker's lifecycle: this lane is a ticket. Absent where false. */
  ticket?: boolean;
  /** Ending the Job ended this instance. Absent where false. */
  ended?: boolean;
  /** The Job's own copy of this lane's lifecycle changed, and how. */
  changed?: string;
  /** A workflow's editor only: what this lane's lifecycle waits on before it is kept or goes. */
  pending?: LanePending;
  nodes: LaneNode[];
  /** The ways a state is left within the lane, as pairs of node ids. */
  moves: [
    string,
    string,
  ][];
};

/** An edge: a rule that fired, or one still waiting, between two nodes. */
export type LaneEdge = {
  /** `<rule>:<source id>` and the target's after `>`; one rule has an edge for each instance. */
  id: string;
  /** The rule this edge is drawn for. */
  rule: string;
  from: string;
  to: string;
  label: string;
  /** What a rule still waiting waits for: `Plan starts`, `PR #812: Approved`. Absent once it fired. */
  waits?: string;
  /** When it fired. Absent while it waits. */
  fired_at?: string;
  autonomy: LaneAutonomy;
  status: LaneOutcome;
  /** What it did, a line for each act. Absent while it waits. */
  did?: string[];
  changed?: string;
  /** The same rule's second edge: drawn with its mark and no words. Absent where false. */
  quiet?: boolean;
  /** A workflow's editor only: the Workspaces and repositories, by name, where the rule applies alone or is changed. */
  differs?: string[];
  /** A workflow's editor only: the Workspaces whose rules tie for this repository: `Web and Mobile differ`. */
  clash?: string;
  /** A workflow's editor only: a rule a draft waiting on Approve would add, which no place keeps yet. Absent where false. */
  proposed?: boolean;
  /** A workflow's editor only: what this rule waits on before it is kept or goes. */
  pending?: LanePending;
};

/**
 * `SessionRecord.lanes`: where the Job stands in its workflow, what it holds, and the lanes and edges drawn from both. Absent for a Job that has
 * ended and for one that holds nothing and follows no workflow.
 *
 * Also `get_workflow_lanes` (`GET /workflows/lanes?workflow=&scope=&manifest_id=`): one workflow's lanes as its editor draws them, at the level
 * `scope` names (`default`, `workspace:<id>` or a repository's Manifest id). No Job stands in them, so `at` is -1, `job` holds only `copy` (the steps,
 * the tab's lifecycles and the rules drawn), and every edge is `waiting` with no `waits`. A `scope` the repository has no tab for is 422
 * `fleet.no_such_lanes_level`; a workflow the repository does not run is 422 `fleet.no_such_workflow_definition`.
 */
export type JobLanesView = {
  /** The workflow's name; empty for a Job on none. */
  workflow: string;
  /** The step the Job is on: -1 before a step starts, the step count once every one has ended. */
  at: number;
  /** The agent is working on the step it is on. Absent where false. */
  working?: boolean;
  /** The steps the person skipped, by index. Absent when none. */
  skipped?: number[];
  job: JobLanesHeld;
  lanes: LaneView[];
  rules: LaneEdge[];
};
