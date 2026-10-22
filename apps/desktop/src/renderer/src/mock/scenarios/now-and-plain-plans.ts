// A Session mid-turn with a plan waiting, for the walk `now-and-plain-plans`: what it is on now reads under its head and on its
// row (the working task, the action Fleet detected, a line a running subagent), the plan reads in his words first with the technical
// groups under Details, Not yet sets the plan aside, and Pause, Resume, Auto land and Approve change the screen at the press while
// Fleet takes two seconds to answer (`pressed.ts`). Over the `one-session-view` scenario, with its first Session rewritten and one
// idle Session beside it.
//
// Two takes, each its own scenario so a walk over it plays by name: `now-and-plain-plans-said` adds the agent's last sentence as a
// third, faintest line under the head, and `now-and-plain-plans-refused` has Fleet refuse Auto land, so the switch reverts with the
// refusal's words in the toast. The url chooses them on the first scenario too: `now=said`, `press=refused`.

import type { WaitingItem } from "@armada/protocol";
import type { Session, WorkGroup } from "@armada/screens/src/draft/sessions";

import { withPressed } from "../../pressed";
import { askToTell } from "../../tell";
import type { Scenario } from "../moment";
import { planCall, withoutPlanCall } from "../plan-call";
import type { SessionsStore } from "../sessions/script";
import { s261OneSessionView as base } from "./one-session-view";

const MINE = "one-view-1";
const QUIET = "one-view-2";

/** `said` draws the agent's last sentence under the Now line; `refused` has Fleet refuse Auto land, two seconds after the press. */
type Take = { said: boolean; refused: boolean };

const query = typeof window === "undefined" ? new URLSearchParams() : new URLSearchParams(window.location.search);
/** The take the url names, on the first scenario: `now=said`, `press=refused`. */
const QUERIED: Take = { said: query.get("now") === "said", refused: query.get("press") === "refused" };

/** How long Fleet takes to answer a press. */
const ANSWER_MS = 2000;

export const TITLE = "Plain plans and Not yet";
export const TASK = "Wire the Not yet press into the plan card";
export const NOT_YET_WORDS = "Not ready for a plan yet. Keep talking, and record no plan until I ask for one.";
export const REFUSAL_WORDS = "Auto land refused: the forge does not allow auto-merge on this repository.";

const SUMMARY = [
  "You see what each agent is working on under the Session title and on its row in the list.",
  "A plan reads in plain words first. The groups, tasks and paths fold under Details.",
  "Not yet puts a plan away, clears it from Needs you and tells the agent to keep talking.",
  "Pause, Resume, Auto land and Approve change the screen the moment you press them.",
].join("\n");

const GROUPS: readonly WorkGroup[] = [
  {
    id: "n1",
    agent: "bridge-engineer",
    title: "Plan card reads the summary first",
    gives: "A plan you can read without the code",
    paths: ["packages/components/src/compositions/SessionThread/SessionThread.tsx"],
    tasks: [
      { id: "N1", text: "Draw the summary and one line a group", state: "done", seen: true },
      { id: "N2", text: TASK, state: "working", seen: true },
      { id: "N3", text: "Fold the groups under Details", state: "open" },
    ],
  },
  {
    id: "n2",
    agent: "bridge-engineer",
    title: "Not yet in the plan's Needs you row",
    gives: "A way to say not yet without typing it",
    paths: ["packages/components/src/compositions/NeedsYouPanel/NeedsYouPanel.tsx", "apps/desktop/src/renderer/src/sessions.tsx"],
    tasks: [{ id: "N4", text: "Add Not yet beside the plan row", state: "open" }],
  },
  {
    id: "n3",
    agent: "bridge-engineer",
    title: "Presses drawn before Fleet answers",
    gives: "Controls that move when you press them",
    paths: ["apps/desktop/src/renderer/src/pressed.ts", "apps/desktop/src/renderer/src/cockpit/deck/SessionTiles.tsx"],
    tasks: [
      { id: "N5", text: "Hold the expected state from the press", state: "open" },
      { id: "N6", text: "Revert on a refusal with its words", state: "open" },
    ],
  },
];

