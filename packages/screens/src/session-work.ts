// What a Session works to, read off its record: the proposer's picks, the workflow it follows, the plan it recorded, the files it
// and its subagents touched, and the concerns a judge raised. One direction and no state: the record and what the window holds
// beside it go in, a `SessionWork` comes out (`draft/sessions.ts`). Fleet's word is used as it comes: a step is lit only where
// Fleet names one, and a task's state is the one Fleet kept.

import type { AdversarialReviewState, Finding } from "@armada/components/src/compositions/AdversarialReview/review-state";
import type { AdversarialFinding, BuildPiece, FileTouch, PlanMark as WirePlanMark, ProposedField, SessionPlan, SessionRecord, SessionRow as WireRow, WaitingItem, WorkflowSummary } from "@armada/protocol";

import type { AgentTask, PlanMark, SessionAttachment, SessionRow, SessionWork, SuggestLine, WorkConcern, WorkDrone, WorkFile, WorkflowChoice, WorkLanding, WorkPlan } from "./draft/sessions";

/** What the window holds beside a record that the work is read with. */
export type WorkBeside = {
  /** The workflows Fleet holds, which name a workflow's steps. */
  workflows: readonly WorkflowSummary[];
  /** The Session's ledger as drawn, where a plan's sketch is found. */
  attachments: readonly SessionAttachment[];
  /** The thread as Fleet sent it, where the row that recorded the plan is found. */
  rows: readonly WireRow[] | undefined;
  /** The thread as drawn, which the plan's card is placed after. */
  drawn: readonly SessionRow[];
  /** Picks the person has changed in this window, before the record says so. */
  unpicked: ReadonlySet<ProposedField>;
  /** The owner's setting `features.workflows` is off: no workflow, no steps and no proposer's mark for one are drawn. */
  workflowsOff?: boolean;
  /** Fleet refused the last press of the adversarial review on this plan version, in its words. */
  reviewRefused?: ReviewRefused;
};

/** Fleet's refusal of a press of the adversarial review, for the plan version it was pressed on. */
export type ReviewRefused = { version: number; said: string };

const findingOf = (one: AdversarialFinding): Finding => ({
  id: one.id,
  consequence: one.consequence,
  expected: one.expected,
  found: one.found,
  ...(one.impact === undefined ? {} : { impact: one.impact }),
  ...(one.cites === undefined ? {} : { cites: one.cites }),
  ...(one.about.kind === "plan" ? {} : { about: one.about.id }),
  ...(one.settled === undefined ? {} : { settled: one.settled.how }),
});

/**
 * The plan's adversarial review as the card draws it. Absent where none was run and nothing was refused, which draws the press.
 * **A review of another version is stale whatever its state**, its findings kept where their parts still stand, so Run again reads
 * the plan as it is now. A refusal of a press on this version is drawn as a failure, in Fleet's words.
 */
export function adversarialOf(plan: Pick<SessionPlan, "version" | "adversarial">, refused?: ReviewRefused): AdversarialReviewState | undefined {
  if (refused !== undefined && refused.version === plan.version) return { state: "failed", said: refused.said };
  const review = plan.adversarial;
  if (review === undefined) return undefined;
  const findings = review.findings.map(findingOf);
  if (review.subject.version !== plan.version) return { state: "stale", read: review.subject.version, findings };
  if (review.state === "running") return { state: "running" };
  if (review.state === "failed") return { state: "failed", said: review.failed ?? "The review failed" };
  return { state: "done", findings };
}

/** The workflows a Session may be given, as the picker lists them. */
export const choicesOf = (workflows: readonly WorkflowSummary[]): WorkflowChoice[] =>
  workflows.map((one) => ({ id: one.id, name: one.name, steps: one.steps.map((step) => step.label), manifestId: one.manifest_id }));

const PICKABLE: readonly ProposedField[] = ["workflow", "model", "effort"];

/** The workflow a Session follows and the step it is at, where Fleet names one. */
function workflowOf(record: SessionRecord, workflows: readonly WorkflowSummary[]): SessionWork["workflow"] {
  if (record.workflow === undefined) return undefined;
  const found = heldOf(record, workflows);
  if (found === undefined) return { name: record.workflow, steps: [] };
  // `workflow_step` is the step's index, which Fleet leaves off for a custom Session, after the last step's pull request merged, and while workflows are off.
  const at = record.workflow_step;
  const phase = at === undefined ? undefined : found.steps[at]?.phase;
  return {
    name: found.name,
    steps: found.steps.map((step) => step.label),
    ...(at === undefined || at < 0 || at >= found.steps.length ? {} : { at }),
    ...(phase === undefined ? {} : { phase }),
  };
}

/** The workflow Fleet holds that the Session follows: the one of its repository, else any of that id. */
function heldOf(record: SessionRecord, workflows: readonly WorkflowSummary[]): WorkflowSummary | undefined {
  if (record.workflow === undefined) return undefined;
  return workflows.find((one) => one.id === record.workflow && (record.manifest_id === undefined || one.manifest_id === record.manifest_id)) ?? workflows.find((one) => one.id === record.workflow);
}

