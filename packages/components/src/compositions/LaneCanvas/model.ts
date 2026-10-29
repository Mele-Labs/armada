// What the lanes draw: a workflow's steps in a top lane, then a lane for each kind of thing a Job touches (a ticket, a pull request, a
// deploy), each a small state machine, with rules as edges between them. **A mock's model**: no Fleet serves a Workspace, a lifecycle or a
// rule yet (`docs/design`, Workspaces, lifecycles and rules). The editor draws the definitions; a Job's Lanes view draws the same lanes
// with one lane for each instance and the rules that fired.

/** What a lifecycle is for. Fleet reads each state from a tracker, the forge or a watch. */
export type LifecycleKind = "ticket" | "pull_request" | "deploy";

/** The lane a node sits in: the workflow, or a lifecycle's own (in the editor) or an instance's (on a Job). */
export type WorkflowLane = "workflow";

/** A place in a lane: the lane, and the step or state in it. */
export type Ref = { lane: WorkflowLane | LifecycleKind; item: string };

/** The id a node is drawn under, and the one a selection names. */
export const nodeId = (lane: string, item: string): string => `${lane}/${item}`;

/** The tracker's, forge's or watch's own words that count as being in a state, for the Workspace or for one repository. */
export type StateReads = { source: string; values: readonly string[]; scope: string };

export type LifecycleState = {
  id: string;
  label: string;
  /** The column it is drawn in, shared with the workflow's steps so time reads across the lanes. */
  col: number;
  reads: readonly StateReads[];
  /** Nothing follows it. */
  end?: true;
};

export type Lifecycle = {
  id: LifecycleKind;
  label: string;
  /** Where Fleet reads its states from. */
  from: string;
  states: readonly LifecycleState[];
  /** The ways a state is left, as pairs of state ids. */
  next: readonly (readonly [string, string])[];
};

/** Who may change a piece: Armada's, the person's own, or the repository's. */
export type Kept = "armada" | "yours" | "repository";

export type Autonomy = "acts" | "asks" | "off";

/** Kinds of act a rule does. A write to the tracker or the forge and a message that speaks as the person default to Asks. */
export type ActKind = "tracker" | "forge" | "message" | "job" | "watch" | "tell" | "command" | "agent";

export type Rule = {
  id: string;
  /** The edge's short label. */
  label: string;
  /** The node whose entering fires it, or a step starting or ending. */
  from: Ref;
  at?: "starts" | "ends";
  /** Where the edge lands: the state it moves something to, or the step it hands something to. */
  to: Ref;
  /** Conditions over linked instances and the Job, each a line. */
  ifs: readonly string[];
  does: readonly { act: ActKind; text: string }[];
  autonomy: Autonomy;
  kept: Kept;
  /** `workspace`, or the id of the one repository it applies in. */
  scope: string;
};

/** One step of the workflow in a lane, as the page lists it. */
export type LaneStep = { id: string; label: string; col: number; /** A lifecycle state that ends it, so the step waits on it. */ endsOn?: { lane: LifecycleKind; state: string } };

export type Repository = { id: string; name: string };

/** A named set of repositories that build one product, and what applies across them. */
export type Workspace = {
  id: string;
  name: string;
  repositories: readonly Repository[];
  /** The columns and ends-on of the steps of the workflows this Workspace draws, by step id. */
  steps: Readonly<Record<string, Omit<LaneStep, "id" | "label">>>;
  lifecycles: readonly Lifecycle[];
  rules: readonly Rule[];
};

// ---- What a canvas draws -------------------------------------------------------------------------------------------------

export type LaneNodeView = {
  id: string;
  label: string;
  /** A line under the label, mono and quiet: what ends a step. */
  sub?: string;
  col: number;
  role: "step" | "state";
  /** The state an instance stands in, or the step a Job is at. */
  lit?: boolean;
  /** An earlier state the instance went through. */
  passed?: boolean;
  /** A step that waits on a lifecycle state, and the words for it. */
  waiting?: string;
  end?: boolean;
  skipped?: boolean;
  /** Who the instance in this state is assigned to, drawn on the node. */
  who?: string;
  /** What is known of the instance at this state, one line each in the node's tooltip: the reviewers on a pull request. */
  facts?: readonly string[];
  /** The Job's own copy changed here, and how. */
  changed?: string;
};