const working = (take: Take) => (one: Session): Session => ({
  ...one,
  title: TITLE,
  turn: { state: "working" },
  standing: "working",
  rows: [
    { id: "n-1", at: "14:02:00", kind: "message", from: { kind: "you" }, text: "Make the plan card read plainly, and let me say not yet." },
    { id: "n-2", at: "14:03:10", kind: "tool", text: "Edit packages/components/src/compositions/SessionThread/SessionThread.tsx" },
    { id: "n-3", at: "14:03:40", kind: "message", from: { kind: "agent" }, text: "Plan v1 is recorded. Approve it, say not yet, or tell me what to change." },
  ],
  attachments: [
    { kind: "slot", slot: 3 },
    { kind: "branch", name: "bridge/plain-plans", slot: 3 },
    { kind: "subagent", id: "sub-1", task: "explorer: find where the plan row is built", state: "running", agentType: "explorer" },
    { kind: "subagent", id: "sub-2", task: "test-writer: a test for the Not yet press", state: "running", agentType: "test-writer" },
  ],
  activity: {
    now: "Editing PlanCard.tsx",
    subagents: [
      { id: "sub-1", label: "explorer: find where the plan row is built", now: "Reading waiting.ts" },
      { id: "sub-2", label: "test-writer: a test for the Not yet press", now: "Editing PlanCard.test.tsx" },
    ],
    ...(take.said ? { said: "Folding the technical groups under Details so the summary reads first" } : {}),
  },
  waitingFor: [...withoutPlanCall(one.waitingFor), planCall(1, new Date().toISOString())],
  work: {
    workflow: { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"], at: 1 },
    plan: { version: 1, summary: SUMMARY, scope: ["packages/components", "apps/desktop"], approved: false, after: "n-3", groups: GROUPS },
  },
});

type Control = Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0];

/** The store over the base's, with the two Sessions shown and Fleet slow to answer a press. */
function sessions(take: Take, control: Control): SessionsStore {
  const store = base.draft!.sessions!(control);
  store.edit(MINE, working(take));
  const timers = new Set<number>();
  const after = (ms: number, run: () => void) => {
    const handle = window.setTimeout(() => {
      timers.delete(handle);
      run();
    }, ms);
    timers.add(handle);
  };
  /** Fleet's answer, two seconds on: `change` lands on the record, and the answer is whether it took the press. */
  const answered = (change: (one: Session) => Session, took = true): Promise<boolean> =>
    new Promise((done) => {
      after(ANSWER_MS, () => {
        if (took) store.edit(MINE, change);
        done(took);
      });
    });
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  const you = (id: string, text: string): Session["rows"][number] => ({ id, at: new Date().toISOString(), kind: "message", from: { kind: "you" }, text });
  const slow: SessionsStore = {
    ...store,
    get: () => {
      const all = store.get();
      if (all !== from) {
        from = all;
        only = all.filter((one) => one.id === MINE || one.id === QUIET);
      }
      return only;
    },
    // Opening the Session must not start the base's script, which asks a question and reads files.
    watch: () => undefined,
    pause: () => answered(({ paused: _, ...one }) => ({ ...one, paused: true, turn: { state: "idle" }, standing: "idle", attachments: one.attachments.filter((it) => it.kind !== "slot") })),
    resume: () => answered(({ paused: _, ...one }) => ({ ...one, turn: { state: "working" }, standing: "working", attachments: [{ kind: "slot", slot: 3 }, ...one.attachments] })),
    autoLand: async (_id, on) => {
      if (take.refused) {
        const took = await answered((one) => one, false);
        askToTell(REFUSAL_WORDS);
        return took;
      }
      return answered(({ autoLand: _, ...one }) => (on ? { ...one, autoLand: true } : one));
    },
    work: {
      ...store.work!,
      approve: () =>
        void answered((one) => ({
          ...one,
          waitingFor: withoutPlanCall(one.waitingFor),
          work: { ...one.work, ...(one.work?.plan === undefined ? {} : { plan: { ...one.work.plan, approved: true } }) },
          rows: [...one.rows, you("n-approved", "Approved the plan.")],
        })),
      // Not yet: the plan is set aside at once, its call on him goes, and the agent is sent the words.
      setAside: () =>
        store.edit(MINE, (one) => ({
          ...one,
          waitingFor: (one.waitingFor ?? []).filter((it: WaitingItem) => it.source !== "plan"),
          work: { ...one.work, ...(one.work?.plan === undefined ? {} : { plan: { ...one.work.plan, setAside: true } }) },
          rows: [...one.rows, you("n-not-yet", NOT_YET_WORDS)],
        })),
    },
    dispose: () => {
      timers.forEach((one) => window.clearTimeout(one));
      store.dispose();
    },
  };
  return withPressed(slow) as SessionsStore;
}

const build = (name: string, says: string, take: Take): Scenario => ({
  ...base,
  name,
  says,
  draft: { ...base.draft, sessions: (control) => sessions(take, control) },
});

export const s275NowAndPlainPlans = build("now-and-plain-plans", "A Session mid-turn: what it is on under its head and on its row, a plan in his words with Not yet beside Approve, and presses drawn before Fleet answers", QUERIED);
export const s276NowAndPlainPlansSaid = build("now-and-plain-plans-said", "The same, with the agent's last sentence as a third line under the head", { said: true, refused: false });
export const s277NowAndPlainPlansRefused = build("now-and-plain-plans-refused", "The same, with Fleet refusing Auto land: the switch reverts with the refusal's words", { said: false, refused: true });
