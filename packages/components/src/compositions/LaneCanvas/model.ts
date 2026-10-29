// What the lanes draw: a workflow's steps in a top lane, then a lane for each lifecycle the workflow uses, each a small state machine, with
// rules as edges between them. **A lifecycle is any lifecycle**: a ticket, a pull request, a deploy, a document. Fleet reads each of its states from
// a tracker, the forge, a watch or an Armada document, and nothing in the lanes names one. **A mock's model**: no Fleet serves a Workspace, a lifecycle
// or a rule yet (`docs/design`, Workspaces, lifecycles and rules). The editor draws the definitions; a Job's Lanes view draws the same lanes with
// one lane for each instance and the rules that fired.

/** The lane a node sits in: the workflow's own, or a lifecycle's id (in the editor) or an instance's (on a Job). */
export type WorkflowLane = "workflow";

/** A place in a lane: the lane, and the step or state in it. */
export type Ref = { lane: string; item: string };

/** The id a node is drawn under, and the one a selection names. */
export const nodeId = (lane: string, item: string): string => `${lane}/${item}`;

/** The tracker's, forge's or watch's own words that count as being in a state, for the Workspace or for one repository. */
export type StateReads = { source: string; values: readonly string[]; scope: string };

/** A state as the Workspace defines it: where Fleet reads it from is the lifecycle's, and the words that count as being in it are its own. */
export type StateDef = {
  id: string;
  label: string;
  reads: readonly StateReads[];
  /** Nothing follows it. */
  end?: true;
};

/** What reads a lifecycle's states, which is the shape its nodes are drawn in: the tracker or the forge (a pill), a watch (a hexagon), an Armada document (a pill) or a Job Armada started (a card). */
export type LifecycleKind = "tracker" | "forge" | "watch" | "document" | "job";

/** A lifecycle as the Workspace defines it, for every workflow that uses it. */
export type LifecycleDef = {
  id: string;
  label: string;
  /** Where Fleet reads its states from. */
  from: string;
  /** What reads them. Absent, a state is drawn as the tracker's. */
  kind?: LifecycleKind;
  states: readonly StateDef[];
  /** The ways a state is left, as pairs of state ids. */
  next: readonly (readonly [string, string])[];
};

/** A state as a workflow draws it: in a column shared with the workflow's steps, so time reads across the lanes. */
export type LifecycleState = StateDef & { col: number };

/** A lifecycle as a workflow draws it, or a Job holds it: the states the workflow places, each with its column. */
export type Lifecycle = Omit<LifecycleDef, "states"> & { states: readonly LifecycleState[] };

/** Who may change a piece: Armada's, the person's own, or the repository's. */
export type Kept = "armada" | "yours" | "repository";

export type Autonomy = "acts" | "asks" | "off";

/** Kinds of act a rule does. A write to the tracker or the forge and a message that speaks as the person default to Asks. */
export type ActKind = "tracker" | "forge" | "message" | "job" | "watch" | "tell" | "command" | "agent";

/** A repository that lets a rule do more or less than the Workspace does. */
export type RuleOverride = { scope: string; autonomy: Autonomy };

export type Rule = {
  id: string;
  /** The edge's short label. */
  label: string;
  /** The node whose entering fires it, or a step starting or ending. */
  from: Ref;
  /** The moment: a step starts or ends, a state is entered or left. Absent, a step starts and a state is entered. */
  at?: "starts" | "ends" | "enters" | "leaves";
  /** Where the edge lands: the state it moves something to, or the step it hands something to. */
  to: Ref;
  /** Conditions over linked instances and the Job, each a line. */
  ifs: readonly string[];
  does: readonly { act: ActKind; text: string }[];
  autonomy: Autonomy;
  kept: Kept;
  /** `workspace`, or the id of the one repository it applies in. */
  scope: string;
  /** Where a repository's autonomy differs from the one above. */
  overrides?: readonly RuleOverride[];
  /** The one workflow it belongs to. Absent, it applies in every workflow that has both ends. */
  workflow?: string;
};

