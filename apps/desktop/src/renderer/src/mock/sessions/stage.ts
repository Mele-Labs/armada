// The Sessions walk's whole world, in memory: a few Sessions, what the walk's
// own Session does moment by moment, and the window's view. Mock-only. Nothing
// here reaches Fleet, and every field it holds is one Fleet would owe
// (`packages/screens/src/draft/sessions.ts`).
//
// **The agent is a script, not a model.** Each moment is what the owner's
// Session would do next, applied when a walk's `later` step lets time pass or,
// for the first, when Send is pressed.

import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";

export type View = { at: "overview" } | { at: "session"; id: string };

export type Stage = {
  sessions: readonly Session[];
  view: View;
  query: string;
  /** The walk's own Session, once it exists. */
  mine?: string;
  /** A permission the agent is held on, in the walk's own Session. */
  asked?: { command: string };
};

const idle = { state: "idle" } as const;

/** The Sessions already open beside the walk's own. */
function others(): Session[] {
  return [
    {
      id: "s2",
      title: "Release notes script",
      turn: idle,
      lastTurn: "13:48",
      rows: [{ id: "s2-1", at: "13:48:02", kind: "message", from: { kind: "agent" }, text: "The script reads the merged pull requests since the last tag." }],
      attachments: [
        { kind: "slot", slot: 5 },
        { kind: "branch", name: "rel/notes-script", slot: 5 },
        { kind: "pull_request", number: 1847, title: "Release notes from merged pull requests", branch: "rel/notes-script", checks: { state: "passed" } },
        { kind: "job", id: "49", title: "Group notes by crate", state: "review", branch: "rel/49-group-notes", slot: 2 },
      ],
    },
    {
      id: "s3",
      title: "Store migration spike",
      turn: idle,
      lastTurn: "12:20",
      rows: [{ id: "s3-1", at: "12:20:41", kind: "message", from: { kind: "agent" }, text: "Named migrations apply in order and each is recorded once." }],
      attachments: [
        { kind: "slot", slot: 7 },
        { kind: "branch", name: "spike/store-migrations", slot: 7 },
        { kind: "job", id: "47", title: "Record applied migrations", state: "running", branch: "job/47-applied-migrations", slot: 8 },
        { kind: "studio", id: "st1", title: "Migration shapes" },
      ],
    },
  ];
}

/** Jobs on the Board that no Session dispatched. */
export const PLAIN_JOBS = [
  { id: "44", title: "Cap the log reader", branch: "job/44-cap-log-reader", slot: 1, state: "landed" as const },
];

export function initial(): Stage {
  return { sessions: others(), view: { at: "overview" }, query: "" };
}

export const MINE = "s7";

let clock = 0;
/** A time of day that only moves forward, so a row's stamp never reads out of order. */
function at(): string {
  clock += 7;
  const minutes = 3 + Math.floor(clock / 60);
  return `14:${String(minutes).padStart(2, "0")}:${String(clock % 60).padStart(2, "0")}`;
}

let rowId = 0;
const row = (make: (id: string, stamp: string) => SessionRow): SessionRow => make(`r${(rowId += 1)}`, at());
const tool = (text: string): SessionRow => row((id, stamp) => ({ id, at: stamp, kind: "tool", text }));
const said = (text: string): SessionRow => row((id, stamp) => ({ id, at: stamp, kind: "message", from: { kind: "agent" }, text }));

export type Store = {
  get: () => Stage;
  subscribe: (listener: () => void) => () => void;
  newSession: () => void;
  open: (id: string) => void;
  overview: () => void;
  search: (query: string) => void;
  send: (text: string) => void;
  /** The walk's own Session does its next thing. */
  later: () => void;
  answer: () => void;
  dispose: () => void;
};

