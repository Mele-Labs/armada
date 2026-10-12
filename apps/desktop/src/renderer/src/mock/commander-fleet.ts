// A Job's Commander on the mock Fleet: its thread, what it changes in the plan and on the Now panel,
// and the two ways it hears from the owner. Mock only: Fleet serves no Commander, and the decision
// that makes one is `.claude/decisions/2026-10-11-a-job-has-a-commander.md`.
//
// **The agent is a script, not a model**, as the Sessions mock's is. A scenario names what the
// Commander does at each moment: a walk's `later` takes the next, the owner's message takes the next
// reply a moment after it is sent, and his answer to its card takes the one that follows. Each moment
// writes through `Commanding`, which publishes the thread, the plan and the panels as Fleet would.

import type { CallView } from "@armada/jobs/draft/calls";
import type { CommanderAct, CommanderDraft, CommanderRow, CommanderView } from "@armada/jobs/draft/commander";
import type { NowView } from "@armada/jobs/draft/now";
import type { TaskBy } from "@armada/jobs/draft/task";
import type { JobDetail, PlanTask } from "@armada/protocol";

import type { Fleet } from "./fake-context";

/** What a moment may do. Every call publishes. */
export type Commanding = {
  /** Fleet woke it, for this fact. The turn is out. */
  wake: (why: string) => void;
  /** The owner's words, as he typed them. */
  you: (text: string) => void;
  read: (text: string) => void;
  say: (text: string) => void;
  act: (act: CommanderAct) => void;
  /** Where the turn stands: out, between turns, or a card out to the owner. */
  state: (state: CommanderView["state"]) => void;
  /** The mark a task wears for what it changed in it. */
  mark: (task: string, said: string) => void;
  /** The plan, as a change makes it. The Job's row counts move with it. */
  plan: (change: (whole: JobDetail) => JobDetail) => void;
  /** What the Now panel and the Dashboard's calls draw for this Job from here. */
  panels: (panels: { now?: NowView; calls?: CallView }) => void;
  /** The owner's answer to the card, from a surface that draws it besides the Now panel. */
  answer: (option: string) => void;
};

export type CommanderScript = {
  job: string;
  /** What the thread and panels hold when the window opens. Nothing here waits. */
  open: (c: Commanding) => void;
  /** One each time a walk lets time pass. */
  moments?: readonly ((c: Commanding) => void)[];
  /** What it does for each message the owner sends, in order; the last answers any more. */
  replies?: readonly ((c: Commanding) => void)[];
  /** What it does once the owner answers its card. */
  answered?: (c: Commanding, option: string) => void;
};

export type CommanderStore = { later: () => void };

/** How long a reply takes: long enough that the working mark is seen. */
const REPLY_MS = 900;

/** The Job's counts as its plan states them. */
export function countedIn(whole: JobDetail): JobDetail {
  const tasks = whole.work_plan?.tasks ?? [];
  const count = (state: string) => tasks.filter((task) => task.state === state).length;
  const failed = count("failed");
  return { ...whole, job: { ...whole.job, tasks: { done: count("done"), working: count("working"), open: count("open"), dropped: count("dropped"), ...(failed === 0 ? {} : { failed }) } } };
}

/** The plan with `task` changed, and a new open task after it where `after` names one. */
export function splitPlan(whole: JobDetail, task: string, narrowed: Partial<PlanTask>, added: PlanTask): JobDetail {
  const plan = whole.work_plan;
  if (plan === undefined) return whole;
  const at = plan.tasks.findIndex((one) => one.id === task);
  const tasks = plan.tasks.flatMap((one, index) => (index === at ? [{ ...one, ...narrowed }, added] : [one]));
  const groups = plan.groups?.map((group) => (group.tasks.includes(task) ? { ...group, tasks: group.tasks.flatMap((id) => (id === task ? [id, added.id] : [id])) } : group));
  return countedIn({ ...whole, work_plan: { ...plan, tasks, ...(groups === undefined ? {} : { groups }) } });
}

/** The plan with these tasks moved to a state. */
export function taskStates(whole: JobDetail, to: Readonly<Record<string, string>>): JobDetail {
  const plan = whole.work_plan;
  if (plan === undefined) return whole;
  const tasks = plan.tasks.map((one) => {
    const state = to[one.id];
    if (state === undefined) return one;
    const { failed_reason: _failed, ...rest } = one;
    return { ...rest, state };
  });
  return countedIn({ ...whole, work_plan: { ...plan, tasks } });
}

