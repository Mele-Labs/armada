// What the lanes draw: a workflow's steps in a top lane, then a lane for each lifecycle the workflow uses, each a small state machine, with
// rules as edges between them. **A lifecycle is any lifecycle**: a ticket, a pull request, a deploy, a document. Fleet reads each of its states from
// a tracker, the forge, a watch or an Armada document, and nothing in the lanes names one. **The editor's model is layered and Fleet's pieces are whole**: the host reads
// the pieces into this model and saves back only the pieces an edit changed. Fleet serves no placement of states, so the host lays them out (`docs/design`, Workspaces, lifecycles and rules). The editor draws the definitions; a Job's Lanes view draws the same lanes with
// one lane for each instance and the rules that fired, read from what Fleet serves of the Job (`packages/screens/src/session-lanes.ts`).

import type { LifecycleRule } from "@armada/protocol";

/** The lane a node sits in: the workflow's own, or a lifecycle's id (in the editor) or an instance's (on a Job). */
export type WorkflowLane = "workflow";

/** A place in a lane: the lane, and the step or state in it. */
export type Ref = { lane: string; item: string };

/** The id a node is drawn under, and the one a selection names. */
export const nodeId = (lane: string, item: string): string => `${lane}/${item}`;

/**
 * **Three tiers**: the default's, which every Workspace and repository keeps until it changes something; a Workspace's, keyed `workspace:<id>`; and one repository's, by its id. Each takes
 * what the tier above says until it says otherwise. A repository can be in two Workspaces, which are peers: the tab reads its own value, else the one Workspace that sets it, else the default's.
 */
export const DEFAULT = "default";

/** The level a Workspace's tab and its pieces are kept at. */
export const workspaceLevel = (id: string): string => `workspace:${id}`;

/** Whether a level is a Workspace's. */
export const isWorkspaceLevel = (level: string): boolean => level.startsWith("workspace:");

/**
 * Pieces of one id that two Workspaces holding the same repository both set at one place: Fleet runs neither for that repository. Each group is the levels that tie, and a repository
 * that is in two of a group reads the level above them.
 */
export type Clash = readonly (readonly string[])[];

/** The Workspaces a page draws, by their level, and the ones that hold each repository. Set from the Workspace the editor draws, since a level's chain depends on it. */
let heldBy: ReadonlyMap<string, readonly string[]> = new Map();

/** The Workspaces the page draws: the ones that hold the repository it is on. A Workspace that names none draws the one `id` and `name` say, or none where `id` is empty. */
export const workspacesOf = (workspace: Pick<Workspace, "id" | "name" | "workspaces">): readonly { id: string; name: string }[] =>
  workspace.workspaces ?? (workspace.id === "" ? [] : [{ id: workspace.id, name: workspace.name }]);

/** The name a level goes by: a Workspace's, else a repository's. */
export const levelName = (workspace: Pick<Workspace, "id" | "name" | "workspaces" | "repositories">, level: string): string =>
  isWorkspaceLevel(level) ? (workspacesOf(workspace).find((one) => workspaceLevel(one.id) === level)?.name ?? level) : (workspace.repositories.find((one) => one.id === level)?.name ?? level);

/** Takes the Workspaces each repository is in from the Workspace the page draws. A repository that names none is in each of the Workspaces. */
export function holding(workspace: Pick<Workspace, "id" | "name" | "workspaces" | "repositories">): void {
  const all = workspacesOf(workspace);
  heldBy = new Map(workspace.repositories.map((one) => [one.id, all.filter((each) => one.in === undefined || one.in.includes(each.id)).map((each) => workspaceLevel(each.id))] as const));
}

/** The levels of a chain that a clash sets aside: those of a group that two or more of the chain are in. */
const setAside = (chain: readonly string[], clash: Clash): string[] => chain.filter((level) => clash.some((group) => group.includes(level) && group.filter((one) => chain.includes(one)).length > 1));

/** The levels a tab reads, nearest first: a repository's own, the Workspaces that hold it, then the default's. Given the clash a piece has, the Workspaces that tie are not read. */
export function levelsOf(scope: string, clash: Clash = []): string[] {
  if (scope === DEFAULT) return [DEFAULT];
  const chain = isWorkspaceLevel(scope) ? [scope] : [scope, ...(heldBy.get(scope) ?? [])];
  const aside = setAside(chain, clash);
  return [...chain.filter((level) => !aside.includes(level)), DEFAULT];
}