export function createStore(): Store {
  let now = initial();
  const listeners = new Set<() => void>();
  const timers = new Set<number>();
  let moment = 0;
  let started = false;
  clock = 0;
  rowId = 0;

  const set = (next: Stage) => {
    now = next;
    listeners.forEach((one) => one());
  };
  const edit = (id: string, change: (one: Session) => Session) =>
    set({ ...now, sessions: now.sessions.map((one) => (one.id === id ? change(one) : one)) });
  const after = (ms: number, run: () => void) => {
    const handle = window.setTimeout(() => {
      timers.delete(handle);
      run();
    }, ms);
    timers.add(handle);
  };
  const add = (id: string, rows: SessionRow[], attachments: SessionAttachment[] = []) =>
    edit(id, (one) => ({ ...one, rows: [...one.rows, ...rows], attachments: [...one.attachments, ...attachments] }));
  const finish = (id: string, stamp = "14:09") => edit(id, (one) => ({ ...one, turn: idle, lastTurn: stamp }));
  const setAttachment = (id: string, match: (one: SessionAttachment) => boolean, to: SessionAttachment) =>
    edit(id, (one) => ({ ...one, attachments: one.attachments.map((a) => (match(a) ? to : a)) }));

  /** The first write: the slot leased and the branch cut, in the thread where it happened. */
  const firstTurn = () => {
    add(MINE, [
      tool("Read crates/store/tests/flaky.rs"),
      row((id, stamp) => ({ id, at: stamp, kind: "lease", slot: 3, branch: "fix/flaky-store" })),
      tool("Edit crates/store/tests/flaky.rs"),
      said("The test reads the wall clock. It is pinned to a fixed instant now."),
    ], [
      { kind: "slot", slot: 3 },
      { kind: "branch", name: "fix/flaky-store", slot: 3 },
    ]);
    edit(MINE, (one) => ({ ...one, title: "Flaky store test" }));
    finish(MINE);
  };

  const dispatching = () => {
    add(MINE, [
      tool("dispatch_job: Pin the store clock in every test"),
      tool("dispatch_job: Retire sleep calls in the store tests"),
      tool("subagent: Read the CI history of store_flaky"),
      tool("subagent: Find other tests that read the wall clock"),
      tool("studio: Flaky store shapes"),
      said("Two Jobs dispatched and two subagents reading."),
    ], [
      { kind: "job", id: "52", title: "Pin the store clock in every test", state: "running", branch: "fix/52-pin-store-clock", slot: 4 },
      { kind: "job", id: "53", title: "Retire sleep calls in the store tests", state: "running", branch: "fix/53-retire-sleeps", slot: 6 },
      { kind: "subagent", id: "a1", task: "Read the CI history of store_flaky", state: "running" },
      { kind: "subagent", id: "a2", task: "Find other tests that read the wall clock", state: "running" },
      { kind: "studio", id: "st2", title: "Flaky store shapes" },
    ]);
    after(1500, () =>
      setAttachment(MINE, (a) => a.kind === "subagent" && a.id === "a1", { kind: "subagent", id: "a1", task: "Read the CI history of store_flaky", state: "done" }),
    );
  };

  const opening = () => {
    add(MINE, [tool("gh pr create --base main"), said("Opened #1843.")], [
      { kind: "pull_request", number: 1843, title: "Pin the store clock", branch: "fix/flaky-store", checks: { state: "pending" } },
    ]);
    after(1400, () => {
      setAttachment(MINE, (a) => a.kind === "pull_request", {
        kind: "pull_request",
        number: 1843,
        title: "Pin the store clock",
        branch: "fix/flaky-store",
        checks: { state: "failed", failing: "store: 2 failed" },
      });
      add(MINE, [said("store failed on #1843: 2 tests.")]);
    });
  };

  /** Another Session writes to this one, and the message wakes it: it takes a turn. */
  const woken = () => {
    const from = { id: "s2", title: "Release notes script" };
    add(MINE, [row((id, stamp) => ({ id, at: stamp, kind: "message", from: { kind: "session", ...from }, text: "your branch broke main" }))]);
    edit(MINE, (one) => ({ ...one, turn: { state: "working", wokenBy: from } }));
    after(1400, () => {
      add(MINE, [
        tool("Read the CI log of store_flaky"),
        tool("Edit crates/store/src/clock.rs"),
        said("main fails in store_flaky because the clock module is imported twice. Fixed on fix/flaky-store. Replied to s2."),
      ]);
      finish(MINE, "14:13");
      set({ ...now, asked: { command: "git push --force-with-lease origin fix/flaky-store" } });
    });
  };

  const turns = [dispatching, opening, woken];

  return {
    get: () => now,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    newSession() {
      const mine: Session = { id: MINE, attachments: [], rows: [], turn: idle };
      set({ ...now, mine: MINE, sessions: [mine, ...now.sessions.filter((one) => one.id !== MINE)], view: { at: "session", id: MINE } });
    },
    open: (id) => set({ ...now, view: { at: "session", id } }),
    overview: () => set({ ...now, view: { at: "overview" } }),
    search: (query) => set({ ...now, query }),
    send(text) {
      const session = now.view.at === "session" ? now.view.id : undefined;
      if (session === undefined || text.trim() === "") return;
      add(session, [row((id, stamp) => ({ id, at: stamp, kind: "message", from: { kind: "you" }, text: text.trim() }))]);
      edit(session, (one) => ({ ...one, turn: { state: "working" } }));
      if (session === MINE && !started) {
        started = true;
        after(600, firstTurn);
      }
    },
    later() {
      const next = turns[moment];
      moment += 1;
      next?.();
    },
    answer: () => set({ ...now, asked: undefined }),
    dispose: () => timers.forEach((one) => window.clearTimeout(one)),
  };
}
