// What the Workflows page draws of the Keep for every Job drafts a repository has open. A draft is the text of the file its approval would save, so a piece the repository holds only
// gets the clock, and one it does not hold is read from the text and drawn: a lifecycle as a lane in the Workspace, a rule as an added rule. **A draft that does not read as a piece is not
// drawn**, which is Fleet's to refuse when the owner approves it.

import { parseDefinitionText } from "@armada/components";
import type { PendingKind, Rule, Workspace } from "@armada/components";
import type { LifecycleDefinition, LifecycleRule, PieceScope, RuleAct, RuleAmong, RuleCondition, RuleTarget, RuleWhen, StateReadFrom } from "@armada/protocol";

import type { KeepDraft } from "../../../shared/api/lifecycles";
import { levelOf, lifecycleLayers, ruleLayers } from "./levels";
import type { PiecesRead } from "./levels";
import { drawnRule, namedOf, workspaceOf } from "./read";

type Table = Record<string, unknown>;

const table = (value: unknown): Table => (typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Table) : {});
const list = (value: unknown): unknown[] => (Array.isArray(value) ? value : []);
const word = (value: unknown): string | undefined => (typeof value === "string" ? value : undefined);
const words = (value: unknown): string[] => list(value).flatMap((one) => word(one) ?? []);

const scopeOf = (file: Table): PieceScope => {
  const within = word(file.scope);
  return within === undefined ? { kind: "everywhere" } : { kind: "within", id: within };
};

function readOf(raw: Table): StateReadFrom | undefined {
  if (raw.tracker !== undefined) return { source: "tracker", words: words(raw.tracker) };
  const forge = word(raw.forge);
  if (forge !== undefined) return { source: "forge", fact: forge };
  const watch = word(raw.watch);
  if (watch !== undefined) return { source: "watch", state: watch };
  const fleet = word(raw.fleet);
  return fleet === undefined ? undefined : { source: "fleet", fact: fleet };
}

function lifecycleOf(file: Table): LifecycleDefinition | undefined {
  const id = word(file.lifecycle_id);
  const name = word(file.name);
  if (id === undefined || name === undefined) return undefined;
  const states = list(file.states).flatMap((one) => {
    const state = table(one);
    const read = readOf(table(state.read));
    const stateId = word(state.id);
    const label = word(state.label);
    return stateId === undefined || label === undefined || read === undefined ? [] : [{ id: stateId, label, read }];
  });
  const transitions = list(file.transitions).flatMap((one) => {
    const move = table(one);
    const from = word(move.from);
    const to = word(move.to);
    return from === undefined || to === undefined ? [] : [{ from, to }];
  });
  return { id, name, scope: scopeOf(file), states, transitions, ends: words(file.ends) };
}

/** The instances a target names, which the file writes as the one key of `entered`, `linked_from`, `linking_to` and `held`, and the state they stand in as `in`. */
function amongOf(on: Table): RuleAmong | undefined {
  const linkedFrom = word(on.linked_from);
  if (linkedFrom !== undefined) return { kind: "linked_from", lifecycle: linkedFrom };
  const linkingTo = word(on.linking_to);
  if (linkingTo !== undefined) return { kind: "linking_to", lifecycle: linkingTo };
  const held = word(on.held);
  if (held !== undefined) return { kind: "held", lifecycle: held };
  return on.entered === true ? { kind: "entered" } : undefined;
}

function targetOf(on: unknown): RuleTarget | undefined {
  const raw = table(on);
  const among = amongOf(raw);
  const inState = word(raw.in);
  return among === undefined ? undefined : { among, ...(inState === undefined ? {} : { in_state: inState }) };
}

function whenOf(raw: Table): RuleWhen | undefined {
  const enters = table(raw.enters);
  const lifecycle = word(enters.lifecycle);
  const state = word(enters.state);
  if (lifecycle !== undefined && state !== undefined) return { kind: "enters", lifecycle, state };
  const starts = word(raw.step_starts);
  if (starts !== undefined) return { kind: "step_starts", step: starts };
  const ends = word(raw.step_ends);
  if (ends !== undefined) return { kind: "step_ends", step: ends };
  const job = word(raw.job);
  return job === undefined ? undefined : { kind: "job", event: job };
}

