// A Session answering his message, for the walks `now-and-plain-plans-*`: while the turn runs, a running account in his words (what is
// done, what it is on, what is next) and a live view of the page it is building, which move on together every few seconds from the
// moment the Session opens. Under a walk they move on its `later` steps alone, so no step misses what it looks at. At the turn's end the reply arrives with a plan in his words, the
// technical groups under Details and Not yet beside Approve, and Pause, Resume, Auto land and Approve change the screen at the press
// while Fleet takes two seconds to answer (`pressed.ts`). Over the `one-session-view` scenario, with its first Session rewritten and
// one idle Session beside it.
//
// One scenario a take, so a walk over it plays by name: where the account and the live view draw (`account-place.ts`), in the thread
// under his message, in a panel beside the thread, or as a band on the composer; and `now-and-plain-plans-refused`, where Fleet
// refuses Auto land, so the switch reverts with the refusal's words in the toast.

import type { WaitingItem } from "@armada/protocol";
import type { Session, WorkGroup } from "@armada/screens/src/draft/sessions";

import { placeAccount, type AccountPlace } from "../../account-place";
import { withPressed } from "../../pressed";
import { askToTell } from "../../tell";
import { showStage } from "../live/stage";
import type { Scenario } from "../moment";
import { planCall, withoutPlanCall } from "../plan-call";
import type { SessionsStore } from "../sessions/script";
import { s261OneSessionView as base } from "./one-session-view";

const MINE = "one-view-1";
const QUIET = "one-view-2";

/** Where the account draws, and whether Fleet refuses Auto land two seconds after the press. */
type Take = { place: AccountPlace; refused: boolean };

/** How long Fleet takes to answer a press. */
const ANSWER_MS = 2000;
/** How long the agent takes over each part of the turn. */
const STEP_MS = 4000;

const query = typeof window === "undefined" ? new URLSearchParams() : new URLSearchParams(window.location.search);
/** A walk moves the turn on with its `later` steps, and the clock is left off. */
const WALKED = query.has("walk") || query.has("steps");

export const TITLE = "Plain plans and Not yet";
export const ASKED = "Make the plan card read like what I get, not how it's built";
export const NOT_YET_WORDS = "Not ready for a plan yet. Keep talking, and record no plan until I ask for one.";
export const REFUSAL_WORDS = "Auto land refused: the forge does not allow auto-merge on this repository.";
export const REPLY = "The plan card now opens on what you get, and the groups and paths sit under Details. The plan for the rest of this work is below.";
export const LIVE_TITLE = "Plan card";

const SUMMARY = [
  "While it works, a few lines say what is done, what it is on and what is next, beside the page it is building.",
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
      { id: "N2", text: "Fold the groups under Details", state: "done", seen: true },
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

/** Each part of the turn: the account as it stands, the tool call the thread gains, and how far the page has got. The last ends the turn. */
const STEPS: readonly { done: readonly string[]; now?: string; next?: string; tool?: string; page: number }[] = [
  { done: [], now: "Looking at what the plan card shows you today", next: "Fold the groups and paths under Details", tool: "Read packages/components/src/compositions/SessionThread/SessionThread.tsx", page: 0 },
  { done: ["Moved the groups and paths under a Details fold"], now: "Rewriting the summary so it says what you get", next: "Show you the card", tool: "Edit packages/components/src/compositions/SessionThread/SessionThread.tsx", page: 1 },
  { done: ["Moved the groups and paths under a Details fold", "Rewrote the summary so it says what you get"], now: "Checking it still reads in a narrow window", next: "Show you the card", tool: "Edit packages/components/src/compositions/SessionThread/SessionThread.css", page: 2 },
  { done: ["Moved the groups and paths under a Details fold", "Rewrote the summary so it says what you get", "Checked it reads in a narrow window"], page: 2 },
];

const LAST = STEPS.length - 1;

/** The Session at part `at` of the turn: its account and thread so far, and at the last part the reply and the plan waiting on him. */
const atStep = (at: number) => (one: Session): Session => {
  const step = STEPS[at]!;
  const ended = at === LAST;
  const now = new Date().toISOString();
  const tools = STEPS.slice(0, at + 1).flatMap((part, n) => (part.tool === undefined ? [] : [{ id: `n-tool-${n}`, at: now, kind: "tool" as const, text: part.tool }]));
  return {
    ...one,
    title: TITLE,
    turn: ended ? { state: "idle" } : { state: "working" },
    standing: ended ? "replied" : "working",
    rows: [
      { id: "n-1", at: "14:02:00", kind: "message", from: { kind: "you" }, text: ASKED },
      ...tools,
      ...(ended ? [{ id: "n-reply", at: now, kind: "message" as const, from: { kind: "agent" as const }, text: REPLY }] : []),
    ],
    account: { for: "n-1", done: step.done, ...(step.now === undefined ? {} : { now: step.now }), ...(step.next === undefined ? {} : { next: step.next }), at: now },
    live: { url: "/live/plan-card.html", title: LIVE_TITLE },
    waitingFor: ended ? [...withoutPlanCall(one.waitingFor), planCall(1, now)] : withoutPlanCall(one.waitingFor),
    work: {
      workflow: { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"], at: ended ? 1 : 0 },
      ...(ended ? { plan: { version: 1, summary: SUMMARY, scope: ["packages/components", "apps/desktop"], approved: false, after: "n-reply", groups: GROUPS } } : {}),
    },
  };
};

type Control = Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0];

/** The store over the base's, with the two Sessions shown, the turn moving on from the moment the Session opens, and Fleet slow to answer a press. */
function sessions(take: Take, control: Control): SessionsStore {
  placeAccount(take.place);
  const store = base.draft!.sessions!(control);
  let step = 0;
  store.edit(MINE, atStep(step));
  showStage(STEPS[step]!.page);
  const timers = new Set<number>();
  const after = (ms: number, run: () => void): number => {
    const handle = window.setTimeout(() => {
      timers.delete(handle);
      run();
    }, ms);
    timers.add(handle);
    return handle;
  };
  // The turn moves on a part every few seconds once the Session is open. A walk's `later` moves it on at once and stops the clock.
  let pending: number | undefined;
  let clocked = !WALKED;
  const advance = () => {
    if (pending !== undefined) {
      window.clearTimeout(pending);
      timers.delete(pending);
      pending = undefined;
    }
    if (step >= LAST) return;
    step += 1;
    store.edit(MINE, atStep(step));
    showStage(STEPS[step]!.page);
    if (clocked && step < LAST) pending = after(STEP_MS, advance);
  };
  let started = false;
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
    // Opening the Session starts its turn's clock, and not the base's script, which asks a question and reads files.
    watch: (id) => {
      if (id !== MINE || started) return undefined;
      started = true;
      if (clocked) pending = after(STEP_MS, advance);
      return undefined;
    },
    later: () => {
      clocked = false;
      advance();
    },
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

export const s275NowAndPlainPlansThread = build("now-and-plain-plans-thread", "A Session answering his message: the account and the page it is building in a card under the message, then the reply and a plan in his words", { place: "thread", refused: false });
export const s276NowAndPlainPlansSide = build("now-and-plain-plans-side", "The same, with the account and the page in a panel beside the thread", { place: "side", refused: false });
export const s277NowAndPlainPlansBand = build("now-and-plain-plans-band", "The same, with the account as a band on the composer that opens to what is done, what is next and the page", { place: "band", refused: false });
export const s278NowAndPlainPlansRefused = build("now-and-plain-plans-refused", "The same, with Fleet refusing Auto land: the switch reverts with the refusal's words", { place: "thread", refused: true });
