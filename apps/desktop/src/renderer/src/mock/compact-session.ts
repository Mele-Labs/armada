// A Session open with less on screen. One full Session in the middle of a build: a long thread with prose, tool calls, commits, checks, pages shown and
// two sketches; a plan with progress; three subagents, two working; a question waiting as the thread's last card; pull requests; a page
// published, files and a doc made; a command running in the background. Beside it, three other Sessions waiting on him, whose rows pulse. Time passing
// strands its bay: the Slot key lights and the thread gets a call.
// Over the Cockpit; the Session's `work` is draft data. Outside `scenarios/`, where every export is read as a row: the rows there are this, once for the
// walk `compactSessionView` and once for `compactCallsInThread`.

import type { TimelineEvent, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment, SessionRow, WorkConcern, WorkDrone, WorkFile, WorkGroup, WorkTask, WorkTaskState } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "./cockpit-areas";
import { holding } from "./holding";
import { planCall } from "./plan-call";
import type { Scenario } from "./moment";
import { arrow, box, scene } from "./scenes";
import { sessionsStore } from "./sessions/script";
import type { BayControl, SessionsStore } from "./sessions/script";
import { READS, READS_TITLE } from "./sessions/sketches";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const MAIN = "compact-1";
const PRUNE = "compact-2";
const NAMES = "compact-3";
const AUDIT = "compact-4";
const FOLD = "compact-5";
const LESSONS = "compact-6";
const BRANCH = "armada/lease-retry";
const SLOT = 3;

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });
const tool = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "tool", text });

// ---- The work -----------------------------------------------------------------------------------------------

const F = "crates/fleet/src/";
const I = "crates/ipc/src/";
const P = "packages/protocol/src/";
const D = "apps/desktop/src/renderer/src/cockpit/deck/";

const task = (id: string, text: string, state: WorkTaskState): WorkTask => ({ id, text, state, seen: state !== "open" });

/** The plan's groups at a beat of the build: 0 is where the thread stands, 1 is a moment later. */
function groups(beat: number): WorkGroup[] {
  return [
    {
      id: "g1",
      agent: "rust-engineer",
      title: "Retry the slot lease when the pool is contended",
      paths: ["crates/fleet"],
      tasks: [task("T1", "Retry the lease when the pool is contended", "done"), task("T2", "Back off between attempts", "done"), task("T3", "Stop after the attempt cap", beat === 0 ? "working" : "done")],
    },
    {
      id: "g2",
      agent: "protocol-engineer",
      title: "Attempt count on the wire",
      paths: ["crates/ipc", "packages/protocol"],
      tasks: [task("T4", "Add attempts to the lease reply", "done"), task("T5", "Mirror it in the TypeScript wire", beat === 0 ? "working" : "done"), task("T6", "Bump the protocol ID", "open")],
    },
    {
      id: "g3",
      agent: "bridge-engineer",
      title: "Attempt on the bay tile",
      paths: ["apps/desktop"],
      tasks: [task("T7", "Show the attempt on the bay tile", "working"), task("T8", "Draw the cap when it is reached", "open")],
    },
    {
      id: "g4",
      agent: "general-purpose",
      title: "Document the retry",
      paths: ["docs/concepts"],
      tasks: [task("T9", "Say what a retry keeps", "open")],
    },
  ];
}

type Beat = readonly (readonly [drone: "main" | "rust" | "protocol" | "bridge", path: string, doing: WorkDrone["doing"], act?: WorkFile["act"]])[];

const BEATS: readonly Beat[] = [
  [["main", F + "pool.rs", "read"], ["rust", F + "retry.rs", "write", "edit"], ["protocol", P + "lease.ts", "write", "write"], ["bridge", D + "BayRows.tsx", "write", "edit"]],
  [["main", F + "lease.rs", "read"], ["rust", F + "backoff.rs", "write", "edit"], ["protocol", I + "lease.rs", "write", "edit"], ["bridge", D + "PoolSlots.tsx", "write", "edit"]],
];

const AGENTS = {
  main: {},
  rust: { agent: "rust-engineer", task: "Stop after the attempt cap" },
  protocol: { agent: "protocol-engineer", task: "Mirror it in the TypeScript wire" },
  bridge: { agent: "bridge-engineer", task: "Show the attempt on the bay tile" },
} as const;