function conditionOf(raw: Table): RuleCondition | undefined {
  for (const quantity of ["every", "any", "no"]) {
    const on = table(raw[quantity]);
    const among = amongOf(on);
    const inState = word(on.in);
    if (raw[quantity] !== undefined && among !== undefined && inState !== undefined) return { kind: "instances", among, quantity, in_state: inState };
  }
  const job = table(raw.job);
  const workflow = word(job.workflow);
  if (workflow !== undefined) return { kind: "workflow", workflow };
  const step = word(job.step);
  if (step !== undefined) return { kind: "at_step", step };
  return job.pull_request_opened === true ? { kind: "pull_request_opened" } : undefined;
}

function actOf(raw: Table): { kind: string; what: RuleAct } | undefined {
  const tracker = table(raw.tracker_write);
  const forge = table(raw.forge_write);
  const target = targetOf(tracker.on ?? forge.on ?? table(raw.draft).on);
  if (raw.tracker_write !== undefined && target !== undefined) {
    const assign = word(tracker.assign);
    const to = word(tracker.move);
    if (assign !== undefined) return { kind: "tracker_write", what: { act: "tracker_assign", to: assign, target } };
    if (to !== undefined) return { kind: "tracker_write", what: { act: "tracker_move", to, target } };
    if (tracker.close === true) return { kind: "tracker_write", what: { act: "tracker_close", target } };
  }
  if (raw.forge_write !== undefined && target !== undefined) {
    if (forge.mark_ready === true) return { kind: "forge_write", what: { act: "forge_mark_ready", target } };
    if (forge.merge === true) return { kind: "forge_write", what: { act: "forge_merge", target } };
    if (forge.auto_merge === true) return { kind: "forge_write", what: { act: "forge_auto_merge", target } };
    if (forge.request_reviewers !== undefined) return { kind: "forge_write", what: { act: "forge_request_reviewers", from: words(forge.request_reviewers), target } };
  }
  const template = word(table(raw.draft).template);
  if (template !== undefined && target !== undefined) return { kind: "draft", what: { act: "draft", template, target } };
  const job = table(raw.start_job);
  const prompt = word(job.prompt);
  if (prompt !== undefined) return { kind: "start_job", what: { act: "start_job", prompt, ...(word(job.workflow) === undefined ? {} : { workflow: word(job.workflow)! }) } };
  const message = word(table(raw.tell_agent).message);
  if (message !== undefined) return { kind: "tell_agent", what: { act: "tell", message } };
  const command = word(table(raw.run_command).command);
  if (command !== undefined) return { kind: "run_command", what: { act: "run_command", command } };
  const skill = word(table(raw.run_agent).skill);
  return skill === undefined ? undefined : { kind: "run_agent", what: { act: "run_skill", skill } };
}

function ruleOf(file: Table): LifecycleRule | undefined {
  const id = word(file.rule_id);
  const name = word(file.name);
  const when = whenOf(table(file.when));
  if (id === undefined || name === undefined || when === undefined) return undefined;
  const autonomy = word(file.autonomy);
  const failure = table(file.on_failure);
  return {
    id,
    name,
    scope: scopeOf(file),
    when,
    conditions: list(file.if).flatMap((one) => conditionOf(table(one)) ?? []),
    does: list(file.do).flatMap((one) => {
      const act = actOf(table(one));
      return act === undefined ? [] : [{ ...act, autonomy: autonomy ?? "asks" }];
    }),
    ...(autonomy === undefined ? {} : { autonomy }),
    on_failure: { retries: typeof failure.retries === "number" ? failure.retries : 0, then: word(failure.then) ?? "ask_owner" },
  };
}

