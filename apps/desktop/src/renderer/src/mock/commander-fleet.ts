// A Job's Commander on the mock Fleet: its thread as the socket would carry it, the card it puts out
// as `JobDetail.commander_card`, what it changes in the plan as `PlanTask.by`, and the two ways it
// hears from the owner. **Every value is the wire's** (`packages/protocol/src/commander.ts`), so the
// screens draw it through the code a real Fleet reaches; only the agent is a script.
//
// **The agent is a script, not a model**, as the Sessions mock's is. A scenario names what the
// Commander does at each moment: a walk's `later` takes the next, the owner's message takes the next
// reply a moment after it is sent, and his answer to its card takes the one that follows. Each moment
// writes through `Commanding`, which publishes the thread, the plan and the panels as Fleet would.

import { callViewOf, commanderAskOf } from "@armada/jobs";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import type { CardOption, CommanderCard, CommanderThread, CommanderThreadItem, JobDetail, Outcome, PlanTask, Saw } from "@armada/protocol";

import type { Fleet } from "./fake-context";
import { unanswered } from "./moment";

/** What a moment may do. Every call publishes. */
export type Commanding = {
  /** Fleet woke it, for this fact. The turn is out. */
  wake: (why: string) => void;
  /** The owner's words, as he typed them. */
  you: (text: string) => void;
  /** A read: a call that changes nothing, by the tool's name. */
  read: (tool: string) => void;
  /** A call that changes the Job, by the tool's name, and the figure the wire carries for it: what the act was taken on. */
  call: (tool: string, figure?: string) => void;
  say: (text: string) => void;
  /** The turn is over. `waiting` is the card it left out, which stays until he answers. */
  end: () => void;
  /** A turn is out again, without a row of its own. */
  working: () => void;
  /** It put a card to the owner. The Job reads as waiting on him until he answers. */
  card: (card: { text: string; reach: string; question: string; options: readonly string[] }) => void;
  /** The plan, as a change makes it. The Job's row counts move with it. */
  plan: (change: (whole: JobDetail) => JobDetail) => void;
  /** Its hand on these tasks: `PlanTask.by`, which a mark on the canvas and the Plan tab read. */
  mark: (...tasks: readonly string[]) => void;
  /** What the Now panel and the Dashboard's calls draw for this Job besides the card. */
  panels: (panels: { now?: NowView }) => void;
};

export type CommanderScript = {
  job: string;
  /** The request the Dashboard's call card quotes beside its card. */
  request?: string;
  /** What the thread and panels hold when the window opens. Nothing here waits. */
  open: (c: Commanding) => void;
  /** One each time a walk lets time pass. */
  moments?: readonly ((c: Commanding) => void)[];
  /** What it does for each message the owner sends, in order; the last answers any more. */
  replies?: readonly ((c: Commanding) => void)[];
  /** What it does once the owner answers its card, with the option he picked. */
  answered?: (c: Commanding, option: CardOption) => void;
};

export type CommanderStore = {
  later: () => void;
  /** `ask_commander`: the owner's words, queued for the next turn. */
  ask: (jobId: string, text: string) => Outcome;
  /** `answer_commander_card`: his pick. Refused for a card that is not the one out. */
  answer: (jobId: string, cardId: string, optionId: string) => Outcome;
};

/** How long a reply takes: long enough that the working mark is seen. */
const REPLY_MS = 900;