/** The Workspace levels a clash sets aside on a tab: the ones whose pieces differ, so the tab reads the default's. */
export const clashAt = (clash: Clash | undefined, scope: string): string[] => setAside(scope === DEFAULT || isWorkspaceLevel(scope) ? [] : [scope, ...(heldBy.get(scope) ?? [])], clash ?? []);

/** The tracker's, forge's or watch's own words that count as being in a state, at one level: the default's, a Workspace's or one repository's. */
export type StateReads = { source: string; values: readonly string[]; scope: string };

/** A state as the Workspace defines it: where Fleet reads it from is the lifecycle's, and the words that count as being in it are its own. */
export type StateDef = {
  id: string;
  label: string;
  /** The level it was added at, so it exists there and below. Absent, it is the default's. */
  scope?: string;
  /** Another name for it at a level below the default. */
  names?: readonly { scope: string; label: string }[];
  reads: readonly StateReads[];
  /** Nothing follows it. */
  end?: true;
  /** The tabs whose own piece does not have it. Fleet runs a tab's piece whole, so a state the default added is not there on a tab that keeps its own. */
  absent?: readonly string[];
  /** The Workspaces whose lifecycles differ, so a repository in both reads the default's. */
  clash?: Clash;
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
  /** The level it was added at, so it exists there and below. Absent, it is the default's. */
  scope?: string;
  /** Who may change the piece that defines it, which decides whether it can be deleted: Armada's cannot. */
  kept?: Kept;
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

/**
 * What a level below the default changes of a rule. **Where the host keeps rules as pieces, the override is the level's own piece whole** (`wire`, with every field drawn from it), since Fleet runs the
 * narrower piece and merges nothing. Elsewhere it is any of the rule's fields, and only those.
 */
export type RuleOverride = { scope: string } & Partial<Pick<Rule, "label" | "from" | "at" | "to" | "ifs" | "does" | "autonomy" | "workflow" | "wire">>;

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
  /** `default`, `workspace:<id>`, or the id of the one repository it applies in. */
  scope: string;
  /** Where a level below changes it from the one above. */
  overrides?: readonly RuleOverride[];
  /** The one workflow it belongs to. Absent, it applies in every workflow that has both ends. */
  workflow?: string;
  /** The Workspaces whose pieces differ, so a repository in both reads the default's. */
  clash?: Clash;
  /** The piece as Fleet keeps it at the rule's own level, which the rule's fields above are drawn from. Absent where the host keeps no pieces. */
  wire?: LifecycleRule;
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

/** What a level below the default changes of a step: its name, its words, what ends it and what passes it over. `endsOn` goes with `ends`. */
export type StepLayer = { step: string; scope: string; label?: string; words?: string; ends?: Waits; endsOn?: { lane: string; state: string }; passedOver?: Waits };

/** A step added at a level below the default: it runs there and below, after the step it follows, and the definition does not hold it. */
export type AddedStep = { id: string; label: string; after: string; scope: string; words?: string };

/** A repository a Workspace holds. `in` is the ids of the Workspaces it is in, among the ones the page draws; absent, all of them. */
export type Repository = { id: string; name: string; in?: readonly string[] };

/** What one workflow draws of the Workspace: where its steps sit, and which states of which lifecycles it has a lane for, and where. */
export type WorkflowPlace = {
  /** The column and ends-on of each of its steps, by step id. */
  steps: Readonly<Record<string, Omit<LaneStep, "id" | "label">>>;
  /** The column of each state it draws, by `lifecycle/state`. **A lifecycle with no state placed has no lane here**, and a state with no column is not drawn. */
  states: Readonly<Record<string, number>>;
  /** What the Workspace and its repositories change of the steps. */
  layers?: readonly StepLayer[];
  /** The steps they add. */
  added?: readonly AddedStep[];
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
  /** The Workspaces that hold the repository the page is on, each with a tab. Absent, the one `id` and `name` say. */
  workspaces?: readonly { id: string; name: string }[];
  repositories: readonly Repository[];
  lifecycles: readonly LifecycleDef[];
  /** What else a step can wait on or be passed over for. */
  observes?: readonly Observed[];
  /** What each workflow draws of the lifecycles, by workflow id. */
  workflows: Readonly<Record<string, WorkflowPlace>>;
  rules: readonly Rule[];
};

/** Whether something added at `own` exists on the tab for `scope`: at that level or below it. What the default holds has no level of its own and exists everywhere. */
export const existsAt = (own: string | undefined, scope: string, clash?: Clash): boolean => own === undefined || own === DEFAULT || levelsOf(scope, clash).includes(own);

/** Whether a state is there on the tab for `scope`: added at that level or above, and not left out of the piece that tab runs. */
export const stateExistsAt = (state: StateDef, scope: string): boolean => existsAt(state.scope, scope, state.clash) && !(state.absent ?? []).includes(scope);

/**
 * The lifecycles a workflow draws, each with the states it places, in the Workspace's order. Given a `scope`, a state added at another level is left out and the ways in and out of it are
 * joined, so the lane reads as it runs there.
 */
export function lifecyclesFor(workspace: Workspace, workflow: string, scope?: string): Lifecycle[] {
  const place = workspace.workflows[workflow]?.states ?? {};
  return workspace.lifecycles.flatMap((lifecycle): Lifecycle[] => {
    if (scope !== undefined && !existsAt(lifecycle.scope, scope)) return [];
    const states = lifecycle.states.flatMap((state): LifecycleState[] => {
      const col = place[`${lifecycle.id}/${state.id}`];
      return col === undefined || (scope !== undefined && !stateExistsAt(state, scope)) ? [] : [{ ...state, col }];
    });
    const drawn = new Set(states.map((state) => state.id));
    // A state left out is stepped over: whatever led into it now leads to whatever it led to.
    let next = lifecycle.next;
    for (const gone of lifecycle.states.filter((state) => !drawn.has(state.id) && scope !== undefined && !stateExistsAt(state, scope))) {
      const into = next.filter(([, to]) => to === gone.id).map(([from]) => from);
      const out = next.filter(([from]) => from === gone.id).map(([, to]) => to);
      next = [...next.filter(([from, to]) => from !== gone.id && to !== gone.id), ...into.flatMap((from) => out.map((to) => [from, to] as const))];
    }
    return states.length === 0 ? [] : [{ ...lifecycle, states, next: next.filter(([from, to]) => drawn.has(from) && drawn.has(to)) }];
  });
}

/** The lifecycles a workflow draws with each state named as it is at a level. */
export function lifecyclesAt(workspace: Workspace, workflow: string, scope: string): Lifecycle[] {
  return lifecyclesFor(workspace, workflow, scope).map((lifecycle) => ({ ...lifecycle, states: lifecycle.states.map((state) => ({ ...state, label: stateLabelAt(state, scope) })) }));
}

/** The levels below the default that name a state another way or read it as other words, as the ids of the Workspace and the repositories. */
export const stateDiffers = (state: StateDef): string[] => [
  ...new Set([...state.reads.map((one) => one.scope), ...(state.names ?? []).map((one) => one.scope), ...(state.absent ?? []), ...(state.scope === undefined ? [] : [state.scope])].filter((one) => one !== DEFAULT)),
];

/** The levels a rule differs at: where it applies alone, and where a level below changes it. */
export const ruleDiffers = (rule: Rule): string[] => [...new Set([...(rule.scope === DEFAULT ? [] : [rule.scope]), ...(rule.overrides ?? []).map((one) => one.scope)])];

/** The levels that change a step. */
export const stepDiffers = (place: WorkflowPlace | undefined, step: string): string[] => [
  ...new Set([...(place?.layers ?? []).filter((one) => one.step === step).map((one) => one.scope), ...(place?.added ?? []).filter((one) => one.id === step).map((one) => one.scope)]),
];

/**
 * The steps at a level: the definition's, with each step added at it or above placed after the step it follows. A step the default added since sits between them where the columns put it,
 * so the order the steps run in is the order they are drawn in.
 */
export function stepsAt<T extends { id: string; label: string }>(steps: readonly T[], place: WorkflowPlace | undefined, scope: string, make: (added: AddedStep) => T): T[] {
  const out = [...steps];
  for (const one of place?.added ?? []) {
    if (!existsAt(one.scope, scope)) continue;
    const after = out.findIndex((each) => each.id === one.after);
    let at = after < 0 ? out.length : after + 1;
    const col = place?.steps[one.id]?.col;
    while (col !== undefined && at < out.length && (place?.steps[out[at]!.id]?.col ?? col) < col) at++;
    out.splice(at, 0, make(one));
  }
  return out;
}

/** A rule as it reads at a level: the default's own, then each level down changing what it changes. `above` leaves the level's own out, for what it would read without it. */
export function ruleAt(rule: Rule, scope: string, above = false): Rule {
  return levelsOf(scope, rule.clash)
    .slice(above ? 1 : 0)
    .reverse()
    .reduce((now, level) => {
      const own = rule.overrides?.find((one) => one.scope === level);
      if (own === undefined) return now;
      const { scope: _level, ...fields } = own;
      return { ...now, ...fields };
    }, rule);
}

/** A state's name at a level: the nearest that names it, else its own. */
export function stateLabelAt(state: StateDef, scope: string, above = false): string {
  for (const level of levelsOf(scope, state.clash).slice(above ? 1 : 0)) {
    const named = state.names?.find((one) => one.scope === level);
    if (named !== undefined) return named.label;
  }
  return state.label;
}

/** The words that count as being in a state at a level, for one source: the nearest level that sets them. Absent where none does. */
export function readsAt(state: StateDef, source: string, scope: string, above = false): readonly string[] | undefined {
  for (const level of levelsOf(scope, state.clash).slice(above ? 1 : 0)) {
    const found = state.reads.find((one) => one.source === source && one.scope === level);
    if (found !== undefined) return found.values;
  }
  return undefined;
}

/** What the levels from the default down to `scope` change of a step, in the order they apply. */
export const layersAt = (place: WorkflowPlace | undefined, step: string, scope: string): StepLayer[] => levelsOf(scope).reverse().flatMap((level) => (place?.layers ?? []).filter((one) => one.step === step && one.scope === level));

// ---- What a canvas draws -------------------------------------------------------------------------------------------------

/**
 * What a node is, which is its shape and its tint (`LaneCanvas.css`): a step is a rounded box; a state read from the tracker or the forge, or from an Armada document, is a pill; a step that waits
 * and a state a watch reads are hexagons; a Job another Job started is a card with the drone glyph; and where the workflow starts and ends are small circles. **A kind is never a status**: its tint is a
 * faint ground from `kinds.css`, and what a node stands in (working, waiting, went through) is its frame and its mark.
 */
export type NodeKind = "step" | "wait" | "tracker" | "forge" | "document" | "watch" | "job" | "start" | "end";

/** What the mark on a piece says while the pull request that removes it from the repository is open. */
export const REMOVAL_WAITS = "Removed from the repository when its pull request merges";

/** What the mark says while the draft that keeps the piece for every Job waits on the owner's Approve. */
export const KEEP_WAITS = "Kept for every Job once its draft is approved";

/** What a piece's clock waits on: a pull request that removes it, or the owner's Approve of a draft that keeps it. */
export type PendingKind = "removal" | "keep";

/** The sentence of each wait, which is its mark's tooltip. */
export const PENDING_SAID: Readonly<Record<PendingKind, string>> = { removal: REMOVAL_WAITS, keep: KEEP_WAITS };

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
  /** A step that waits on a lifecycle state, and the words for it: one sentence for each end it has not had, each drawn as a line. */
  waiting?: readonly string[];
  end?: boolean;
  skipped?: boolean;
  /** Where an instance stood when ending the Job ended it. It takes the place of `here`. */
  ended?: boolean;
  /** Who the instance in this state is assigned to, drawn on the node. */
  who?: string;
  /** The ticket the Job on this lane was started for and now holds, drawn on the node the Job stands at, as `who` is. */
  ticket?: CarriedTicket;
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
  /** What kind of thing the lane is, in its lifecycle's own name, drawn under the title in place of `source` where the title is not that name: a Job's lane is titled for the Job, so it reads `Job`. */
  what?: string;
  /** The tooltip of the title: a ticket's own title. */
  note?: string;
  /** The tracker's lifecycle: this lane is a ticket. */
  ticket?: true;
  /** Its head opens the Workflow of the Job the lane stands for (`onOpen`): set by the caller, which knows whether that Job has lanes to open. */
  opens?: true;
  /** Ending the Job ended this instance. */
  ended?: boolean;
  /** The Job's own copy of this lane's lifecycle changed, and how. */
  changed?: string;
  nodes: readonly LaneNodeView[];
  /** The ways a state is left within the lane. */
  moves: readonly (readonly [string, string])[];
  /** The repository's file that defines it is being removed by a pull request that has not merged, or a draft that keeps it for every Job waits on Approve. */
  pending?: PendingKind;
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
  /** On a Job: fired and acted, fired and asked, still waiting, or fired and set aside. */
  status?: "acted" | "asked" | "waiting" | "skipped";
  /** What an asked rule left for the person to answer, by the id of its card: pressing the rule opens it. */
  needs?: string;
  /** The repositories this differs in, by name: where it applies alone, or where its autonomy is another. */
  differs?: readonly string[];
  /** Added by a change waiting on Approve. */
  proposed?: true;
  /** The Job's own copy changed this rule, and how. */
  changed?: string;
  /** The repository's file that defines it is being removed by a pull request that has not merged, or a draft that keeps it for every Job waits on Approve. */
  pending?: PendingKind;
  /** The same rule's second edge: drawn with its mark and no words. */
  quiet?: true;
  /** What the rule did, a line for each act, as the chip's tooltip says it: `Assigned to you, moved to In progress`. Absent reads the status. */
  did?: readonly string[];
};