/** What a draft's text is, where it is a lifecycle or a rule. */
function piece(draft: KeepDraft): { lifecycle: LifecycleDefinition } | { rule: LifecycleRule } | undefined {
  let file: Table;
  try {
    file = table(parseDefinitionText(draft.text));
  } catch {
    return undefined;
  }
  if (draft.kind === "lifecycle") {
    const lifecycle = lifecycleOf(file);
    return lifecycle === undefined ? undefined : { lifecycle };
  }
  const rule = ruleOf(file);
  return rule === undefined ? undefined : { rule };
}

/** The clock of every piece with an open draft, as `rule:<id>` and `lifecycle:<id>`, whether the repository holds the piece or not. */
export const keepMarks = (drafts: readonly KeepDraft[]): Map<string, PendingKind> => new Map(drafts.map((one) => [`${one.kind}:${one.id}`, "keep" as const]));

/** What every piece with a clock waits on: the drafts that keep it, then the removals a pull request has open, which stand where one piece has both. */
export function pendingMarks(drafts: readonly KeepDraft[], removing: Iterable<string>): Map<string, PendingKind> {
  const marks = keepMarks(drafts);
  for (const key of removing) marks.set(key, "removal");
  return marks;
}

/** The read with the lifecycles of its drafts that the repository does not hold, kept as the repository's own so the lanes draw them. */
export function withDraftLifecycles(read: PiecesRead, drafts: readonly KeepDraft[], repository: string): PiecesRead {
  const held = lifecycleLayers(read);
  const added = drafts.flatMap((one) => {
    const found = one.kind === "lifecycle" && !held.has(one.id) ? piece(one) : undefined;
    return found !== undefined && "lifecycle" in found ? [{ lifecycle: found.lifecycle, kept: { rank: { reach: "repository", source: "repository" }, path: `${one.id}.yml` } }] : [];
  });
  if (added.length === 0) return read;
  return { ...read, repositories: read.repositories.map((one) => (one.id === repository ? { ...one, lifecycles: [...one.lifecycles, ...added] } : one)) };
}

/** The rules of the drafts that the repository does not hold, drawn from their text against the lifecycles of the Workspace. A rule that lands on no node is not drawn, as one held is not. */
export function draftedRules(read: PiecesRead, drafts: readonly KeepDraft[], workspace: Workspace): Rule[] {
  const held = ruleLayers(read);
  const defs = namedOf(workspace.lifecycles);
  return drafts.flatMap((one): Rule[] => {
    const found = one.kind === "rule" && !held.has(one.id) ? piece(one) : undefined;
    if (found === undefined || !("rule" in found)) return [];
    const drawn = drawnRule(found.rule, defs);
    if (drawn === undefined) return [];
    const { workflow, ...fields } = drawn;
    return [{ id: found.rule.id, ...fields, kept: "repository", scope: levelOf(found.rule.scope, read), wire: found.rule, ...(workflow === undefined ? {} : { workflow }) }];
  });
}

/**
 * The Workspace the page draws: the read with the lifecycles its drafts would keep. **A lifecycle only a draft holds is kept by no file**, so it carries no `kept`, which is what stops the editor
 * offering its Name and its Delete: there is no piece to rename or remove, and Approve is what makes one.
 */
export function workspaceDrawn(read: PiecesRead, drafts: readonly KeepDraft[], repository: string): Workspace | undefined {
  const held = lifecycleLayers(read);
  const workspace = workspaceOf(withDraftLifecycles(read, drafts, repository));
  const only = new Set(drafts.flatMap((one) => (one.kind === "lifecycle" && !held.has(one.id) ? [one.id] : [])));
  if (workspace === undefined || only.size === 0) return workspace;
  return { ...workspace, lifecycles: workspace.lifecycles.map((one) => (only.has(one.id) ? withoutKept(one) : one)) };
}

const withoutKept = ({ kept: _kept, ...rest }: Workspace["lifecycles"][number]): Workspace["lifecycles"][number] => rest;