/** The plan with a task failed, and why. */
export function failedTask(whole: JobDetail, task: string, reason: string): JobDetail {
  const plan = whole.work_plan;
  if (plan === undefined) return whole;
  return countedIn({ ...whole, work_plan: { ...plan, tasks: plan.tasks.map((one) => (one.id === task ? { ...one, state: "failed", failed_reason: reason } : one)) } });
}

/** The step's red runs of a group dropped from the record the lead reads, once the Check that failed them is no longer asked. */
export function runsCleared(whole: JobDetail, group: string): JobDetail {
  return { ...whole, steps: whole.steps.map((step) => (step.check_runs === undefined ? step : { ...step, check_runs: step.check_runs.filter((run) => run.group !== group) })) };
}

/** A group moved to a state, with its open run closed or a new one begun. */
export function groupState(whole: JobDetail, group: string, state: string): JobDetail {
  const plan = whole.work_plan;
  if (plan === undefined) return whole;
  const { ended_at: _ended, ...running } = plan.groups?.find((one) => one.id === group) ?? { id: group, tasks: [], state };
  const groups = plan.groups?.map((one) => (one.id !== group ? one : { ...running, state, attempts: [...(one.attempts ?? []), { attempt: (one.attempts?.length ?? 0) + 1, step_id: "implement", step_attempt: 1, started_at: new Date().toISOString() }] }));
  return { ...whole, work_plan: { ...plan, ...(groups === undefined ? {} : { groups }) } };
}

export function commandedBy(script: CommanderScript) {
  return (fleet: Fleet): CommanderStore => {
    let rows: readonly CommanderRow[] = [];
    let state: CommanderView["state"] = "idle";
    let marks: Readonly<Record<string, TaskBy>> = {};
    let now: NowView | undefined;
    let calls: CallView | undefined;
    let clock = 0;
    let made = 0;
    let moment = 0;
    let replied = 0;

    /** A time of day that only moves forward, so a row's stamp never reads out of order. */
    const at = () => {
      clock += 7;
      return `14:${String(6 + Math.floor(clock / 60)).padStart(2, "0")}:${String(clock % 60).padStart(2, "0")}`;
    };
    const publish = () => {
      const held = fleet.draft.get() ?? {};
      fleet.draft.set({
        ...held,
        commander: draft(),
        ...(now === undefined ? {} : { now: { ...held.now, [script.job]: now } }),
        ...(calls === undefined ? {} : { calls: { ...held.calls, [script.job]: calls } }),
      });
    };
    const add = (row: (id: string, stamp: string) => CommanderRow) => {
      made += 1;
      rows = [...rows, row(`c${made}`, at())];
      publish();
    };
    const c: Commanding = {
      wake: (why) => {
        add((id, stamp) => ({ id, at: stamp, kind: "woke", why }));
        c.state("working");
      },
      you: (text) => add((id, stamp) => ({ id, at: stamp, kind: "you", text })),
      read: (text) => add((id, stamp) => ({ id, at: stamp, kind: "read", text })),
      say: (text) => add((id, stamp) => ({ id, at: stamp, kind: "said", text })),
      act: (act) => add((id, stamp) => ({ id, at: stamp, kind: "act", act })),
      state: (next) => {
        state = next;
        publish();
      },
      mark: (task, said) => {
        marks = { ...marks, [task]: { who: "commander", said } };
        publish();
      },
      plan: (change) => fleet.stepping(script.job, (whole) => change(whole)),
      panels: (next) => {
        if (next.now !== undefined) now = next.now;
        if (next.calls !== undefined) calls = next.calls;
        publish();
      },
      answer: (option) => draft().answer(script.job, "card", option),
    };
    const draft = (): CommanderDraft => ({
      views: { [script.job]: { state, rows, marks } },
      send: (_job, text) => {
        c.you(text);
        c.state("working");
        const reply = script.replies?.[Math.min(replied, (script.replies?.length ?? 1) - 1)];
        replied += 1;
        if (reply !== undefined) window.setTimeout(() => reply(c), REPLY_MS);
      },
      answer: (_job, _key, option) => {
        if (script.answered === undefined) return;
        c.state("working");
        window.setTimeout(() => script.answered?.(c, option), REPLY_MS);
      },
    });

    script.open(c);
    return {
      later: () => {
        const next = script.moments?.[moment];
        moment += 1;
        next?.(c);
      },
    };
  };
}
