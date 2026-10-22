// A Session answering his message, for the walk `now-and-plain-plans`: while the turn runs, a running account in his words (what is
// done, what it is on, what is next) at the top of Progress and a live view of the page it is building under it, which move on together every few seconds from the
// moment the Session opens. Under a walk they move on its `later` steps alone, so no step misses what it looks at. At the turn's end the reply arrives with a plan in his words, the
// technical groups under Details and Not yet beside Approve, and Pause, Resume, Auto land and Approve change the screen at the press
// while Fleet takes two seconds to answer (`pressed.ts`). Over the `one-session-view` scenario, with its first Session rewritten and
// one idle Session beside it.
//
// Beside it, `now-and-plain-plans-refused`, where Fleet refuses Auto land, so the switch reverts with the refusal's words in the toast.

import type { WaitingItem } from "@armada/protocol";
import type { Session, SessionAttachment, WorkGroup, WorkTask } from "@armada/screens/src/draft/sessions";

import { withPressed } from "../../pressed";
import { askToTell } from "../../tell";
import { showStage } from "../live/stage";
import type { Scenario } from "../moment";
import { planCall, withoutPlanCall } from "../plan-call";
import type { SessionsStore } from "../sessions/script";
import { s261OneSessionView as base } from "./one-session-view";

const MINE = "one-view-1";
const QUIET = "one-view-2";

/** Whether Fleet refuses Auto land two seconds after the press. */
type Take = { refused: boolean };

/** How long Fleet takes to answer a press. */
const ANSWER_MS = 2000;
/** How long the agent takes over each part of the turn. */
const STEP_MS = 4000;

const query = typeof window === "undefined" ? new URLSearchParams() : new URLSearchParams(window.location.search);
/** A walk moves the turn on with its `later` steps, and the clock is left off. */
const WALKED = query.has("walk") || query.has("steps");

export const TITLE = "Email notifications page";
export const ASKED = "Add a page where I can turn email notifications on and off";
export const NOT_YET_WORDS = "Not ready for a plan yet. Keep talking, and record no plan until I ask for one.";
export const REFUSAL_WORDS = "Auto land refused: the forge does not allow auto-merge on this repository.";
export const REPLY = "The page is up with its three switches and a Save button. The plan for keeping the choices and sending the emails is below.";
export const LIVE_TITLE = "Notifications page";

const SUMMARY = [
  "The three switches keep their setting when you come back to the page.",
  "Comments and Mentions emails stop when their switch is off.",
  "The Weekly digest goes out on Mondays to everyone who has it on.",
].join("\n");

/** The plan the Session works to while the turn runs, approved before it began. `at` is the part of the turn, which moves its tasks on. */
const building = (at: number): readonly WorkGroup[] => {
  const mark = (id: string, from: number, to: number): WorkTask => ({ id, text: TASKS[id]!, state: at >= to ? "done" : at >= from ? "working" : "open", seen: at >= from });
  return [
    { id: "p1", agent: "general-purpose", title: "The page and its switches", gives: "A page with the three switches", paths: ["src/pages/notifications.tsx"], tasks: [{ id: "P0", text: TASKS.P0!, state: "done", seen: true }, mark("P1", 0, 1), mark("P2", 1, 2)] },
    { id: "p2", agent: "general-purpose", title: "A line under each switch, and Save", gives: "Each switch says what it does", paths: ["src/pages/notifications.tsx"], tasks: [mark("P3", 2, 3), mark("P4", 3, 4)] },
    { id: "p3", agent: "test-writer", title: "A narrow window", gives: "Nothing is cut off on a small screen", paths: ["src/pages/notifications.test.tsx"], tasks: [mark("P5", 4, 5)] },
  ];
};

const TASKS: Record<string, string> = {
  P0: "Add Notifications to the settings menu",
  P1: "Add the page and the Comments switch",
  P2: "Add the Mentions and Weekly digest switches",
  P3: "Add a line under each switch",
  P4: "Add Save below the switches",
  P5: "Check the page fits a narrow window",
};

