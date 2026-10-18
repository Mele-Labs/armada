// One view of a Session, on the Cockpit and on the Sessions page: its thread and composer in the middle, its plan and workflow in the Status
// area, its drones on a Field, a judge's concerns as cards, and a proposal to start from. One Session goes from its dispatched words to a
// pull request ready, a step each time the walk lets time pass or he presses an act. The Session's `work` is draft data
// (`packages/screens/src/draft/sessions.ts`); nothing here reaches Fleet. The walk `oneSessionView` plays it, in two takes.

import type { TimelineEvent, WriteClash, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { PlanChanges, Session, SessionRow, SessionWork, WorkConcern, WorkDrone, WorkFile, WorkGroup, WorkTask, WorkTaskState, WorkflowChoice } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { linesOf } from "../../session-view/plan-marks";
import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { arrow, box, scene } from "../scenes";
import { sessionsStore } from "../sessions/script";
import type { SessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const MINE = "one-view-1";
const QUIET = "one-view-2";
const BUSY = "one-view-3";
const BIG = "one-view-4";
const LIVE = "one-view-5";
const BRANCH = "armada/lease-retry";

/** A long workflow, to see how twelve steps read in the Status area. */
const RELEASE: WorkflowChoice = { id: "Release", name: "Release", steps: ["Branch", "Changelog", "Version", "Build", "Sign", "Notarize", "Smoke", "Verify", "Tag", "Publish", "Announce", "Pull request"] };

const WORKFLOWS: readonly WorkflowChoice[] = [
  { id: "Feature", name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"] },
  { id: "Bug fix", name: "Bug fix", steps: ["Reproduce", "Fix", "Verify", "Pull request"] },
  { id: "Code review", name: "Code review", steps: ["Read", "Review", "Report"] },
  RELEASE,
];
const FEATURE = WORKFLOWS[0]!;

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

const WORDS = "Make the slot lease retry when two Sessions ask for the same bay at once.";

/** The Session as dispatched: the words are in the thread, and the proposer has read them. Nothing has run. */
const dispatched: Session = {
  id: MINE,
  title: "Retry the slot lease",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurn: "14:02",
  lastTurnAt: ago(1),
  standing: "idle",
  rows: [you("v-1", "14:02:00", WORDS)],
  attachments: [],
  // The live bar carries all of these at once: a big context, a stale mod, a write clash with another Session.
  context: { tokens: 342_000, window: 400_000 },
  modOutOfDate: true,
  work: { proposal: { workflow: FEATURE.name, picked: ["workflow", "model", "effort"] }, workflow: { ...FEATURE, at: 0 } },
};

const quiet: Session = {
  id: QUIET,
  title: "Name the store migrations",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurn: "13:41",
  lastTurnAt: ago(22),
  standing: "idle",
  rows: [agent("q-1", "13:41:10", "The migrations are named by their order.")],
  attachments: [{ kind: "branch", name: "fix/migration-names", slot: 2 }],
};

const busy: Session = {
  id: BUSY,
  title: "Rotate the audit log",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "working" },
  lastTurn: "14:00",
  lastTurnAt: ago(2),
  standing: "working",
  rows: [you("b-1", "13:50:00", "Rotate the audit log at midnight."), { id: "b-2", at: "14:00:05", kind: "tool", text: "Edit crates/store/src/audit.rs" }],
  attachments: [{ kind: "branch", name: "armada/audit-rotation", slot: 1 }],
  // The long workflow with the Session at Smoke, six steps done.
  work: {
    workflow: { ...RELEASE, at: 6 },
    plan: {
      scope: ["crates/store", "apps/desktop"],
      approved: true,
      groups: [
        { id: "a1", agent: "rust-engineer", title: "Rotate at midnight", paths: ["crates/store"], tasks: [{ id: "A1", text: "Rotate the log file at midnight", state: "done", seen: true }, { id: "A2", text: "Keep seven files", state: "done", seen: true }] },
        { id: "a2", agent: "bridge-engineer", title: "Show the rotation", paths: ["apps/desktop"], tasks: [{ id: "A3", text: "List rotated files in System", state: "working", seen: true }] },
      ],
    },
  },
};

// ---- What the Session records as it goes ------------------------------------------------------------------

const SCOPE = ["crates/fleet", "crates/ipc", "packages/protocol", "apps/desktop"];

const task = (id: string, text: string, state: WorkTaskState): WorkTask => ({ id, text, state, seen: state !== "open" });

/** The task groups at a stage of the build: 0 recorded, 1 started, 2 the first two done, 3 the Fleet group done, 4 everything done. */
function groups(stage: number): WorkGroup[] {
  const at = (from: number, done: number): WorkTaskState => (stage >= done ? "done" : stage >= from ? "working" : "open");
  const r = revision.on;
  return [
    { id: "g1", agent: "rust-engineer", title: "Retry the slot lease when the pool is contended, with back off", paths: ["crates/fleet"], tasks: [task("T1", "Retry the lease when the pool is contended", at(1, 2)), ...(r ? [] : [task("T2", "Back off between attempts", at(2, 3))])] },
    { id: "g2", agent: "protocol-engineer", title: "Attempt count on the wire", paths: ["crates/ipc", "packages/protocol"], tasks: [task("T3", "Add attempts to the lease reply", at(1, 2)), ...(r ? [task("T5", "Bump the protocol ID", at(1, 2))] : [])] },
    { id: "g3", agent: "bridge-engineer", title: "Attempt on the bay", paths: ["apps/desktop"], tasks: [task("T4", r ? "Show the attempt count on the bay tile" : "Show the attempt on the bay tile", at(2, 4))] },
  ];
}

/** Whether the agent has recorded the plan again after his changes: the groups then read as it revised them. */
const revision = { on: false };

/** What the revision changed, as the plan records it. */
const CHANGES: PlanChanges = {
  added: ["T5"],
  renamed: [{ id: "T4", from: "Show the attempt on the bay tile" }],
  removed: [{ group: "g1", text: "Back off between attempts" }],
};

// ---- A plan at size: six groups and twenty-five tasks, unevenly spread -------------------------------------------

/** The change the plan is for: Sessions absorb Jobs. A task's state is a letter: open, working, handed in, done, failed. */
const BIG_GROUPS: readonly { id: string; agent: string; title: string; paths: readonly string[]; tasks: readonly string[] }[] = [
  {
    id: "p1",
    agent: "rust-engineer",
    title: "Plan tool in Fleet",
    paths: ["crates/fleet", "crates/store"],
    tasks: [
      "Add a record_plan tool to the Session MCP",
      "Store the plan on the Session record",
      "Check each group's paths against the repository",
      "Refuse a plan with no tasks",
      "Keep the sketch with the plan",
      "Replace a plan the agent records again",
      "Emit a plan event when a task changes state",
      "Read the tasks back through get_session",
    ],
  },
  {
    id: "p2",
    agent: "protocol-engineer",
    title: "Plan on the wire",
    paths: ["crates/ipc", "packages/protocol"],
    tasks: ["Put the plan in the Session reply", "Carry task state on the event", "Mirror the types in packages/protocol", "Bump the protocol ID"],
  },
  {
    id: "p3",
    agent: "bridge-engineer",
    title: "Plan panel with edits, notes and moves between groups, in the overlay",
    paths: ["apps/desktop/src/renderer/src/session-view"],
    tasks: ["Draw each group as an expanding row", "Tick tasks as events arrive", "Fold the panel to its band", "Open the sketch from the header"],
  },
  {
    id: "p4",
    agent: "bridge-engineer",
    title: "Approval card in the thread",
    paths: ["packages/components/src/compositions/SessionThread"],
    tasks: ["Draw the compact card above the composer", "Open the plan in a review overlay", "Settle the card once it is approved"],
  },
  {
    id: "p5",
    agent: "general-purpose",
    title: "Judge at step boundaries",
    paths: ["crates/fleet", "crates/judge"],
    tasks: ["Read the work at each step boundary", "Raise concerns as calls", "Send a concern to the Session as a message", "Let a person send a judge by hand"],
  },
  {
    id: "p6",
    agent: "rust-engineer",
    title: "Remove Jobs",
    paths: ["crates/api", "apps/desktop/src/renderer/src/jobs"],
    tasks: ["Take the Job routes out of the API", "Delete the Jobs page and its tab"],
  },
];

/** Where each group's tasks stand at each moment of the build, a letter a task. The first is as it is approved; two more follow. */
const BIG_STATES: readonly (readonly string[])[] = [
  ["ddfdhwoo", "ddwo", "dwoo", "woo", "oooo", "oo"],
  ["ddfddwwo", "dddw", "ddwo", "wwo", "owoo", "oo"],
  ["ddfdddwo", "dddd", "dddw", "dwo", "dwoo", "wo"],
];
const LETTER: Record<string, WorkTaskState> = { o: "open", w: "working", h: "handed_in", d: "done", f: "failed" };

/** The plan's groups at a moment: `at` indexes `BIG_STATES`, and none draws every task open. */
function bigGroups(at: number | undefined): WorkGroup[] {
  let n = 0;
  return BIG_GROUPS.map((one, g) => ({
    id: one.id,
    agent: one.agent,
    title: one.title,
    paths: one.paths,
    tasks: one.tasks.map((text, i) => {
      n += 1;
      const state = at === undefined ? "open" : LETTER[BIG_STATES[at]![g]![i]!]!;
      return task(`B${n}`, text, state);
    }),
  }));
}

const BIG_SKETCH_ID = "sketch:Plan";
const BIG_SKETCH = scene(
  [
    box("tool", 0, 0, "Fleet", "record_plan tool, the plan on the record", { icon: "server", step: 1 }),
    box("wire", 340, 0, "Wire", "Plan and task state on the reply and the event", { icon: "webhook", step: 2 }),
    box("panel", 680, -70, "Plan panel", "Groups tick as events arrive", { icon: "layers", step: 3 }),
    box("card", 680, 70, "Thread card", "Approve, or open to review", { icon: "scroll-text", step: 3 }),
    box("judge", 340, 150, "Judge", "Reads at each step boundary", { icon: "scale", step: 4 }),
  ],
  [arrow("tool", "wire", "plan", undefined, 2), arrow("wire", "panel", "events", undefined, 3), arrow("wire", "card", "plan", undefined, 3), arrow("judge", "tool", "concerns", undefined, 4)],
);

const big: Session = {
  id: BIG,
  title: "Fold Jobs into Sessions",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurn: "14:05",
  lastTurnAt: ago(1),
  standing: "idle",
  rows: [
    you("g-1", "14:03:00", "Fold Jobs into Sessions. A Session plans, he approves, and its work is the Job."),
    agent("g-2", "14:05:10", "The plan is recorded: twenty-five tasks in six groups. Approve it, or tell me what to change."),
  ],
  attachments: [{ kind: "sketch", id: BIG_SKETCH_ID, title: "Plan", by: "agent", drawing: BIG_SKETCH }],
  work: {
    workflow: { ...FEATURE, at: 0 },
    plan: { scope: ["crates/fleet", "crates/ipc", "packages/protocol", "apps/desktop"], approved: false, after: "g-2", sketch: { id: BIG_SKETCH_ID, title: "Plan", scene: BIG_SKETCH }, groups: bigGroups(undefined) },
  },
};

/** The shape one real Session had pending: four groups, ten tasks, ten scope paths, and the person's own message last. */
const live: Session = {
  id: LIVE,
  title: "Queue session for later",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurn: "14:09",
  lastTurnAt: ago(2),
  standing: "idle",
  rows: [
    you("l-1", "14:03:00", "Queue a Session for later, so it starts when a bay is free."),
    agent("l-2", "14:05:10", "The plan is recorded: ten tasks in four groups. Approve it, or tell me what to change."),
    you("l-3", "14:08:40", "Update the plan: the queue should survive a restart, and say so in the first group."),
  ],
  attachments: [
    { kind: "slot", slot: 9 },
    { kind: "branch", name: "bridge/queue-for-later", slot: 9 },
    { kind: "artifact", form: "window", id: "http://localhost:40108/?walk=queue-for-later", title: "Queue for later walk" },
  ],
  work: {
    workflow: { ...FEATURE, at: 0 },
    plan: {
      version: 2,
      scope: ["crates/fleet/src/queue.rs", "crates/fleet/src/pool.rs", "crates/store/src/queue.rs", "crates/ipc/src/queue.rs", "crates/ipc/operations/queue_session.toml", "packages/protocol/src/queue.ts", "apps/desktop/src/main/queue.ts", "apps/desktop/src/renderer/src/queue.tsx", "apps/desktop/src/renderer/src/dispatch-panel", "docs/concepts/session.md"],
      approved: false,
      after: "l-2",
      groups: [
        { id: "q1", agent: "rust-engineer", title: "Keep the queue in the store", paths: ["crates/store/src/queue.rs", "crates/fleet/src/queue.rs"], tasks: [task("Q1", "Add the queue table", "open"), task("Q2", "Read it back when Fleet starts", "open"), task("Q3", "Take a Session off it when a bay is free", "open")] },
        { id: "q2", agent: "protocol-engineer", title: "Queue operation on the wire", paths: ["crates/ipc/src/queue.rs", "crates/ipc/operations/queue_session.toml", "packages/protocol/src/queue.ts"], tasks: [task("Q4", "Add queue_session", "open"), task("Q5", "Mirror it in the TypeScript wire", "open")] },
        { id: "q3", agent: "bridge-engineer", title: "Queue from the dispatch panel", paths: ["apps/desktop/src/main/queue.ts", "apps/desktop/src/renderer/src/queue.tsx", "apps/desktop/src/renderer/src/dispatch-panel"], tasks: [task("Q6", "Offer Queue beside Start", "open"), task("Q7", "Show a queued Session in the list", "open"), task("Q8", "Let him take one off the queue", "open")] },
        { id: "q4", agent: "general-purpose", title: "Document the queue", paths: ["docs/concepts/session.md", "crates/fleet/src/pool.rs"], tasks: [task("Q9", "Write the queue's place in the Session concept", "open"), task("Q10", "Say what a restart keeps", "open")] },
      ],
    },
  },
  waitingFor: [
    { id: "w-1", text: "Restart onto the preview", since: ago(3), source: "agent", act: { kind: "run", target: "scripts/preview --restart" } },
    { id: "w-2", text: "Run the tests", since: ago(2), source: "agent", act: { kind: "run", target: "pnpm test" } },
  ],
};

// ---- The floor and the drones over it ---------------------------------------------------------------------

const F = "crates/fleet/src/";
const I = "crates/ipc/src/";
const P = "packages/protocol/src/";
const D = "apps/desktop/src/renderer/src/cockpit/deck/";

/** The files the plots hold before the work starts, each drawn as a bar. */
const EXISTING = [
  ...["lease.rs", "pool.rs", "bay.rs", "slots.rs", "lib.rs", "clock.rs", "errors.rs"].map((name) => F + name),
  ...["lease.rs", "wire.rs", "reply.rs", "lib.rs"].map((name) => I + name),
  ...["index.ts", "bays.ts", "wire.ts", "errors.ts"].map((name) => P + name),
  ...["BayRows.tsx", "PoolSlots.tsx", "PoolSlots.css", "Deck.tsx", "Incoming.tsx", "Ledger.tsx"].map((name) => D + name),
];

type Beat = Partial<Record<"main" | "rust" | "protocol" | "bridge", readonly [path: string, doing: WorkDrone["doing"], act?: WorkFile["act"]]>>;

/** Where each drone is over the floor, beat by beat, and what it does to the file there. The beats of a stage are the drones travelling. */
const BEATS: Record<number, readonly Beat[]> = {
  0: [{ main: [F + "lease.rs", "read"] }, { main: [F + "pool.rs", "read"] }, { main: [F + "bay.rs", "read"] }, { main: [F + "slots.rs", "read"] }],
  1: [
    { main: [F + "pool.rs", "read"], rust: [F + "lease.rs", "write", "edit"], protocol: [I + "wire.rs", "read"] },
    { main: [F + "bay.rs", "read"], rust: [F + "retry.rs", "write", "write"], protocol: [I + "lease.rs", "write", "edit"] },
    { main: [F + "lease.rs", "read"], rust: [F + "backoff.rs", "write", "write"], protocol: [P + "lease.ts", "write", "write"] },
    { main: [F + "slots.rs", "read"], rust: [F + "lease.rs", "write", "edit"], protocol: [P + "index.ts", "write", "edit"] },
  ],
  2: [
    { main: [F + "pool.rs", "read"], rust: [F + "backoff.rs", "write", "edit"], bridge: [D + "BayRows.tsx", "write", "edit"] },
    { main: [F + "lease.rs", "read"], rust: [F + "slots.rs", "write", "remove"], bridge: [D + "PoolSlots.tsx", "write", "edit"] },
    { main: [F + "bay.rs", "read"], rust: [F + "lib.rs", "write", "edit"], bridge: [D + "PoolSlots.css", "write", "edit"] },
    { main: [F + "pool.rs", "read"], rust: [F + "retry.rs", "write", "edit"], bridge: [D + "BayTile.tsx", "write", "write"] },
  ],
  3: [
    { main: [F + "lease.rs", "run"], bridge: [D + "BayTile.tsx", "write", "edit"] },
    { main: [F + "retry.rs", "run"], bridge: [D + "BayRows.tsx", "write", "edit"] },
    { main: [F + "lease.rs", "run"], bridge: [D + "PoolSlots.css", "write", "edit"] },
  ],
  4: [{ main: [F + "lease.rs", "run"] }, { main: [F + "retry.rs", "run"] }],
};

const AGENTS = {
  main: {},
  rust: { agent: "rust-engineer", task: "Retry the lease when the pool is contended" },
  protocol: { agent: "protocol-engineer", task: "Add attempts to the lease reply" },
  bridge: { agent: "bridge-engineer", task: "Show the attempt on the bay tile" },
} as const;

/** One beat as the Field draws it: each drone at its file, and the files with what is being done to them. `made` and `gone` are the files the work has added and taken away so far. */
function fieldAt(beat: Beat, made: ReadonlySet<string>, gone: ReadonlySet<string>, again: boolean): { field: WorkDrone[]; files: WorkFile[] } {
  const field = (Object.keys(AGENTS) as (keyof typeof AGENTS)[]).flatMap((id): WorkDrone[] => {
    const at = beat[id];
    return at === undefined ? [] : [{ id, ...AGENTS[id], path: at[0].split("/").slice(0, 2).join("/"), doing: at[1], file: at[0] }];
  });
  // On a second time round a made file is only changed, and a file already taken away stays gone.
  const acts = new Map(Object.values(beat).flatMap((at) => (at?.[2] === undefined || (again && at[2] === "remove") ? [] : [[at[0], at[2] === "write" && again ? "edit" : at[2]] as const])));
  const files = [...EXISTING, ...made].filter((path) => !gone.has(path) || acts.get(path) === "remove").map((path): WorkFile => (acts.has(path) ? { path, act: acts.get(path)! } : { path }));
  return { field, files };
}

const CONCERNS: WorkConcern[] = [
  { id: "c1", step: "Build", judge: "Judge", text: "The retry reads the pool without holding the lease lock, so two Sessions can still take one bay.", cites: ["crates/fleet/src/lease.rs"] },
  { id: "c2", step: "Build", judge: "Judge", text: "The back off has no ceiling.", cites: ["crates/fleet/src/backoff.rs"] },
];

/** The sketch the agent draws with its plan: the change wired together, in the format `show_sketch` takes. */
const PLAN_SKETCH_ID = "sketch:Plan";
const PLAN_SKETCH = scene(
  [
    box("fleet", 0, 0, "Fleet", "Lease retry:\nback off and ask again when the pool is contended", { icon: "server", step: 1 }),
    box("wire", 360, 0, "Wire", "Attempt count on the lease reply", { icon: "webhook", step: 2 }),
    box("bridge", 720, 0, "Bridge", "Bay tile shows the attempt", { icon: "globe", step: 3 }),
  ],
  [arrow("fleet", "wire", "attempts", undefined, 2), arrow("wire", "bridge", "lease reply", undefined, 3)],
);

/** What a judge sent by hand finds the first time: one concern on the build so far. A second read finds nothing. */
const SENT_CONCERN: WorkConcern = { id: "j1", judge: "Judge", text: "The lease is renewed after the retry, so a slow attempt can outlive it.", cites: ["crates/fleet/src/lease.rs"] };
const READ_MS = 2200;

const REPLIES: Record<string, string> = {
  c1: "The read now happens under the lease lock.",
  c2: "The back off stops at two seconds.",
};

const QUESTIONS = [
  {
    question: "How many attempts before it gives up?",
    header: "Attempts",
    multi_select: false,
    options: [
      { label: "Three", description: "Gives up after the third try" },
      { label: "Five", description: "Gives up after the fifth try" },
    ],
  },
];

// ---- The store -------------------------------------------------------------------------------------------

function sessions(): SessionsStore {
  const base = sessionsStore([dispatched, quiet, busy, big, live]);
  // The mock's first start is a fixture Session with a script of its own; it is spent here and kept off the ship.
  base.start();
  revision.on = false;
  const own = new Set([MINE, QUIET, BUSY, BIG, LIVE]);
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  let stage = 0;
  let rowId = 0;
  /** How many times a judge has been sent by hand. */
  let judged = 0;
  let clock = 2 * 60 + 2;
  const timers = new Set<number>();
  const stamp = () => {
    clock += 1;
    return `14:${String(Math.floor(clock / 60) % 60).padStart(2, "0")}:${String(clock % 60).padStart(2, "0")}`;
  };
  const next = <T extends SessionRow>(make: (id: string, at: string) => T): T => make(`v-r${(rowId += 1)}`, stamp());
  const said = (text: string) => next((id, at) => agent(id, at, text));
  const asked = (text: string) => next((id, at) => you(id, at, text));
  const tool = (text: string): SessionRow => next((id, at) => ({ id, at, kind: "tool", text }));
  const mine = () => base.get().find((one) => one.id === MINE);
  const change = (edit: (one: Session) => Session) => base.edit(MINE, edit);
  const works = (edit: (work: SessionWork) => SessionWork) => change((one) => ({ ...one, work: edit(one.work ?? {}) }));
  const push = (rows: SessionRow[], more: Partial<Session> = {}) => change((one) => ({ ...one, ...more, rows: [...one.rows, ...rows] }));
  const after = (ms: number, run: () => void) => {
    const handle = window.setTimeout(() => {
      timers.delete(handle);
      run();
    }, ms);
    timers.add(handle);
  };
  const idle = (): Partial<Session> => ({ turn: { state: "idle" }, standing: "replied", lastTurnAt: new Date().toISOString() });
  const working = (): Partial<Session> => ({ turn: { state: "working" }, standing: "working" });

  // The drones travel: each beat of a stage puts every drone over another file, a moment apart, and the stage goes round again until the next.
  const BEAT_MS = 2600;
  const made = new Set<string>();
  const gone = new Set<string>();
  let tour = 0;
  const fly = (at: number) => {
    const beats = BEATS[at] ?? BEATS[4]!;
    const mark = (tour += 1);
    let step = 0;
    let removing: string | undefined;
    const go = () => {
      if (mark !== tour) return;
      const lap = Math.floor(step / beats.length);
      const beat = beats[step % beats.length]!;
      if (removing !== undefined) gone.add(removing);
      removing = undefined;
      for (const one of Object.values(beat)) {
        if (one?.[2] === "write" && lap === 0) made.add(one[0]);
        if (one?.[2] === "remove") removing = one[0];
      }
      const drawn = fieldAt(beat, made, gone, lap > 0);
      works((work) => ({ ...work, field: drawn.field, files: drawn.files }));
      step += 1;
      after(BEAT_MS, go);
    };
    go();
  };

  /** The build at `at`: the groups, the drones, and the step the workflow is at. */
  const building = (at: number) => {
    stage = at;
    fly(at);
    works((work) => ({
      ...work,
      ...(work.plan === undefined ? {} : { plan: { ...work.plan, groups: groups(at) } }),
      ...(work.workflow === undefined ? {} : { workflow: { ...work.workflow, at: at >= 4 ? work.workflow.steps.length - 1 : 1 } }),
    }));
  };

  // The agent begins as the proposal lands: the first step is lit, it reads, and it asks before it plans. It starts when the Session is first opened.
  let begun = false;
  /** Model and effort changes in the composer wake nobody: they wait here and the next turn is told, as a row where it starts. */
  const owed = new Map<string, { from: string; to: string }>();
  const told = (): SessionRow[] => {
    const rows = [...owed].map(([what, one]) => next((id, at): SessionRow => ({ id, at, kind: "change", what, from: one.from, to: one.to })));
    owed.clear();
    return rows;
  };
  const begin = () => {
    if (begun) return;
    begun = true;
    works((work) => (work.workflow === undefined ? work : { ...work, workflow: { ...work.workflow, at: 0 } }));
    fly(0);
    push([tool("Read crates/fleet/src/lease.rs"), said("Two Sessions can ask for the same bay between the read and the write. One question before I plan.")], {
      asked: { command: QUESTIONS[0]!.question, call: "v-q1", offers: ["allow_once", "refuse"], questions: QUESTIONS },
    });
  };

  // The long plan, approved: the agent is mid-build and groups tick as the walk lets time pass.
  let bigAt = 0;
  const bigPlan = (edit: (plan: NonNullable<SessionWork["plan"]>) => NonNullable<SessionWork["plan"]>) =>
    base.edit(BIG, (one) => ({ ...one, work: { ...one.work, ...(one.work?.plan === undefined ? {} : { plan: edit(one.work.plan) }) } }));
  const approveBig = () => {
    bigAt = 0;
    bigPlan((plan) => ({ ...plan, approved: true, groups: bigGroups(0) }));
    base.edit(BIG, (one) => ({
      ...one,
      ...working(),
      work: { ...one.work, workflow: one.work?.workflow === undefined ? { ...FEATURE, at: 1 } : { ...one.work.workflow, at: 1 } },
      rows: [...one.rows, you("g-3", "14:05:40", "Approved the plan."), agent("g-4", "14:05:42", "Starting with the plan tool in Fleet. The wire and the panel follow.")],
    }));
  };

  const approveMine = () => {
    works((work) => (work.plan === undefined ? work : { ...work, plan: { ...work.plan, approved: true } }));
    push(
      [
        asked("Approved the plan."),
        said("Starting with the lease in Fleet. The wire and the bay tile follow."),
        tool("Edit crates/fleet/src/lease.rs"),
        next((id, at) => ({ id, at, kind: "lease", slot: 3, branch: BRANCH })),
      ],
      { ...working(), attachments: [...(mine()?.attachments ?? []), { kind: "branch", name: BRANCH, slot: 3 }] },
    );
    building(1);
  };

  return {
    ...base,
    // Only the menu's Retro item is drawn here; the press writes nothing.
    retro: async () => true,
    tune: (id, tuning) => {
      const now = id === MINE ? mine() : undefined;
      if (now !== undefined) {
        for (const [what, was, to] of [["Model", now.model ?? "default", tuning.model ?? "default"], ["Effort", now.effort ?? "default", tuning.effort ?? "default"]] as const) {
          const first = owed.get(what)?.from ?? was;
          if (first === to) owed.delete(what);
          else owed.set(what, { from: first, to });
        }
      }
      return base.tune(id, tuning);
    },
    watch: (id) => {
      base.watch?.(id);
      if (id === MINE) begin();
    },
    get: () => {
      const all = base.get();
      if (all !== from) {
        from = all;
        only = all.filter((one) => own.has(one.id));
      }
      return only;
    },
    work: {
      workflows: WORKFLOWS,
      // The proposal is read again: a different workflow draws a different rail, and none draws no rail at all.
      choose: (_id, name) => {
        const chosen = WORKFLOWS.find((one) => one.id === name);
        const was = mine()?.work?.proposal?.workflow ?? null;
        works((work) => {
          const { workflow, ...rest } = work;
          void workflow;
          return { ...rest, proposal: { workflow: name, ...(work.proposal?.picked === undefined ? {} : { picked: work.proposal.picked.filter((one) => one !== "workflow") }) }, ...(chosen === undefined ? {} : { workflow: { ...chosen, at: work.workflow?.at ?? 0 } }) };
        });
        // The agent is told at once: a row in his colour, and it adjusts to the steps it will now follow.
        if (begun && name !== was) {
          push([next((id, at) => ({ id, at, kind: "change", what: "Workflow", from: was ?? "None", to: name ?? "None" }))]);
          after(700, () => push([said(chosen === undefined ? "No workflow now." : `Following ${chosen.name}: ${chosen.steps.join(", ")}.`)]));
        }
      },
      unpick: (_id, value) =>
        works((work) => (work.proposal?.picked === undefined ? work : { ...work, proposal: { ...work.proposal, picked: work.proposal.picked.filter((one) => one !== value) } })),
      approve: (id) => {
        if (id === BIG) return approveBig();
        if (id === LIVE) return base.edit(LIVE, (one) => ({ ...one, work: { ...one.work, ...(one.work?.plan === undefined ? {} : { plan: { ...one.work.plan, approved: true } }) } }));
        approveMine();
      },
      // The marks he put on the plan, sent as one message. The agent answers each: it takes some, pushes back on one with a reason,
      // and records the plan again with what changed marked. Once the plan is approved the answer is a line and the build goes on.
      suggest: (id, marks, approving) => {
        if (id !== MINE) return;
        const plan = mine()?.work?.plan;
        if (plan === undefined) return;
        push([next((rid, at): SessionRow => ({ id: rid, at, kind: "suggest", lines: linesOf(plan, marks) }))], working());
        if (approving === true) {
          approveMine();
          after(700, () => push([said("Taking those in as I start.")], working()));
        } else if (plan.approved) {
          after(700, () => push([said("Noted. I will fold those into the tasks still open.")], working()));
        } else {
          after(800, () =>
            push(
              [
                said(
                  "Taking the rename, and dropping Back off between attempts: the pool's own backoff covers it. Keeping a new field on the wire: nothing existing carries the attempt count, so reusing one would be a guess. That adds a task for the protocol ID. Recording the plan again.",
                ),
              ],
              idle(),
            ),
          );
          after(2200, () => {
            revision.on = true;
            const note = said("The plan is recorded again with three changes. Approve it, or tell me what to change.");
            works((work) => (work.plan === undefined ? work : { ...work, plan: { ...work.plan, groups: groups(0), approved: false, changes: CHANGES, after: note.id } }));
            push([note], idle());
          });
        }
      },
      sendConcern: (_id, concern) => {
        const one = mine()?.work?.concerns?.find((it) => it.id === concern);
        if (one === undefined) return;
        works((work) => ({ ...work, concerns: (work.concerns ?? []).filter((it) => it.id !== concern) }));
        push([asked(`${one.step ?? "Judge tile"}: ${one.text}`)], working());
        after(700, () => push([tool("Edit crates/fleet/src/lease.rs"), said(REPLIES[concern] ?? "Noted.")], idle()));
      },
      judge: async () => {
        works((work) => ({ ...work, judging: true }));
        after(READ_MS, () => {
          const found = judged === 0;
          judged += 1;
          works((work) => {
            const { judging: was, ...rest } = work;
            void was;
            return { ...rest, judged: { at: new Date().toISOString(), concerns: found ? 1 : 0 }, ...(found ? { concerns: [...(work.concerns ?? []), SENT_CONCERN] } : {}) };
          });
        });
      },
      dismissConcern: (_id, concern) => {
        const one = mine()?.work?.concerns?.find((it) => it.id === concern);
        if (one === undefined) return;
        // Folded to a settled line in the thread. The agent is told nothing.
        works((work) => ({ ...work, concerns: (work.concerns ?? []).filter((it) => it.id !== concern) }));
        push([next((id, at): SessionRow => ({ id, at, kind: "concern", step: one.step ?? "Judge", text: one.text }))]);
      },
    },
    // The question the agent asked is answered, and it goes on to record the plan.
    answer: (_id, answer, answers) => {
      change((one) => {
        const { asked: was, ...rest } = one;
        void was;
        return rest;
      });
      if (answer === "refuse" || (answers ?? []).length === 0) return push([...told(), said("Taking three attempts.")], working());
      push([...told(), asked(answers!.map((one) => `**${one.question}**\n${one.chosen.join(", ")}`).join("\n\n")), said(`${answers!.map((one) => one.chosen.join(" and ")).join(", ")} attempts. Writing the plan.`)], working());
    },
    // Words he types go to the agent, which answers in a line.
    send: (id, sent) => {
      const done = base.send(id, sent);
      if (id === MINE) {
        const rows = told();
        if (rows.length > 0) push(rows);
        after(700, () => push([said("Noted.")], idle()));
      }
      return done;
    },
    later() {
      if (base.get().find((one) => one.id === BIG)?.work?.plan?.approved === true && bigAt < BIG_STATES.length - 1) {
        bigAt += 1;
        const at = bigAt;
        bigPlan((plan) => ({ ...plan, groups: bigGroups(at) }));
        return;
      }
      const now = mine();
      if (now === undefined) return;
      const plan = now.work?.plan;
      if (plan === undefined) {
        // The agent records its plan after planning with him, and it arrives for his approval.
        const note = said("The plan is recorded: four tasks in three groups. Approve it, or tell me what to change.");
        works((work) => ({ ...work, plan: { scope: SCOPE, groups: groups(0), approved: false, after: note.id, sketch: { id: PLAN_SKETCH_ID, title: "Plan", scene: PLAN_SKETCH } } }));
        push([note], { ...idle(), attachments: [...(now.attachments ?? []), { kind: "sketch", id: PLAN_SKETCH_ID, title: "Plan", by: "agent", drawing: PLAN_SKETCH }] });
        return;
      }
      if (!plan.approved) return;
      if (stage === 1) {
        building(2);
        push([tool("Edit crates/ipc/src/lease.rs"), tool("Edit apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx")], working());
      } else if (stage === 2) {
        building(3);
        // A judge reads the Build step and raises two concerns. Neither blocks: the agent goes on.
        works((work) => ({ ...work, concerns: CONCERNS }));
        // At the same moment another Session asks for something: its call is the Cockpit's, and the concerns above are not.
        base.edit(QUIET, (one) => ({ ...one, asked: { command: "git push -u origin fix/migration-names", call: "v-q2", offers: ["allow_once", "refuse"] } }));
      } else if (stage === 3) {
        building(4);
        push(
          [tool("Bash armada check test lease"), said("The checks pass on the branch. The change is ready for a pull request.")],
          {
            ...idle(),
            attachments: [
              ...(now.attachments ?? []).filter((one) => one.kind === "sketch"),
              { kind: "branch", name: BRANCH, slot: 3 },
              {
                kind: "pull_request",
                number: 2390,
                title: "Retry the slot lease",
                branch: BRANCH,
                address: "https://git.example/armada/pull/2390",
                checks: { state: "passed" },
                state: "draft",
                auto: false,
              },
            ],
          },
        );
      }
    },
    dispose: () => {
      timers.forEach((one) => window.clearTimeout(one));
      base.dispose();
    },
    start: (tag, root) => {
      const id = base.start(tag, root);
      if (typeof id === "string") {
        own.add(id);
        base.edit(id, (one) => ({ ...one, manifestId: OWNER }));
      }
      return id;
    },
  };
}

// ---- The Cockpit round it --------------------------------------------------------------------------------

const bay = (slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({ manifest_id: OWNER, slot, path: `${ROOT}/.armada/slots/slot-${slot}`, base: "main", warm: true, behind: 0, held, ...rest });

function pool(): WorktreesHeld {
  return {
    slots: [
      bay(1, { state: "hosted_session", session_id: BUSY, session_title: busy.title! }, { branch: "armada/audit-rotation", since: ago(35) }),
      bay(2, { state: "hosted_session", session_id: QUIET, session_title: quiet.title! }, { branch: "fix/migration-names", since: ago(90) }),
      bay(3, { state: "free" }),
      bay(4, { state: "free" }),
    ],
  };
}

let count = 0;
const event = (minutes: number, session: Session, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, detail?: string): TimelineEvent => {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: { kind: "session", id: session.id }, title: session.title!, tone, says, manifest_id: OWNER, ...(detail === undefined ? {} : { detail }) };
};
const EVENTS: TimelineEvent[] = [
  event(35, busy, "session.started", "run", "Started"),
  event(35, busy, "slot.leased", "idle", "Slot 1 leased", "slot-1"),
  event(3, busy, "session.edited", "run", "Edited 1 file", "crates/store/src/audit.rs"),
  event(90, quiet, "session.started", "run", "Started"),
  event(22, quiet, "session.turn", "idle", "Replied"),
];

/** MINE edits a file QUIET also changes and two BUSY also changes: the header's clash, and the one bar under it that holds both. */
const CLASHES: readonly WriteClash[] = [
  { one: { kind: "session", id: MINE, label: dispatched.title! }, other: { kind: "session", id: QUIET, label: quiet.title! }, paths: ["crates/store/src/migrations.rs"], since: ago(3) },
  { one: { kind: "session", id: MINE, label: dispatched.title! }, other: { kind: "session", id: BUSY, label: busy.title! }, paths: ["crates/store/src/audit.rs", "crates/fleet/src/lease.rs"], since: ago(2) },
];

function build(): Scenario {
  const base = holding("one-session-view", "One view of a Session on the Cockpit and the Sessions page: plan and workflow, field, concerns, proposal");
  return {
    ...base,
    state: { ...base.state, repository: null },
    held: pool(),
    draft: { sessions, clashes: CLASHES, events: EVENTS, areas: () => mockAreas("/armada") },
  };
}

export const s261OneSessionView: Scenario = build();