/** What the Session's workflow lands in, as Fleet resolves it. Absent where Fleet does not say. */
const landsOf = (record: SessionRecord, workflows: readonly WorkflowSummary[]): WorkLanding | undefined => heldOf(record, workflows)?.lands;

/** The wire row that recorded this version of the plan, and the last row before it that the thread draws. */
function afterOf(plan: SessionPlan, rows: readonly WireRow[] | undefined, drawn: readonly SessionRow[]): string | undefined {
  if (rows === undefined) return undefined;
  const at = rows.findIndex((one) => one.kind === "plan" && one.version === plan.version);
  if (at < 0) return undefined;
  const ids = new Set(drawn.map((one) => one.id));
  for (let back = at - 1; back >= 0; back -= 1) {
    const id = rows[back]!.id;
    if (ids.has(id)) return id;
  }
  return undefined;
}

/** The plan as the view draws it. A sketch the ledger does not hold yet is left off. */
export function planOf(plan: SessionPlan, beside: Pick<WorkBeside, "attachments" | "rows" | "drawn">): WorkPlan {
  const sketch = plan.sketch === undefined ? undefined : beside.attachments.find((one) => one.kind === "sketch" && one.id === plan.sketch?.id);
  const after = afterOf(plan, beside.rows, beside.drawn);
  return {
    version: plan.version,
    scope: plan.scope,
    approved: plan.approved,
    ...(plan.summary === "" ? {} : { summary: plan.summary }),
    ...(plan.set_aside_at === undefined ? {} : { setAside: true as const }),
    groups: plan.groups.map((group) => ({
      id: group.id,
      agent: group.agent,
      title: group.title,
      paths: group.paths,
      ...(group.gives === undefined || group.gives === "" ? {} : { gives: group.gives }),
      ...(group.issue === undefined ? {} : { issue: group.issue }),
      tasks: group.tasks.map((task) => ({
        id: task.id,
        text: task.text,
        state: task.state,
        ...(task.seen === true ? { seen: true as const } : {}),
        ...(task.changed_at === undefined ? {} : { changedAt: task.changed_at }),
        ...(task.marked === undefined ? {} : { marked: task.marked }),
        ...(task.stale === undefined ? {} : { stale: { why: task.stale.why, since: task.stale.since } }),
      })),
    })),
    ...(plan.changes === undefined ? {} : { changes: { added: plan.changes.added, renamed: plan.changes.renamed, removed: plan.changes.removed } }),
    ...(sketch === undefined || sketch.kind !== "sketch" || plan.sketch === undefined || !("nodes" in sketch.drawing) ? {} : { sketch: { id: plan.sketch.id, title: plan.sketch.title, scene: sketch.drawing } }),
    ...(after === undefined ? {} : { after }),
  };
}

/** The directory a file's drone hovers over: the first two directories, which the Field's plots are drawn at. */
const plotOf = (path: string): string => {
  const parts = path.split("/");
  return parts.slice(0, Math.min(2, parts.length - 1)).join("/");
};

/** The drones and the files the touches name: each drone at the file it touched last, and each file with what was last done to it. */
export function fieldOf(touches: readonly FileTouch[]): { field: WorkDrone[]; files: WorkFile[] } {
  const ordered = [...touches].sort((a, b) => a.at.localeCompare(b.at));
  const last = new Map<string, FileTouch>();
  const files = new Map<string, WorkFile>();
  for (const touch of ordered) {
    last.set(touch.by?.id ?? "", touch);
    files.set(touch.path, { path: touch.path, act: touch.act });
  }
  const field = [...last].map(([id, touch]): WorkDrone => ({
    id: id === "" ? "main" : id,
    ...(touch.by === undefined ? {} : { agent: touch.by.agent }),
    path: plotOf(touch.path),
    doing: "write",
    file: touch.path,
    at: touch.at,
  }));
  return { field, files: [...files.values()] };
}

/** The concerns waiting on the person, as the thread draws them. */
export function concernsOf(items: readonly WaitingItem[] | undefined): WorkConcern[] {
  return (items ?? [])
    .filter((one) => one.source === "judge")
    .map((one) => ({
      id: one.id,
      text: one.text,
      ...(one.concern?.from.kind === "step" ? { step: one.concern.from.label } : {}),
      ...(one.concern?.expected === undefined ? {} : { expected: one.concern.expected }),
      ...(one.concern?.produced === undefined ? {} : { produced: one.concern.produced }),
      ...(one.concern?.consequence === undefined ? {} : { consequence: one.concern.consequence }),
      ...(one.cites === undefined || one.cites.length === 0 ? {} : { cites: one.cites.map((cite) => (cite.line === undefined ? cite.path : `${cite.path}:${cite.line}`)) }),
    }));
}

/** The agent's own task list, from the build record's pieces, in the order made: a finished piece is done, one the tool has in progress is working, the rest are open. */
export function tasksOf(pieces: readonly BuildPiece[]): AgentTask[] {
  return pieces.map((one): AgentTask => ({ id: one.id, subject: one.text, state: one.piece === "solid" || one.piece === "hollow" ? "done" : one.working === true ? "working" : "open" }));
}