/** What a step is ended by, or passed over for: a thing Fleet watches (`Observed`) or a lifecycle, and the state of it. */
export type Waits = { on: string; state: string };

/** One step of the workflow in a lane, as the page lists it. */
export type LaneStep = {
  id: string;
  label: string;
  col: number;
  /** A lifecycle state that ends it, so the step waits on it. */
  endsOn?: { lane: string; state: string };
  /** What the person chose to end it, where it is not the words Fleet read from the definition. */
  ends?: Waits;
  /** What the person chose to pass it over for. */
  passedOver?: Waits;
};

export type Repository = { id: string; name: string };

/** What one workflow draws of the Workspace: where its steps sit, and which states of which lifecycles it has a lane for, and where. */
export type WorkflowPlace = {
  /** The column and ends-on of each of its steps, by step id. */
  steps: Readonly<Record<string, Omit<LaneStep, "id" | "label">>>;
  /** The column of each state it draws, by `lifecycle/state`. **A lifecycle with no state placed has no lane here**, and a state with no column is not drawn. */
  states: Readonly<Record<string, number>>;
};

/**
 * Something Fleet watches that a step can wait on besides a lifecycle: the agent saying it is done, the pull request, the Jobs this Job starts. Each state is a thing it can be,
 * and `says` is the sentence Fleet's reading of a workflow ends the step with, so the step's words select it. A state that a lifecycle's also is names that one in `endsOn`.
 */
export type Observed = {
  id: string;
  label: string;
  /** The lifecycle whose states these are, so that lifecycle is not listed twice. */
  lifecycle?: string;
  states: readonly { id: string; label: string; says: string; endsOn?: { lane: string; state: string } }[];
};

/** A named set of repositories that build one product, and what applies across them. */
export type Workspace = {
  id: string;
  name: string;
  repositories: readonly Repository[];
  lifecycles: readonly LifecycleDef[];
  /** What else a step can wait on or be passed over for. */
  observes?: readonly Observed[];
  /** What each workflow draws of the lifecycles, by workflow id. */
  workflows: Readonly<Record<string, WorkflowPlace>>;
  rules: readonly Rule[];
};

/** The lifecycles a workflow draws, each with the states it places, in the Workspace's order. */
export function lifecyclesFor(workspace: Workspace, workflow: string): Lifecycle[] {
  const place = workspace.workflows[workflow]?.states ?? {};
  return workspace.lifecycles.flatMap((lifecycle): Lifecycle[] => {
    const states = lifecycle.states.flatMap((state): LifecycleState[] => {
      const col = place[`${lifecycle.id}/${state.id}`];
      return col === undefined ? [] : [{ ...state, col }];
    });
    const drawn = new Set(states.map((state) => state.id));
    return states.length === 0 ? [] : [{ ...lifecycle, states, next: lifecycle.next.filter(([from, to]) => drawn.has(from) && drawn.has(to)) }];
  });
}

/** The tracker values that differ in a repository, as repository ids: a state mapped to other words. */
export const stateDiffers = (state: StateDef): string[] => [...new Set(state.reads.filter((one) => one.scope !== "workspace").map((one) => one.scope))];

/** The repositories a rule differs in: where it applies alone, or where its autonomy is another. */
export const ruleDiffers = (rule: Rule): string[] => [...new Set([...(rule.scope === "workspace" ? [] : [rule.scope]), ...(rule.overrides ?? []).map((one) => one.scope)])];

// ---- What a canvas draws -------------------------------------------------------------------------------------------------

/**
 * What a node is, which is its shape and its tint (`LaneCanvas.css`): a step is a rounded box; a state read from the tracker or the forge, or from an Armada document, is a pill; a step that waits
 * and a state a watch reads are hexagons; a Job another Job started is a card with the drone glyph; and where the workflow starts and ends are small circles. **A kind is never a status**: its tint is a
 * faint ground from `kinds.css`, and what a node stands in (working, waiting, went through) is its frame and its mark.
 */