const EXISTING = [
  ...["lease.rs", "pool.rs", "bay.rs", "slots.rs", "lib.rs", "clock.rs"].map((name) => F + name),
  ...["lease.rs", "wire.rs", "reply.rs", "lib.rs"].map((name) => I + name),
  ...["index.ts", "bays.ts", "wire.ts", "errors.ts"].map((name) => P + name),
  ...["BayRows.tsx", "PoolSlots.tsx", "PoolSlots.css", "Deck.tsx", "Incoming.tsx"].map((name) => D + name),
];

/** Where the drones are at a beat, and the files with what has been done to them. */
function fieldAt(at: number): { field: WorkDrone[]; files: WorkFile[] } {
  const beat = BEATS[at % BEATS.length]!;
  const field = beat.map(([drone, path, doing]): WorkDrone => ({ id: drone, ...AGENTS[drone], path: path.split("/").slice(0, 2).join("/"), doing, file: path }));
  const made = [F + "retry.rs", F + "backoff.rs", P + "lease.ts"];
  const acts = new Map(beat.flatMap(([, path, , act]) => (act === undefined ? [] : [[path, act] as const])));
  const files: WorkFile[] = [...EXISTING, ...made].map((path) => (acts.has(path) ? { path, act: acts.get(path)! } : { path }));
  return { field, files };
}

const CONCERN: WorkConcern = { id: "c1", step: "Build", judge: "Judge", text: "The retry reads the pool without holding the lease lock, so two Sessions can still take one bay.", cites: ["crates/fleet/src/lease.rs"] };