/**
 * What a Session works to. A Helm Session has none. The proposer's picks and the workflow picker are a hosted Session's, since a
 * terminal's workflow is not Fleet's to set.
 */
export function workOf(record: SessionRecord, beside: WorkBeside): SessionWork | undefined {
  if (record.helm === true) return undefined;
  const hosted = record.hosted !== undefined;
  const off = beside.workflowsOff === true;
  const picked = (record.proposal?.picked ?? []).filter((one) => PICKABLE.includes(one) && !beside.unpicked.has(one) && !(off && one === "workflow")) as ("workflow" | "model" | "effort")[];
  const workflow = off ? undefined : workflowOf(record, beside.workflows);
  const landing = landsOf(record, beside.workflows);
  const { field, files } = fieldOf(record.touches ?? []);
  const concerns = record.state === "ended" ? [] : concernsOf(record.waiting_for);
  const tasks = record.plan !== undefined || record.state === "ended" ? [] : tasksOf(record.build?.pieces ?? []);
  const adversarial = record.plan === undefined ? undefined : adversarialOf(record.plan, beside.reviewRefused);
  return {
    ...(hosted
      ? { proposal: { workflow: record.workflow ?? null, ...(record.proposal?.reading === true ? { reading: true as const } : {}), ...(picked.length === 0 ? {} : { picked }) } }
      : {}),
    ...(workflow === undefined ? {} : { workflow }),
    ...(record.plan === undefined ? {} : { plan: planOf(record.plan, beside) }),
    ...(adversarial === undefined ? {} : { adversarial }),
    ...(tasks.length === 0 ? {} : { tasks }),
    ...(field.length === 0 ? {} : { field, files }),
    ...(concerns.length === 0 ? {} : { concerns }),
    ...(record.judging === true && record.state !== "ended" ? { judging: true as const } : {}),
    ...(record.judge === undefined ? {} : { judged: { at: record.judge.at, concerns: record.judge.concerns } }),
    ...(off ? { railOff: true as const } : {}),
    ...(landing === undefined ? {} : { landing }),
  };
}

/**
 * The concerns the person settled, each a line of the thread at the time it was settled. A sent one is his message to the agent,
 * left out where the thread already carries it; a dismissed one is the folded line and the agent was told nothing.
 */
export function settledRowsOf(items: readonly WaitingItem[] | undefined, thread: readonly SessionRow[]): { at: string; row: SessionRow }[] {
  const said = thread.flatMap((one) => (one.kind === "message" && one.from.kind === "you" ? [one.text] : []));
  return (items ?? []).flatMap((one): { at: string; row: SessionRow }[] => {
    const settled = one.concern?.settled;
    if (one.source !== "judge" || settled === undefined) return [];
    const at = settled.at;
    if (settled.how === "dismissed") {
      const step = one.concern?.from.kind === "step" ? one.concern.from.label : "Judge";
      return [{ at: settled.at, row: { id: `settled:${one.id}`, at, kind: "concern" as const, step, text: one.text } }];
    }
    if (said.some((text) => text.includes(one.text))) return [];
    return [{ at: settled.at, row: { id: `settled:${one.id}`, at, kind: "message" as const, from: { kind: "you" as const }, text: one.text } }];
  });
}

/** His marks as Fleet takes them: an added task and an added group are two marks, and the rest keep their names. */
export function wireMarksOf(marks: readonly PlanMark[]): WirePlanMark[] {
  return marks.map((one): WirePlanMark => {
    switch (one.kind) {
      case "add":
        return one.task === undefined ? { kind: "add_group", id: one.id, text: one.text } : { kind: "add_task", group: one.group, id: one.id, text: one.text };
      case "move":
        return { kind: "move_task", task: one.task, from: one.from, to: one.to };
      case "agent":
        return { kind: "subagent", group: one.group, from: one.from, to: one.to };
      default:
        return one;
    }
  });
}

const VERBS: readonly SuggestLine["verb"][] = ["Renamed", "Removed", "Added", "Moved", "Subagent", "Note"];

/** The lines of a plan-changes row, in the words the thread draws. */
export const linesOfWire = (lines: readonly { verb: string; place: string; from?: string; text?: string }[]): SuggestLine[] =>
  lines.map((one) => ({
    verb: VERBS.find((verb) => verb === one.verb) ?? "Note",
    where: one.place,
    ...(one.from === undefined ? {} : { from: one.from }),
    ...(one.text === undefined ? {} : { text: one.text }),
  }));

const CHANGED = { "session.workflow": "Workflow", "session.model": "Model", "session.effort": "Effort" } as const;

/** What a timeline entry that changed a Session's workflow, model or effort says: the value's name and what it was and became. */
export function changeOfEvent(kind: string, says: string): { what: string; from: string; to: string } | undefined {
  const what = CHANGED[kind as keyof typeof CHANGED];
  if (what === undefined) return undefined;
  const found = /^[a-z]+:\s*(.*?)\s*(?:→|->)\s*(.*)$/.exec(says);
  return found === null ? undefined : { what, from: found[1]!, to: found[2]! };
}
