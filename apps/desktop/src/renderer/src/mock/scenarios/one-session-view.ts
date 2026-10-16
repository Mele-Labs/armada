// One view of a Session, on the Cockpit and on the Sessions page: its thread and composer in the middle, its plan and workflow in the Status
// area, its drones on a Field, a judge's concerns as cards, and a proposal to start from. One Session goes from its dispatched words to a
// pull request ready, a step each time the walk lets time pass or he presses an act. The Session's `work` is draft data
// (`packages/screens/src/draft/sessions.ts`); nothing here reaches Fleet. The walk `oneSessionView` plays it, in two takes.

import { featureRunning } from "@armada/jobs/fake";
import type { TimelineEvent, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session, SessionRow, SessionWork, WorkConcern, WorkDrone, WorkFile, WorkGroup, WorkTask, WorkTaskState, WorkflowChoice } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { asRow, holding, servedFrom } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { SessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const board = asRow(featureRunning(), 83, "migrate", "Order the store migrations");
const OWNER = board.manifests[0]!.id;
const ROOT = servedFrom(board.manifests[0]!).root;

const MINE = "one-view-1";
const QUIET = "one-view-2";
const BUSY = "one-view-3";
const BRANCH = "armada/lease-retry";

/** A long workflow, to see how twelve steps read in the Status area. */
const RELEASE: WorkflowChoice = { name: "Release", steps: ["Branch", "Changelog", "Version", "Build", "Sign", "Notarize", "Smoke", "Verify", "Tag", "Publish", "Announce", "Pull request"] };

const WORKFLOWS: readonly WorkflowChoice[] = [
  { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"] },
  { name: "Bug fix", steps: ["Reproduce", "Fix", "Verify", "Pull request"] },
  { name: "Code review", steps: ["Read", "Review", "Report"] },
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
  work: { proposal: { workflow: FEATURE.name, picked: ["workflow", "model", "effort"] }, workflow: { ...FEATURE, at: -1 } },
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
  return [
    { id: "g1", agent: "rust-engineer", title: "Lease retry in Fleet", paths: ["crates/fleet"], tasks: [task("T1", "Retry the lease when the pool is contended", at(1, 2)), task("T2", "Back off between attempts", at(2, 3))] },
    { id: "g2", agent: "protocol-engineer", title: "Attempt count on the wire", paths: ["crates/ipc", "packages/protocol"], tasks: [task("T3", "Add attempts to the lease reply", at(1, 2))] },
    { id: "g3", agent: "bridge-engineer", title: "Attempt on the bay", paths: ["apps/desktop"], tasks: [task("T4", "Show the attempt on the bay tile", at(2, 4))] },
  ];
}

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
  { id: "c1", step: "Build", judge: "Judge", text: "The retry reads the pool without holding the lease lock, so two Sessions can still take one bay." },
  { id: "c2", step: "Build", judge: "Judge", text: "The back off has no ceiling." },
];

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

function sessions(control: Parameters<typeof sessionsStore>[3]): SessionsStore {
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  const base = sessionsStore([none, none], none, [], control, [], [dispatched, quiet, busy]);
  // The mock's first start is a fixture Session with a script of its own; it is spent here and kept off the ship.
  base.start();
  const own = new Set([MINE, QUIET, BUSY]);
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  let stage = 0;
  let rowId = 0;
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
    cockpitOnly: true,
    work: {
      workflows: WORKFLOWS,
      // The proposal is read again: a different workflow draws a different rail, and none draws no rail at all.
      choose: (_id, name) => {
        const chosen = WORKFLOWS.find((one) => one.name === name);
        works((work) => {
          const { workflow, ...rest } = work;
          void workflow;
          return { ...rest, proposal: { workflow: name, ...(work.proposal?.picked === undefined ? {} : { picked: work.proposal.picked.filter((one) => one !== "workflow") }) }, ...(chosen === undefined ? {} : { workflow: { ...chosen, at: -1 } }) };
        });
      },
      unpick: (_id, value) =>
        works((work) => (work.proposal?.picked === undefined ? work : { ...work, proposal: { ...work.proposal, picked: work.proposal.picked.filter((one) => one !== value) } })),
      // Start begins the work: the first step, and the agent reads and asks before it plans.
      start: () => {
        works((work) => {
          return { ...work, ...(work.proposal === undefined ? {} : { proposal: { ...work.proposal, started: true } }), ...(work.workflow === undefined ? {} : { workflow: { ...work.workflow, at: 0 } }) };
        });
        fly(0);
        push([tool("Read crates/fleet/src/lease.rs"), said("Two Sessions can ask for the same bay between the read and the write. One question before I plan.")], {
          asked: { command: QUESTIONS[0]!.question, call: "v-q1", offers: ["allow_once", "refuse"], questions: QUESTIONS },
        });
      },
      approve: () => {
        works((work) => (work.plan === undefined ? work : { ...work, plan: { ...work.plan, approved: true } }));
        push(
          [
            asked("Approved the plan."),
            said("Starting with the lease in Fleet. The wire and the bay tile follow."),
            tool("Edit crates/fleet/src/lease.rs"),
            next((id, at) => ({ id, at, kind: "lease", slot: 3, branch: BRANCH })),
          ],
          { ...working(), attachments: [{ kind: "branch", name: BRANCH, slot: 3 }] },
        );
        building(1);
      },
      sendConcern: (_id, concern) => {
        const one = mine()?.work?.concerns?.find((it) => it.id === concern);
        if (one === undefined) return;
        works((work) => ({ ...work, concerns: (work.concerns ?? []).filter((it) => it.id !== concern) }));
        push([asked(`${one.step}: ${one.text}`)], working());
        after(700, () => push([tool("Edit crates/fleet/src/lease.rs"), said(REPLIES[concern] ?? "Noted.")], idle()));
      },
      dismissConcern: (_id, concern) => works((work) => ({ ...work, concerns: (work.concerns ?? []).filter((it) => it.id !== concern) })),
    },
    // The question the agent asked is answered, and it goes on to record the plan.
    answer: (_id, answer, answers) => {
      change((one) => {
        const { asked: was, ...rest } = one;
        void was;
        return rest;
      });
      if (answer === "refuse" || (answers ?? []).length === 0) return push([said("Taking three attempts.")], working());
      push([asked(answers!.map((one) => `**${one.question}**\n${one.chosen.join(", ")}`).join("\n\n")), said(`${answers!.map((one) => one.chosen.join(" and ")).join(", ")} attempts. Writing the plan.`)], working());
    },
    // Words he types go to the agent, which answers in a line.
    send: (id, sent) => {
      const done = base.send(id, sent);
      if (id === MINE) after(700, () => push([said("Noted.")], idle()));
      return done;
    },
    later() {
      const now = mine();
      if (now === undefined) return;
      const plan = now.work?.plan;
      if (plan === undefined) {
        // The agent records its plan after planning with him, and it arrives for his approval.
        const note = said("The plan is recorded: four tasks in three groups. Approve it, or tell me what to change.");
        works((work) => ({ ...work, plan: { scope: SCOPE, groups: groups(0), approved: false, after: note.id } }));
        push([note], idle());
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
      } else if (stage === 3) {
        building(4);
        push(
          [tool("Bash armada check test lease"), said("The checks pass on the branch. The change is ready for a pull request.")],
          {
            ...idle(),
            attachments: [
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
    worktrees: [],
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

function build(): Scenario {
  const rows = [{ ...board, job: { ...board.job, owner_manifest_id: OWNER, started_at: ago(30) } }];
  const base = holding("one-session-view", "One view of a Session on the Cockpit and the Sessions page: plan and workflow, field, concerns, proposal", rows);
  return {
    ...base,
    state: { ...base.state, repository: null, jobs: rows.map((one) => one.job) },
    held: pool(),
    draft: { sessions, events: EVENTS, areas: () => mockAreas("/armada") },
  };
}

export const s261OneSessionView: Scenario = build();