const FLOW = { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"] } as const;

// ---- The Session ----------------------------------------------------------------------------------------------

const SKETCH_PLAN_ID = "sketch:Lease retry";
const SKETCH_PLAN = scene(
  [
    box("fleet", 0, 0, "Fleet", "Lease retry:\nback off and ask again when the pool is contended", { icon: "server", step: 1 }),
    box("wire", 360, 0, "Wire", "Attempt count on the lease reply", { icon: "webhook", step: 2 }),
    box("bridge", 720, 0, "Bridge", "Bay tile shows the attempt", { icon: "globe", step: 3 }),
  ],
  [arrow("fleet", "wire", "attempts", undefined, 2), arrow("wire", "bridge", "lease reply", undefined, 3)],
);

const main: Session = {
  id: MAIN,
  title: "Retry the slot lease",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurn: "14:12",
  lastTurnAt: ago(1),
  standing: "idle",
  context: { tokens: 128_000, window: 400_000 },
  rows: [
    you("m-1", "13:20:00", "Make the slot lease retry when two Sessions ask for the same bay at once."),
    tool("m-2", "13:20:12", "Read crates/fleet/src/lease.rs"),
    tool("m-3", "13:20:13", "Grep lease_slot crates/"),
    tool("m-4", "13:20:16", "Read crates/fleet/src/pool.rs"),
    agent(
      "m-5",
      "13:21:02",
      "Two Sessions can ask for the same bay between the read and the write. The second one is refused and gives up at once.\n\nThe fix is to ask again after a short wait, up to a cap, and to tell the caller how many tries it took.\n\n- **Retry** the lease when the pool is contended\n- **Back off** between attempts, with a ceiling\n- **Report** the attempt count on the lease reply",
    ),
    you("m-6", "13:22:40", "Keep the attempt count on the lease reply so the bay tile can show it."),
    agent("m-7", "13:23:05", "Recorded the plan: nine tasks in four groups. The attempt count is its own task in the wire group."),
    you("m-8", "13:23:30", "Approved the plan."),
    agent("m-9", "13:23:34", "Starting with the lease in Fleet. The wire and the bay tile follow."),
    tool("m-10", "13:23:50", "Edit crates/fleet/src/lease.rs"),
    { id: "m-11", at: "13:23:51", kind: "lease", slot: SLOT, branch: BRANCH },
    tool("m-12", "13:24:20", "Write crates/fleet/src/retry.rs"),
    tool("m-13", "13:24:44", "Write crates/fleet/src/backoff.rs"),
    tool("m-14", "13:25:10", "Edit crates/fleet/src/pool.rs"),
    tool("m-15", "13:26:02", "Bash armada check test lease"),
    { id: "m-16", at: "13:26:40", kind: "check", name: "fleet", run: 4410, state: "passed" },
    tool("m-17", "13:27:01", 'Bash git commit -m "Retry the slot lease when the pool is contended"'),
    agent("m-18", "13:27:30", "The retry is in Fleet and its checks pass. Moving to the wire."),
    tool("m-19", "13:28:10", "Edit crates/ipc/src/lease.rs"),
    tool("m-20", "13:28:30", "Write packages/protocol/src/lease.ts"),
    tool("m-21", "13:29:02", "Bash git commit -m \"Carry the attempt count on the lease reply\""),
    agent("m-22", "13:30:15", "I drew how the lease flows now."),
    tool("m-23", "13:30:16", `show_sketch ${JSON.stringify({ title: "Lease retry", scene: SKETCH_PLAN })}`),
    { id: "m-24", at: "13:31:00", kind: "window", title: "Bay tile with the attempt", url: "http://127.0.0.1:5176/?walk=bay-attempt" },
    you("m-25", "13:32:10", "The tile shows the attempt but not the cap. Add the cap when it is reached."),
    agent("m-26", "13:32:40", "Adding the cap to the tile. I will hand the wire types to a subagent while I draw it."),
    tool("m-27", "13:33:05", "Agent protocol-engineer: Mirror the attempt count in the TypeScript wire"),
    tool("m-28", "13:33:09", "Agent bridge-engineer: Show the attempt on the bay tile"),
    tool("m-29", "13:34:20", "Edit apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx"),
    tool("m-30", "13:35:02", "Read packages/protocol/src/lease.ts"),
    { id: "m-31", at: "13:36:10", kind: "ran", command: "git status --short", code: 0, tail: " M crates/fleet/src/lease.rs\n M crates/fleet/src/pool.rs\n?? crates/fleet/src/retry.rs" },
    tool("m-32", "13:37:30", "Bash armada check test bay"),
    { id: "m-33", at: "13:38:20", kind: "check", name: "desktop", run: 4417, state: "running" },
    tool("m-34", "13:39:00", 'Bash git commit -m "Show the attempt on the bay tile"'),
    agent(
      "m-35",
      "13:40:12",
      "The change is on the branch and the pull request is open. Checks on the desktop are still running.\n\nOne question before I raise the attempt cap: the tile has room for three or five tries.",
    ),
  ],
  attachments: [
    { kind: "slot", slot: SLOT },
    { kind: "branch", name: BRANCH, slot: SLOT },
    { kind: "pull_request", number: 2381, title: "Name the lease record", branch: "armada/lease-record", address: "https://git.example/armada/pull/2381", checks: { state: "passed" }, state: "merged", auto: false },
    { kind: "pull_request", number: 2390, title: "Retry the slot lease", branch: BRANCH, address: "https://git.example/armada/pull/2390", checks: { state: "pending" }, state: "open", auto: false },
    { kind: "sketch", id: SKETCH_PLAN_ID, title: "Lease retry", by: "agent", drawing: SKETCH_PLAN },
    { kind: "sketch", id: `sketch:${READS_TITLE}`, title: READS_TITLE, by: "agent", drawing: READS },
    { kind: "artifact", form: "window", id: "http://127.0.0.1:5176/?walk=bay-attempt", title: "Bay tile with the attempt" },
    { kind: "artifact", form: "window", id: "http://127.0.0.1:5176/?walk=lease-retry-cap", title: "Cap on the bay tile" },
    { kind: "artifact", form: "page", id: "https://notes.example/armada/lease-retry", title: "Lease retry notes" },
    { kind: "artifact", form: "file", id: "docs/spikes/lease-contention.md", title: "Lease contention measured" },
    { kind: "artifact", form: "file", id: "/tmp/armada/lease-trace.txt", title: "Lease trace" },
    { kind: "artifact", form: "doc", id: "https://docs.example/d/lease-retry", title: "Lease retry design" },
    { kind: "artifact", form: "image", id: "/tmp/armada/bay-tile.png", title: "Bay tile before" },
    { kind: "subagent", id: "sa-1", task: "Mirror the attempt count in the TypeScript wire", state: "running", agentType: "protocol-engineer" },
    { kind: "subagent", id: "sa-2", task: "Show the attempt on the bay tile", state: "running", agentType: "bridge-engineer" },
    { kind: "subagent", id: "sa-3", task: "Find every caller of lease_slot", state: "done", agentType: "Explore", report: "Eleven callers in crates/fleet and crates/drone. Only the pool's reads the reply." },
    { kind: "background", id: "bg-1", form: "command", task: "Serve the mock", command: "pnpm --filter @armada/desktop mock", started: ago(24), state: "running" },
    { kind: "background", id: "bg-2", form: "command", task: "Run the lease tests", command: "armada check test lease", started: ago(12), ended: ago(11), state: "done", exit: 0 },
  ] satisfies SessionAttachment[],
  waitingFor: [{ id: "w-pr", text: "Approve #2390", since: ago(2), source: "agent", act: { kind: "approve_pr", target: "2390" } }],
  asked: {
    command: "How many attempts before it gives up?",
    call: "m-q1",
    offers: ["allow_once", "refuse"],
    questions: [
      {
        question: "How many attempts before it gives up?",
        header: "Attempts",
        multi_select: false,
        options: [
          { label: "Three", description: "Gives up after the third try" },
          { label: "Five", description: "Gives up after the fifth try" },
        ],
      },
    ],
  },
  work: {
    workflow: { ...FLOW, steps: [...FLOW.steps], at: 1 },
    plan: { version: 1, scope: ["crates/fleet", "crates/ipc", "packages/protocol", "apps/desktop"], approved: true, after: "m-7", sketch: { id: SKETCH_PLAN_ID, title: "Lease retry", scene: SKETCH_PLAN }, groups: groups(0) },
    ...fieldAt(0),
    concerns: [CONCERN],
  },
};

// ---- The other Sessions: three wait on him, two do not ----------------------------------------------------------

const other = (id: string, title: string, rest: Partial<Session> & Pick<Session, "rows">): Session => ({
  id,
  title,
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurn: "13:50",
  lastTurnAt: ago(20),
  standing: "idle",
  attachments: [],
  ...rest,
});

const prune = other(PRUNE, "Prune the stale branches", {
  rows: [you("p-1", "13:30:00", "Prune the branches that are merged and a month old."), agent("p-2", "13:31:10", "Eleven branches are merged and old enough. Deleting them needs your word.")],
  asked: { command: "git push origin --delete armada/old-retry armada/old-lease", call: "p-q1", offers: ["allow_once", "refuse"] },
});
const names = other(NAMES, "Name the store migrations", {
  rows: [you("n-1", "13:10:00", "Name the store migrations by their order."), agent("n-2", "13:41:10", "The migrations are named. The pull request is ready for you.")],
  attachments: [{ kind: "branch", name: "fix/migration-names", slot: 2 }, { kind: "pull_request", number: 2377, title: "Name the store migrations", branch: "fix/migration-names", address: "https://git.example/armada/pull/2377", checks: { state: "passed" }, state: "open", auto: false }],
  waitingFor: [{ id: "w-names", text: "Approve #2377", since: ago(18), source: "agent", act: { kind: "approve_pr", target: "2377" } }],
});
const audit = other(AUDIT, "Rotate the audit log", {
  turn: { state: "working" },
  standing: "working",
  lastTurnAt: ago(1),
  rows: [you("a-1", "13:50:00", "Rotate the audit log at midnight."), tool("a-2", "14:00:05", "Edit crates/store/src/audit.rs")],
  attachments: [{ kind: "branch", name: "armada/audit-rotation", slot: 1 }],
});
const fold = other(FOLD, "Fold Jobs into Sessions", {
  waitingFor: [planCall(undefined, ago(14))],
  rows: [you("f-1", "14:03:00", "Fold Jobs into Sessions."), agent("f-2", "14:05:10", "The plan is recorded: four tasks in two groups. Approve it, or tell me what to change.")],
  work: {
    workflow: { ...FLOW, steps: [...FLOW.steps], at: 0 },
    plan: {
      scope: ["crates/api"],
      approved: false,
      after: "f-2",
      groups: [
        { id: "x1", agent: "rust-engineer", title: "Take the Job routes out", paths: ["crates/api"], tasks: [task("X1", "Remove the Job routes", "open"), task("X2", "Remove the Job events", "open")] },
        { id: "x2", agent: "bridge-engineer", title: "Delete the Jobs page", paths: ["apps/desktop"], tasks: [task("X3", "Delete the page and its tab", "open"), task("X4", "Delete its tests", "open")] },
      ],
    },
  },
});
const lessons = other(LESSONS, "Write the lessons page", { rows: [agent("l-1", "13:05:10", "The lessons page lists each retro newest first.")], lastTurnAt: ago(55) });

// ---- The store ------------------------------------------------------------------------------------------------

function sessions(control: BayControl): SessionsStore {
  const base = sessionsStore([main, prune, names, audit, fold, lessons]);
  // The mock's first start is a fixture Session with a script of its own; it is spent here and kept off the ship.
  base.start();
  const own = new Set([MAIN, PRUNE, NAMES, AUDIT, FOLD, LESSONS]);
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  let beat = 0;
  return {
    ...base,
    get: () => {
      const all = base.get();
      if (all !== from) {
        from = all;
        only = all.filter((one) => own.has(one.id));
      }
      return only;
    },
    // A judge's concern, sent or dismissed, is gone from the Session's work; the rest of the acts are the store's.
    work: {
      workflows: [],
      choose: () => undefined,
      unpick: () => undefined,
      approve: () => undefined,
      suggest: () => undefined,
      ...base.work,
      sendConcern: (id, concern) => base.edit(id, (one) => (one.work === undefined ? one : { ...one, work: { ...one.work, concerns: (one.work.concerns ?? []).filter((it) => it.id !== concern) } })),
      dismissConcern: (id, concern) => base.edit(id, (one) => (one.work === undefined ? one : { ...one, work: { ...one.work, concerns: (one.work.concerns ?? []).filter((it) => it.id !== concern) } })),
    },
    // Time passes: the drones move to the next files, a task ticks, and the bay is left stranded with work in it.
    later() {
      beat += 1;
      const next = fieldAt(beat);
      base.edit(MAIN, (one) => ({ ...one, work: { ...one.work, ...next, ...(one.work?.plan === undefined ? {} : { plan: { ...one.work.plan, groups: groups(1) } }) } }));
      control.pool?.set(pool(true));
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

// ---- The Cockpit round it ----------------------------------------------------------------------------------------

const bay = (slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({ manifest_id: OWNER, slot, path: `${ROOT}/.armada/slots/slot-${slot}`, base: "main", warm: true, behind: 0, held, ...rest });

/** The pool: the Session's bay is its own, or, once time has passed, left stranded with its work still in it. */
function pool(stranded: boolean): WorktreesHeld {
  return {
    slots: [
      bay(1, { state: "hosted_session", session_id: AUDIT, session_title: audit.title! }, { branch: "armada/audit-rotation", since: ago(35) }),
      bay(2, { state: "hosted_session", session_id: NAMES, session_title: names.title! }, { branch: "fix/migration-names", since: ago(90) }),
      stranded
        ? bay(
            SLOT,
            { state: "stranded", why: "2 uncommitted, first crates/fleet/src/lease.rs" },
            {
              branch: BRANCH,
              since: ago(21),
              behind: 31,
              stranded: {
                uncommitted: ["crates/fleet/src/lease.rs", "crates/fleet/src/pool.rs"],
                commits: [{ sha: "a41c9e07d2", subject: "Show the attempt on the bay tile", home: "only_here" }],
                unpushed: 1,
                last_holder: { kind: "session", id: MAIN, title: main.title!, open: true },
              },
            },
          )
        : bay(SLOT, { state: "hosted_session", session_id: MAIN, session_title: main.title! }, { branch: BRANCH, since: ago(21), behind: 4 }),
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
  event(21, main, "session.started", "run", "Started"),
  event(21, main, "slot.leased", "idle", "Slot 3 leased", "slot-3"),
  event(4, main, "session.edited", "run", "Edited 2 files", "crates/fleet/src/lease.rs"),
  event(35, audit, "session.started", "run", "Started"),
  event(35, audit, "slot.leased", "idle", "Slot 1 leased", "slot-1"),
  event(90, names, "session.started", "run", "Started"),
];

export function compactSession(name: string, said: string): Scenario {
  const base = holding(name, said);
  return {
    ...base,
    state: { ...base.state, repository: null },
    held: pool(false),
    draft: { sessions, events: EVENTS, areas: () => mockAreas("/armada") },
  };
}