export type NodeKind = "step" | "wait" | "tracker" | "forge" | "document" | "watch" | "job" | "start" | "end";

/** Where the workflow starts and ends, drawn as small circles at the ends of its lane: not steps, so nothing picks them. */
export const START_ITEM = "$start";
export const END_ITEM = "$end";

export type LaneNodeView = {
  id: string;
  label: string;
  /** A line under the label, mono and quiet: what ends a step. */
  sub?: string;
  col: number;
  role: "step" | "state";
  kind: NodeKind;
  /** The step the agent is working on. It takes the running hue and the working glyph, and nothing else on the lanes does. */
  lit?: boolean;
  /** Where an instance stands, or the step a Job is at with nobody working on it: a place and not an act, drawn static in no hue. */
  here?: boolean;
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
  /** The repositories this differs in, by name: a state mapped to other tracker values. */
  differs?: readonly string[];
};

export type LaneView = {
  id: string;
  kind: "workflow" | "lifecycle";
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
  /** What an asked rule left for the person to answer, by the id of its card: pressing the rule opens it. */
  needs?: string;
  /** The repositories this differs in, by name: where it applies alone, or where its autonomy is another. */
  differs?: readonly string[];
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
  /** The id of the lifecycle it is an instance of. */
  lifecycle: string;
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
export type Fired = {
  rule: string;
  instance: string;
  outcome: "acted" | "asked";
  at: string;
  /** What it did, for the chip's tooltip. */
  did?: string;
  /** The id of the card an asked rule left in the thread, while it waits to be answered. */
  needs?: string;
};

/** A change made to this Job's own copy, marked on the node it changes: `lane` is the instance and `item` its state, or `workflow` and `item` the step added or skipped. */
export type JobChange = { id: string; lane: string; item: string; text: string; at: string };

/** What the agent made of a human comment left on an instance. */
export type CommentRead = "change" | "answer" | "skip";

export type TriagedComment = { id: string; author: string; where: string; text: string; read: CommentRead; sent?: true };

/** What a Job holds of the Workspace: its copy of the definitions, frozen when it started, and everything standing on them. */
export type JobLanes = {
  workspace: string;
  copy: { steps: readonly LaneStep[]; lifecycles: readonly Lifecycle[]; rules: readonly Rule[] };
  instances: readonly Instance[];
  fired: readonly Fired[];
  changes: readonly JobChange[];
  /** The step that is waiting on an instance reaching a state, and which. `and` are the other instances it waits on as well: the step names them all and not their state. */
  waitingOn?: { step: string; instance: string; state: string; and?: readonly string[] };
  /** The comments left on an instance while it is in review. */
  triage?: { instance: string; comments: readonly TriagedComment[] };
};

// ---- Words ---------------------------------------------------------------------------------------------------------------

/** Enough of a lifecycle to name its states. */
type Named = { id: string; label: string; states: readonly { id: string; label: string }[] };

/** The label of a state or a step in the lifecycles and steps given. */
export function labelOf(ref: Ref, lifecycles: readonly Named[], steps: readonly { id: string; label: string }[]): string {
  if (ref.lane === "workflow") return steps.find((one) => one.id === ref.item)?.label ?? ref.item;
  return lifecycles.find((one) => one.id === ref.lane)?.states.find((one) => one.id === ref.item)?.label ?? ref.item;
}

/** What fires a rule, as a line: `Document enters Approved`, `Plan starts`. */
export function whenSaid(rule: Rule, lifecycles: readonly Named[], steps: readonly { id: string; label: string }[]): string {
  const name = labelOf(rule.from, lifecycles, steps);
  if (rule.from.lane === "workflow") return `${name} ${rule.at === "ends" ? "ends" : "starts"}`;
  const lifecycle = lifecycles.find((one) => one.id === rule.from.lane);
  return `${lifecycle?.label ?? rule.from.lane} ${rule.at === "leaves" ? "leaves" : "enters"} ${name}`;
}