/** The first instant of a thread; every row after it is seven seconds on, so a stamp never reads out of order. */
const START = Date.parse("2026-10-11T14:06:00.000Z");

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
    let items: CommanderThreadItem[] = [];
    let replying = false;
    let card: CommanderCard | undefined;
    let now: NowView | undefined;
    let clock = 0;
    let made = 0;
    let moment = 0;
    let replied = 0;

    const stamp = () => {
      clock += 7;
      return new Date(START + clock * 1000).toISOString();
    };
    const id = () => String(made++);
    const thread = (): CommanderThread => ({ state: "open", jobId: script.job, replying, skipped: 0, missed: 0, items });
    /** The panel's rows with the card among its asks, and the calls the Dashboard draws from them. */
    const panels = () => {
      const held = fleet.draft.get() ?? {};
      const shown: NowView | undefined = card === undefined ? now : { ...now, asks: [...(now?.asks ?? []), commanderAskOf(card)] };
      const calls: CallView | undefined =
        shown === undefined ? undefined : { ...callViewOf(shown), ...(script.request === undefined ? {} : { request: script.request }) };
      fleet.draft.set({
        ...held,
        ...(shown === undefined ? {} : { now: { ...held.now, [script.job]: shown } }),
        ...(calls === undefined ? {} : { calls: { ...held.calls, [script.job]: calls } }),
      });
    };
    const publish = () => {
      fleet.publish({ commander: thread() });
      panels();
    };
    const add = (item: (id: string, ts: string) => CommanderThreadItem) => {
      items = [...items, item(id(), stamp())];
      publish();
    };
    const row = (saw: Saw) => add((itemId, ts) => ({ kind: "row", id: itemId, turn: { ts, seq: Number(itemId), by: "drone", saw } }));
    // A call with no figure carries the Job it names, as the door's calls do.
    const called = (tool: string, figure?: string) => row({ event: "called", tool: `mcp__armada__${tool}`, call: `call-${made}`, detail: figure ?? script.job, truncated: false });
    /** The Job's detail carrying the card out, or none. */
    const carrying = (whole: JobDetail): JobDetail => {
      const { commander_card: _gone, ...rest } = whole;
      return { ...rest, ...(card === undefined ? {} : { commander_card: card }), job: { ...whole.job, asking: card !== undefined } };
    };
    /** The card and the Job's row, which reads as waiting on the owner while it is out. */
    const waiting = (out: CommanderCard | undefined) => {
      card = out;
      fleet.move(script.job, { asking: out !== undefined });
      fleet.stepping(script.job, carrying);
    };
    // A detail read after the card went out carries it, as Fleet's would: the window opens the Job
    // after the script's first moment, and what that read finds is the scenario's own.
    fleet.listen((state) => {
      const watched = state.watched;
      if (watched.state === "read" && watched.jobId === script.job && watched.detail.commander_card?.card_id !== card?.card_id) fleet.stepping(script.job, carrying);
    });
    const c: Commanding = {
      wake: (why) => {
        replying = true;
        add((itemId, ts) => ({ kind: "woke", id: itemId, ts, why }));
      },
      you: (text) => {
        replying = true;
        add((itemId, ts) => ({ kind: "asked", id: itemId, ts, text }));
      },
      read: (tool) => called(tool),
      call: called,
      say: (text) => row({ event: "said", text }),
      end: () => {
        replying = false;
        row({ event: "ended", turns: 3, cost_micros: 12_000, refusals: 0 });
      },
      working: () => {
        replying = true;
        publish();
      },
      card: (out) => {
        const options = out.options.map((label, at): CardOption => ({ id: String(at + 1), label }));
        waiting({ card_id: "01M3WJ6FGZ0CARD0000000T7C1", job_id: script.job, by: "commander", raised_at: stamp(), text: out.text, reach: out.reach, question: out.question, options });
        publish();
      },
      plan: (change) => fleet.stepping(script.job, change),
      mark: (...tasks) =>
        fleet.stepping(script.job, (whole) => {
          const plan = whole.work_plan;
          if (plan === undefined) return whole;
          const touched = (task: PlanTask): PlanTask => (tasks.includes(task.id) ? { ...task, by: { who: "commander" } } : task);
          return { ...whole, work_plan: { ...plan, tasks: plan.tasks.map(touched) } };
        }),
      panels: (next) => {
        if (next.now !== undefined) now = next.now;
        panels();
      },
    };

    script.open(c);
    // A Commander that has done nothing is still the Job's, and its panel is drawn.
    publish();
    return {
      later: () => {
        const next = script.moments?.[moment];
        moment += 1;
        next?.(c);
      },
      ask: (_job, text) => {
        c.you(text);
        const reply = script.replies?.[Math.min(replied, (script.replies?.length ?? 1) - 1)];
        replied += 1;
        if (reply !== undefined) window.setTimeout(() => reply(c), REPLY_MS);
        return { ok: true };
      },
      answer: (_job, cardId, optionId) => {
        const picked = card?.options.find((one) => one.id === optionId);
        if (card === undefined || card.card_id !== cardId || picked === undefined) return unanswered(`/jobs/${script.job}/commander/card/answer`);
        const question = card.question;
        waiting(undefined);
        // Fleet keeps the pick, then wakes the Commander with it.
        c.wake(`The owner answered your card, "${question}": ${picked.label}`);
        if (script.answered !== undefined) window.setTimeout(() => script.answered?.(c, picked), REPLY_MS);
        return { ok: true };
      },
    };
  };
}