// ---- A Job's lanes -------------------------------------------------------------------------------------------------------

/**
 * A ticket a started Job holds for the Job that started it: its key, its title where the source gave one, the name of the state it stands in as the lifecycle names it, and that
 * state's id. **Both are `""` where it stands in none**: it is not read yet, or the source said a status no state lists, which `state` then keeps as it was said.
 */
export type CarriedTicket = { label: string; title?: string; state: string; stateId: string };

/** One thing a Job holds: a ticket, its pull request, a deploy. */
export type Instance = {
  id: string;
  /** The id of the lifecycle it is an instance of. */
  lifecycle: string;
  /** What the lane and the rail call it: `ARM-41`, `PR #812`, `Deploy`. */
  label: string;
  /** A ticket's own title. */
  title?: string;
  /** For a Job this one started: the ticket it was started for. That Job holds the ticket and its rules move it, so this Job has no lane for it and reads it from there. */
  ticket?: CarriedTicket;
  /** `""` where it stands in no state: not read yet, or a status its lifecycle lists none for. */
  state: string;
  /** The word its source used, for a status no state lists. It is in flight until the lifecycle maps it. */
  unmapped?: string;
  /** Ending the Job ended it, standing where it was. It is no longer in flight. */
  ended?: true;
  /** The states it went through, in order, the current one last. */
  visited: readonly string[];
  assignee?: string;
  /** What is known of it at a state, by state id: `Maya, Jun` at Reviewers on. */
  says?: Readonly<Record<string, string>>;
};