export type LaneView = {
  id: string;
  kind: "workflow" | LifecycleKind;
  title: string;
  /** Under the title, quiet: where its states are read from. */
  source: string;
  /** The tooltip of the title: a ticket's own title. */
  note?: string;
  nodes: readonly LaneNodeView[];
  /** The ways a state is left within the lane. */
  moves: readonly (readonly [string, string])[];
};

export type RuleView = {
  id: string;
  /** The rule this edge is drawn for. Several edges can be one rule's: one for each ticket. */
  rule: string;
  from: string;
  to: string;
  label: string;
  /** A second line: when it fired, or what it waits for. */
  meta?: string;
  autonomy: Autonomy;
  /** On a Job: fired and acted, fired and asked, or still waiting. */
  status?: "acted" | "asked" | "waiting";
  /** Added by a change waiting on Approve. */
  proposed?: true;
  /** The same rule's second edge: drawn with its mark and no words. */
  quiet?: true;
  /** What the rule did, as the chip's tooltip says it: `Assigned to you, moved to In progress`. Absent reads the status. */
  did?: string;
};

// ---- A Job's lanes -------------------------------------------------------------------------------------------------------

/** One thing a Job holds: a ticket, its pull request, a deploy. */
export type Instance = {
  id: string;
  lifecycle: LifecycleKind;
  /** What the lane and the rail call it: `ARM-41`, `PR #812`, `Deploy`. */
  label: string;
  /** A ticket's own title. */
  title?: string;
  state: string;
  /** The states it went through, in order, the current one last. */
  visited: readonly string[];
  assignee?: string;
  /** What is known of it at a state, by state id: `Maya, Jun` at Reviewers on. */
  says?: Readonly<Record<string, string>>;
};

/** A rule that fired for an instance, and how it ended. */
export type Fired = { rule: string; instance: string; outcome: "acted" | "asked"; at: string; /** What it did, for the chip's tooltip. */ did?: string };

/** A change made to this Job's own copy, marked on the state of the lane it changes: `lane` is the instance and `item` its state. */
export type JobChange = { id: string; lane: string; item: string; text: string; at: string };

/** What the agent made of a human comment on the pull request. */
export type CommentRead = "change" | "answer" | "skip";

export type TriagedComment = { id: string; author: string; where: string; text: string; read: CommentRead; sent?: true };

/** What a Job holds of the Workspace: its copy of the definitions, frozen when it started, and everything standing on them. */
export type JobLanes = {
  workspace: string;
  copy: { steps: readonly LaneStep[]; lifecycles: readonly Lifecycle[]; rules: readonly Rule[] };
  instances: readonly Instance[];
  fired: readonly Fired[];
  changes: readonly JobChange[];
  /** The step that is waiting on an instance reaching a state, and which. */
  waitingOn?: { step: string; instance: string; state: string };
  /** The comments on the pull request while it is in review. */
  triage?: { instance: string; comments: readonly TriagedComment[] };
};

// ---- Words ---------------------------------------------------------------------------------------------------------------

/** The label of a state or a step in the lifecycles and steps given. */
export function labelOf(ref: Ref, lifecycles: readonly Lifecycle[], steps: readonly { id: string; label: string }[]): string {
  if (ref.lane === "workflow") return steps.find((one) => one.id === ref.item)?.label ?? ref.item;
  return lifecycles.find((one) => one.id === ref.lane)?.states.find((one) => one.id === ref.item)?.label ?? ref.item;
}

/** What fires a rule, as a line: `Pull request enters Draft`, `Plan starts`. */
export function whenSaid(rule: Rule, lifecycles: readonly Lifecycle[], steps: readonly { id: string; label: string }[]): string {
  const name = labelOf(rule.from, lifecycles, steps);
  if (rule.from.lane === "workflow") return `${name} ${rule.at === "ends" ? "ends" : "starts"}`;
  const lifecycle = lifecycles.find((one) => one.id === rule.from.lane);
  return `${lifecycle?.label ?? rule.from.lane} enters ${name}`;
}