/** The plan the agent records at the turn's end, waiting on him: what is left, in his words. */
const GROUPS: readonly WorkGroup[] = [
  {
    id: "n1",
    agent: "general-purpose",
    title: "Remember each choice",
    gives: "Your choices are still there when you come back",
    paths: ["src/pages/notifications.tsx", "src/api/preferences.ts"],
    tasks: [
      { id: "N1", text: "Keep the three choices when Save is pressed", state: "open" },
      { id: "N2", text: "Load them when the page opens", state: "open" },
    ],
  },
  {
    id: "n2",
    agent: "general-purpose",
    title: "Send only what is switched on",
    gives: "No email goes out for a switch that is off",
    paths: ["src/server/notify.ts"],
    tasks: [{ id: "N3", text: "Skip a comment or mention email when its switch is off", state: "open" }],
  },
  {
    id: "n3",
    agent: "general-purpose",
    title: "The Weekly digest",
    gives: "A Monday email with the week's activity",
    paths: ["src/server/digest.ts", "src/server/schedule.ts"],
    tasks: [
      { id: "N4", text: "Build the digest from the week's activity", state: "open" },
      { id: "N5", text: "Send it on Mondays to everyone who has it on", state: "open" },
    ],
  },
];

/** What the Session is linked to while it works: two issues and the pull request for the first. */
const LINKED: readonly SessionAttachment[] = [
  { kind: "issue", number: "41", title: "Let me turn off email notifications", address: "https://git.example/app/issues/41", state: "open" },
  { kind: "issue", number: "44", title: "Make the weekly digest optional", address: "https://git.example/app/issues/44", state: "open" },
  { kind: "pull_request", number: 52, title: "Add a notifications settings page", branch: "notifications-page", address: "https://git.example/app/pull/52", checks: { state: "pending" }, state: "open", auto: false, closes: [41] },
];

/** Each part of the turn: the account as it stands, the tool call the thread gains, and how far the page has got. The last ends the turn. */
const STEPS: readonly { done: readonly string[]; now?: string; next?: string; tool?: string; page: number }[] = [
  { done: [], now: "Adding the page and its first switch", next: "Adding the other switches", tool: "Read src/pages/settings.tsx", page: 0 },
  { done: ["Added the page and its first switch"], now: "Adding the other switches", next: "Adding a line under each switch, and Save", tool: "Edit src/pages/notifications.tsx", page: 1 },
  { done: ["Added the page and its first switch", "Added the other switches"], now: "Adding a line under each switch, and Save", next: "Checking it fits a narrow window", tool: "Edit src/pages/notifications.tsx", page: 2 },
  { done: ["Added the page and its first switch", "Added the other switches", "Added a line under each switch, and Save", "Checked it fits a narrow window"], page: 2 },
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
    live: { url: "/live/notifications.html", title: LIVE_TITLE },
    attachments: [...one.attachments.filter((it) => it.kind !== "issue" && it.kind !== "pull_request"), ...LINKED],
    waitingFor: ended ? [...withoutPlanCall(one.waitingFor), planCall(1, now)] : withoutPlanCall(one.waitingFor),
    work: {
      workflow: { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"], at: ended ? 1 : 0 },
      plan: ended
        ? { version: 2, summary: SUMMARY, scope: ["src/pages", "src/api", "src/server"], approved: false, after: "n-reply", groups: GROUPS }
        : { version: 1, scope: ["src/pages"], approved: true, after: "n-1", groups: building(at) },
    },
  };
};

type Control = Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0];

/** The store over the base's, with the two Sessions shown, the turn moving on from the moment the Session opens, and Fleet slow to answer a press. */
function sessions(take: Take, control: Control): SessionsStore {
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

export const s275NowAndPlainPlans = build("now-and-plain-plans", "A Session answering his message: the account and the page it is building at the top of Progress, then the reply and a plan in his words", { refused: false });
export const s278NowAndPlainPlansRefused = build("now-and-plain-plans-refused", "The same, with Fleet refusing Auto land: the switch reverts with the refusal's words", { refused: true });