/** Where the edge of a rule that hands to a step lands when the rule does not name one: the step the Job was on when it fired, else the one it is on. */
export const CURRENT_STEP = "$step";

/** One act of a rule as the Job's rules kept it, for the rule's panel. `standing` is Fleet's word: `planned`, `asked`, `stuck`, `done`, `skipped`, `told` or `not_carried`. */
export type FiredAct = { text: string; course: "acts" | "asks"; standing: string; said?: string; at: string };

/** A rule that fired for an instance, and how it ended. */
export type Fired = {
  rule: string;
  instance: string;
  outcome: "acted" | "asked" | "waiting" | "skipped";
  at: string;
  /** What it did, a line for each act, for the chip's tooltip. */
  did?: readonly string[];
  /** The id of the card an asked rule left in the thread, while it waits to be answered. */
  needs?: string;
  /** The step the Job was on when it fired, which an edge that hands to the current step lands on. */
  step?: string;
  /** Each act it planned and where it stands. Absent for a rule the mock fires. */
  acts?: readonly FiredAct[];
};

/**
 * A change made to this Job's own copy, marked on what it changes: `lane` is the instance and `item` its state, `workflow` and `item` the step added or skipped,
 * `rule` and `item` the rule, or a lifecycle's id and no `item` for the lifecycle as a whole.
 */
export type JobChange = { id: string; lane: string; item: string; text: string; at: string; /** A rule or a lifecycle the Job took out of its copy: nothing of it is left to keep for every Job. */ dropped?: true; /** A changed rule's name, from any rule the Job or its repository holds, drawn or not. */ name?: string };

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
  /** What the step the Job is on waits for, a sentence for each end that has not come, as Fleet words them: `Waiting on PR #812: Approved`. Absent when it waits on none. */
  waiting?: readonly string[];
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
