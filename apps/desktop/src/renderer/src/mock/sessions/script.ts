// What the Sessions mock holds: a few Sessions, and what the walk's own Session
// does moment by moment. Mock-only. Nothing here reaches Fleet, and every field
// it holds is one Fleet would owe (`packages/screens/src/draft/sessions.ts`).
//
// **The agent is a script, not a model.** Each moment is what the person's
// Session would do next, applied when a walk's `later` step lets time pass or,
// for the first, when the person sends a message.

import { closeWalkWindow, markWalkWindow, mockPage, openWalkWindow, type Variation } from "@armada/jobs/fake";
import { askToOpenSession } from "../../open-session";
import { removedSaid } from "./removed";
import { DATA_FLOW, DATA_FLOW_TITLE } from "./data-flow-sketch";
import { PINNED, PINNED_TITLE, READS, READS_AGAIN, READS_TITLE, SEQUENCE, SEQUENCE_TITLE, WORKFLOW, WORKFLOW_TITLE } from "./sketches";
import type { Scene } from "@armada/components";
import type { Session, SessionTag, SessionAttachment, SessionCommand, SessionRow, SessionSketch, SessionsDraft } from "@armada/screens/src/draft/sessions";

/** The draft the window reads, and what only the mock does: take the next turn, stop, and take a claim Fleet would have put in the ledger. */
export type SessionsStore = SessionsDraft & {
  later: () => void;
  dispose: () => void;
  attach: (id: string, attachment: SessionAttachment) => void;
  /** Fleet drops an item for good. One the agent stated also reaches it as the person's message, with `reason` where he gave one. */
  drop: (id: string, itemId: string, reason?: string) => void;
  /** Changes a Session as it stands, for a scenario that scripts one Session of its own. */
  edit: (id: string, change: (one: Session) => Session) => void;
  /** How slow Fleet is to answer the Cockpit's routes, and then to stop carrying what it answered: at once where absent. */
  pace?: Pace;
};

/** Something a route waits on until it is let go: a walk's step, not a timer, so a person can sit on the wait. */
export type Gate = { wait: () => Promise<void>; release: () => void };

/** How long Fleet takes to answer, and then to stop carrying what it answered. Where `held` is set it waits on that instead of on the clocks. */
export type Pace = { answerMs: number; settleMs: number; held?: Gate };

/** The Session that draws a sketch each time it is spoken to: the first, the same title again, and a change to it. */
export const SKETCHER = "s14";

/** The Session that draws how data moves through Fleet: a hub of boxes, converging labelled arrows, a long body. */
export const HUBBER = "s15";

/** A Job the walk's Session dispatches. The real Board holds it, so its id is the Board's. */
export type DispatchedJob = { id: string; number: number; title: string; branch: string; slot: number };

const idle = { state: "idle" } as const;
const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** The models Fleet lists, as `list_models` answers them, and the efforts a Drone's settings offer. */
const MODELS = ["haiku", "sonnet", "opus"];
const EFFORTS = ["low", "medium", "high"];

/** What `/` offers, as a terminal session lists them: skills first, then the commands. */
export const COMMANDS: readonly SessionCommand[] = [
  { name: "review", says: "Review the pull request" },
  { name: "simplify", says: "Simplify the changed code" },
  { name: "security-review", says: "Review the pending changes for security" },
  { name: "init", says: "Write the repository notes an agent reads first" },
  { name: "compact", says: "Clear the history and keep a summary" },
  { name: "clear", says: "Start over with an empty history" },
  { name: "model", says: "Set the model for this Session" },
];

const drawing = (boxes: [string, number, number, string][], lines: [string, string][] = []): SessionSketch => ({
  boxes: boxes.map(([id, x, y, body]) => ({ id, x, y, body })),
  lines: lines.map(([from, to], at) => ({ id: `l${at}`, from, to })),
});

/** The Sessions already open beside the walk's own. */
/** The Sessions every walk over the store opens with. */
export function others(): Session[] {
  return [
    {
      id: "s2",
      title: "Release notes script",
      turn: idle,
      lastTurn: "13:48",
      lastTurnAt: minutesAgo(4),
      rows: [{ id: "s2-1", at: "13:48:02", kind: "message", from: { kind: "agent" }, text: "The script reads the merged pull requests since the last tag." }],
      attachments: [
        { kind: "slot", slot: 5 },
        { kind: "branch", name: "rel/notes-script", slot: 5 },
        { kind: "pull_request", number: 1847, title: "Release notes from merged pull requests", branch: "rel/notes-script", address: "https://example.com/pull/1847", checks: { state: "passed" }, state: "open", auto: false },
      ],
    },
    {
      id: "s3",
      title: "Store migration spike",
      turn: idle,
      lastTurn: "12:20",
      lastTurnAt: minutesAgo(135),
      rows: [{ id: "s3-1", at: "12:20:41", kind: "message", from: { kind: "agent" }, text: "Named migrations apply in order and each is recorded once." }],
      attachments: [
        { kind: "slot", slot: 7 },
        { kind: "branch", name: "spike/store-migrations", slot: 7 },
        {
          kind: "pull_request",
          number: 1849,
          title: "Apply named migrations in order",
          branch: "spike/store-migrations",
          address: "https://example.com/pull/1849",
          checks: { state: "pending" },
          state: "draft",
          auto: false,
        },
        {
          kind: "pull_request",
          number: 1850,
          title: "Record each migration once",
          branch: "spike/store-migrations-record",
          address: "https://example.com/pull/1850",
          checks: { state: "pending" },
          state: "open",
          auto: false,
        },
        { kind: "studio", id: "st1", title: "Migration shapes" },
      ],
    },
  ];
}

/** How long the mock's pull request act takes to come back from the forge. */
const ACT_MS = 1500;

/** What every subagent the mock holds does, in order: the brief, its calls, and the report it ends on. */
const SUBAGENT_SCRIPT: readonly (readonly ["message" | "tool", string])[] = [
  ["message", "Read the CI history of store_flaky and say when it fails."],
  ["tool", "gh run list --workflow ci --limit 30"],
  ["tool", "Read crates/store/tests/flaky.rs"],
  ["message", "Thirty runs listed. Reading the four that failed."],
  ["tool", "gh run view 1043 --log-failed"],
  ["tool", "gh run view 1051 --log-failed"],
  ["tool", "gh run view 1060 --log-failed"],
  ["message", "store_flaky failed 4 of the last 30 runs, each when the test crossed a second boundary."],
];

export const MINE = "s7";
/** The Session that shows a page in a window when it is next spoken to. */
export const SHOWER = "s11";
const SHOWN = "Clock read report";
/** The Session that shows three takes of one page in a window when it is next spoken to. */
export const VARIER = "s12";
const VARIED = "Board layouts";
/** Its takes: the mock itself on three scenarios, the way a Session would serve three branches of one page. */
const takes = (): Variation[] =>
  [
    { name: "Every state", scenario: "every-state" },
    { name: "Needs you first", scenario: "dashboard-needs-you" },
    { name: "Sessions beside", scenario: "sessions" },
  ].map(({ name, scenario }) => {
    const url = new URL(mockPage());
    url.searchParams.set("scenario", scenario);
    return { name, url: url.toString() };
  });

/** The Session that shows three walks in a window when it is next spoken to: the bar draws their controls. */
export const WALKER = "s13";
const WALKED = "Walks in a window";
/** Its takes: three short walks, each the mock playing itself in the frame, the way a Session would serve a walkthrough. */
const walkTakes = (): Variation[] =>
  [
    { name: "Ledger filter", walk: "session-ledger-filter" },
    { name: "Task marks", walk: "taskMarkTooltips" },
    { name: "Sleep cockpit", walk: "sleepCockpit" },
  ].map(({ name, walk }) => {
    const url = new URL(mockPage());
    url.search = `?walk=${walk}&walked`;
    return { name, url: url.toString() };
  });

/** A Job on the Board that a person can tag: where it stands, and how the Session would hold it. */
export type TaggableJob = DispatchedJob & { state: "escalated" | "review" };

/** What the mock Fleet lets a Session do to the Board: move a Job to another status, on the row and on its detail. */
export type BoardControl = {
  setStatus: (jobId: string, status: string) => void;
  /** Rows that reach the Board, added to what it holds and never replacing a row already there. */
  add: (rows: readonly unknown[]) => void;
};

const PICTURE = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 200"><rect width="320" height="200" fill="#2a3340"/><rect x="24" y="24" width="272" height="28" rx="4" fill="#4b5a6e"/><rect x="24" y="68" width="120" height="108" rx="4" fill="#4b5a6e"/><rect x="160" y="68" width="136" height="48" rx="4" fill="#6b7f99"/><rect x="160" y="128" width="136" height="48" rx="4" fill="#4b5a6e"/></svg>`;

/** The last lines of what a background command wrote, by the file it writes them to. */
const BACKGROUND_OUTPUT: Record<string, string> = {
  "/tmp/armada/tasks/serve-mock.output": "VITE v6.3.5  ready in 412 ms\n\n  Local:   http://127.0.0.1:5176/\n\nGET /?scenario=sessions 200 11ms\nGET /?scenario=sessions&walked 200 9ms\nGET /?scenario=sessions&walk=ledger-background 200 8ms\n",
  "/tmp/armada/tasks/watch-tests.output": "RUN  v3.2.4 /slots/slot-6/apps/desktop\n\n Test Files  3 passed (3)\n      Tests  41 passed (41)\n   Duration  6.18s\n\nWaiting for file changes...\n",
  "/tmp/armada/tasks/watch-ci.output": "Run ci (9321177) triggered by pull_request\n* check  passed  1m 12s\n* test   in progress\n* gate   queued\n",
  "/tmp/armada/tasks/build-renderer.output": "vite v6.3.5 building for production...\ntransforming...\n2214 modules transformed.\nout/renderer/index.html            0.62 kB\nout/renderer/assets/index.css    148.30 kB\nout/renderer/assets/index.js   1,904.77 kB\nbuilt in 9.41s\n",
  "/tmp/armada/tasks/tail-fleet.output": "2026-10-16T14:02:11Z INFO fleet: session aa994f6f woke\n2026-10-16T14:02:12Z INFO fleet: lease slot 6 for aa994f6f\n2026-10-16T14:02:40Z INFO fleet: check run 41 passed\n",
};

const NOTE = "# Store clock\n\nThe store reads the clock once per write.\n\n- Pinned in tests\n- Read through `Clock::now`\n";

/** What the fake runner prints for a command containing the first of each pair. */
const RUN_OUTPUT: readonly (readonly [string, readonly string[]])[] = [
  ["typecheck", ["> @armada/desktop typecheck", "> tsc --noEmit -p tsconfig.web.json", "Found 0 errors."]],
  ["vitest", ["RUN  v3.2.4 packages/components", "✓ Prose.test.tsx (14)", "✗ SessionThread.test.tsx (1)", "AssertionError: expected 3 to equal 4", "Tests  1 failed | 41 passed"]],
  ["lsof", ["Stopping the server on 5788", "pid 41822 ended", "Port 5788 is free"]],
  ["tail -f", ["build.log: compiling", "build.log: 212 modules"]],
  ["--autostash", ["Created autostash: 4be1f3a", "Updating 6c0d2e1..a91f7b3", "Fast-forward", " armada.yml | 6 +-", " crates/fleet/src/checkouts.rs | 212 +++++++++", "Applied autostash"]],
];
const RUN_DEFAULT = ["done"];

export function sessionsStore(
  jobs: readonly [DispatchedJob, DispatchedJob],
  review: DispatchedJob,
  taggableJobs: readonly TaggableJob[],
  board: BoardControl,
  /** The Board rows the dispatching moment adds. */
  dispatchedRows: readonly unknown[] = [],
  /** Sessions open beside the usual ones. */
  more: readonly Session[] = [],
  /** What an agent says to a Session, one message each time a walk lets time pass, before the usual turns. */
  arrivals: readonly (readonly [id: string, text: string])[] = [],
): SessionsStore {
  // A Session in `more` with a usual one's id stands in for it.
  let now: readonly Session[] = [...others().filter((one) => !more.some((it) => it.id === one.id)), ...more];
  let clock = 0;
  let rowId = 0;
  let moment = 0;
  let arrived = 0;
  let started = false;
  let made = 0;
  /** How many times a Session that was handed a Job has been asked, to know which of its two turns is next. */
  const asked = new Map<string, number>();
  const listeners = new Set<() => void>();
  const timers = new Set<number>();
  /** How many times each subagent's thread has been read, which is how far a running one has got. */
  const reads = new Map<string, number>();

  const set = (next: readonly Session[]) => {
    now = next;
    listeners.forEach((one) => one());
  };
  const edit = (id: string, change: (one: Session) => Session) => set(now.map((one) => (one.id === id ? change(one) : one)));
  const after = (ms: number, run: () => void) => {
    const handle = window.setTimeout(() => {
      timers.delete(handle);
      run();
    }, ms);
    timers.add(handle);
  };

  /** A time of day that only moves forward, so a row's stamp never reads out of order. */
  const at = () => {
    clock += 7;
    return `14:${String(3 + Math.floor(clock / 60)).padStart(2, "0")}:${String(clock % 60).padStart(2, "0")}`;
  };
  const row = (make: (id: string, stamp: string) => SessionRow): SessionRow => make(`r${(rowId += 1)}`, at());
  const tool = (text: string): SessionRow => row((id, stamp) => ({ id, at: stamp, kind: "tool", text }));
  const said = (text: string): SessionRow => row((id, stamp) => ({ id, at: stamp, kind: "message", from: { kind: "agent" }, text }));
  const addTo = (id: string, rows: SessionRow[], attachments: SessionAttachment[] = []) =>
    edit(id, (one) => ({ ...one, rows: [...one.rows, ...rows], attachments: [...one.attachments, ...attachments] }));
  const add = (rows: SessionRow[], attachments: SessionAttachment[] = []) => addTo(MINE, rows, attachments);
  const finishOf = (id: string, stamp: string) => edit(id, (one) => ({ ...one, turn: idle, lastTurn: stamp, lastTurnAt: new Date().toISOString() }));
  const finish = (stamp: string) => finishOf(MINE, stamp);

  /** The window on a page a Session showed. **A note taken in it is a message to that Session**, which wakes it. */
  const openWindow = (id: string, url: string) => {
    const session = now.find((one) => one.id === id);
    if (session === undefined) return;
    // The window's own title, apart from the Session's name. One titled by its address has none.
    const shown = session.attachments.find((one) => one.kind === "artifact" && one.form === "window" && one.id === url);
    openWalkWindow(session.title ?? id, url, {
      title: shown?.kind === "artifact" && shown.title !== undefined && shown.title !== url ? shown.title : null,
      job: session.title ?? id,
      into: "Session",
      onOpenOwner: () => askToOpenSession(id),
      // A note taken on one of several takes names it first, so the Session knows which page it is about.
      onNote: (note, picked) => {
        const on = picked.variation === undefined ? "" : `${picked.variation.letter}, ${picked.variation.name}: `;
        return store.send(id, { text: `${note}\nOn ${on}${picked.element}, ${picked.location}`, files: [], sketches: [], tags: [] });
      },
      // Approve is a message from the person too, and wakes the Session the same way.
      onApprove: async (address, variation) => {
        const which = variation === undefined ? "" : ` ${variation.letter}, ${variation.name}`;
        await store.send(id, { text: `Approved${which}: ${address}`, files: [], sketches: [], tags: [] });
        return { ok: true };
      },
      ...(id === VARIER ? { variations: takes() } : id === WALKER ? { variations: walkTakes() } : {}),
      // Shown is said to be ready: a Session shows a page when it is done with it.
      readiness: "ready",
    });
  };
  /** The Session shows a page: its window opens by itself, and the ledger and the thread keep it. */
  const showWindow = (id: string, url: string, title: string) => {
    addTo(
      id,
      [row((rid, stamp) => ({ id: rid, at: stamp, kind: "window", title, url }))],
      now.find((one) => one.id === id)?.attachments.some((a) => a.kind === "artifact" && a.form === "window" && a.id === url) === true
        ? []
        : [{ kind: "artifact", form: "window", id: url, title }],
    );
    openWindow(id, url);
  };
  /**
   * The agent calls `show_sketch`: the call is a tool row that carries its arguments, as Fleet's thread does, and
   * the sketch is a row on the ledger that a sketch of the same title replaces. The card is drawn from the row.
   */
  const showSketch = (id: string, title: string, drawing: Scene, against?: { title: string; drawing: Scene }) => {
    const key = `sketch:${title}`;
    const sketch: SessionAttachment = { kind: "sketch", id: key, title, by: "agent", drawing, ...(against === undefined ? {} : { against }) };
    edit(id, (one) => ({
      ...one,
      rows: [...one.rows, tool(`show_sketch ${JSON.stringify({ title, scene: drawing, ...(against === undefined ? {} : { against: against.title }) })}`)],
      attachments: one.attachments.some((a) => a.kind === "sketch" && a.id === key)
        ? one.attachments.map((a) => (a.kind === "sketch" && a.id === key ? sketch : a))
        : [...one.attachments, sketch],
    }));
  };
  const attach = (match: (one: SessionAttachment) => boolean, to: SessionAttachment) =>
    edit(MINE, (one) => ({ ...one, attachments: one.attachments.map((a) => (match(a) ? to : a)) }));

  /** The first write: the slot leased and the branch cut, in the thread where it happened. */
  const firstTurn = () => {
    add(
      [
        tool("Read crates/store/tests/flaky.rs"),
        row((id, stamp) => ({ id, at: stamp, kind: "lease", slot: 3, branch: "fix/flaky-store" })),
        tool("Edit crates/store/tests/flaky.rs"),
        said("The test reads the wall clock. It is pinned to a fixed instant now."),
      ],
      [
        { kind: "slot", slot: 3 },
        { kind: "branch", name: "fix/flaky-store", slot: 3 },
      ],
    );
    edit(MINE, (one) => ({ ...one, title: "Flaky store test" }));
    finish("14:09");
  };

  const dispatching = () => {
    board.add(dispatchedRows);
    const [first, second] = jobs;
    add(
      [
        tool(`dispatch_job: ${first.title}`),
        tool(`dispatch_job: ${second.title}`),
        tool("subagent: Read the CI history of store_flaky"),
        tool("subagent: Find other tests that read the wall clock"),
        tool("studio: Flaky store shapes"),
        tool("publish_sketch: CI history of store_flaky"),
        said("Two Jobs dispatched and two subagents reading."),
      ],
      [
        { kind: "job", ...first, state: "running" },
        { kind: "job", ...second, state: "running" },
        { kind: "subagent", id: "a1", task: "Read the CI history of store_flaky", state: "running" },
        { kind: "subagent", id: "a2", task: "Find other tests that read the wall clock", state: "running" },
        { kind: "studio", id: "st2", title: "Flaky store shapes" },
        {
          kind: "sketch",
          id: "k3",
          title: "CI history of store_flaky",
          by: "agent",
          drawing: drawing(
            [
              ["a", 0, 0, "store_flaky passes"],
              ["b", 320, 0, "store_flaky fails"],
              ["c", 160, 140, "the wall clock crosses a second"],
            ],
            [
              ["c", "a"],
              ["c", "b"],
            ],
          ),
        },
      ],
    );
    after(1500, () =>
      attach((a) => a.kind === "subagent" && a.id === "a1", {
        kind: "subagent",
        id: "a1",
        task: "Read the CI history of store_flaky",
        state: "done",
        report: "store_flaky failed 4 of the last 30 runs, each when the test crossed a second boundary.",
      }),
    );
  };

  const opening = () => {
    const pr = { kind: "pull_request", number: 1843, title: "Pin the store clock", branch: "fix/flaky-store", address: "https://example.com/pull/1843", state: "open", auto: false } as const;
    add([tool("gh pr create --base main"), said("Opened #1843.")], [{ ...pr, checks: { state: "pending" } }]);
    after(1400, () => {
      attach((a) => a.kind === "pull_request", { ...pr, checks: { state: "failed", failing: "store: 2 failed" } });
      add([said("store failed on #1843: 2 tests.")]);
    });
  };

  /** Another Session writes to this one, and the message wakes it: it takes a turn. */
  const woken = () => {
    const from = { id: "s2", title: "Release notes script" };
    add([row((id, stamp) => ({ id, at: stamp, kind: "message", from: { kind: "session", ...from }, text: "your branch broke main" }))]);
    edit(MINE, (one) => ({ ...one, turn: { state: "working", wokenBy: from } }));
    after(1400, () => {
      add([
        tool("Read the CI log of store_flaky"),
        tool("Edit crates/store/src/clock.rs"),
        said("main fails in store_flaky because the clock module is imported twice. Fixed on fix/flaky-store. Replied to s2."),
      ]);
      edit(MINE, (one) => ({ ...one, asked: { command: "git push --force-with-lease origin fix/flaky-store" } }));
      finish("14:13");
    });
  };

  /** Looking at a Job somebody tagged: the agent reads it with the tools Fleet gives it, then says what it found. */
  const investigating = (id: string, job: SessionTag & { job: NonNullable<SessionTag["job"]> }) => {
    const n = job.job.number;
    addTo(
      id,
      [
        tool(`examine_job ${n}`),
        tool(`get_job_log ${n}`),
        tool(`get_check_output ${n} lint`),
        tool(`get_diff ${n}`),
        said(`Job ${n} stopped at its gate: the lint Check failed. The Drone added a retry loop with no cap, and clippy flags the loop at retry.rs:41. Capping it at five attempts would clear the Check.`),
      ],
    );
    finishOf(id, "14:21");
  };
  const redirecting = (id: string, job: SessionTag & { job: NonNullable<SessionTag["job"]> }) => {
    const n = job.job.number;
    addTo(id, [tool(`redirect_drone ${n}: cap the retry loop at five attempts`), said(`Redirected the Drone on Job ${n}. It is running again.`)]);
    edit(id, (one) => ({
      ...one,
      attachments: one.attachments.map((a) => (a.kind === "job" && a.id === job.id ? { ...a, state: "running" as const } : a)),
    }));
    finishOf(id, "14:23");
  };

  const turns = [dispatching, opening, woken];

  const store: SessionsStore = {
    get: () => now,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    start(tag) {
      made += 1;
      const id = made === 1 && tag === undefined ? MINE : `s${7 + made}`;
      set([{ id, attachments: [], rows: [], turn: idle, ...(tag === undefined ? {} : { pendingTags: [tag] }) }, ...now]);
      return id;
    },
    taggable() {
      const jobsOnBoard = taggableJobs.map((j): SessionTag => ({
        kind: "job",
        id: j.id,
        title: `${j.number} ${j.title}`,
        job: { number: j.number, branch: j.branch, slot: j.slot, state: j.state === "escalated" ? "escalated" : "review" },
      }));
      const dispatched = now.flatMap((one) =>
        one.attachments.flatMap((a): SessionTag[] =>
          a.kind === "job" && a.looking !== true && !taggableJobs.some((j) => j.id === a.id)
            ? [{ kind: "job", id: a.id, title: `${a.number} ${a.title}`, job: { number: a.number, branch: a.branch, ...(a.slot === undefined ? {} : { slot: a.slot }), state: a.state === "escalated" ? "escalated" : "review" } }]
            : [],
        ),
      );
      return [...jobsOnBoard, ...dispatched];
    },
    pilot(jobId, outcome) {
      void outcome;
      const tag = store.taggable().find((one) => one.kind === "job" && one.id === jobId);
      made += 1;
      const id = `s${7 + made}`;
      const n = tag?.job?.number ?? 0;
      const branch = tag?.job?.branch ?? "";
      const slot = tag?.job?.slot ?? 0;
      const title = (tag?.title ?? "").replace(/^\d+ /, "");
      board.setStatus(jobId, "piloted");
      set([
        {
          id,
          title,
          turn: idle,
          lastTurn: "14:30",
          rows: [
            {
              id: `h${made}`,
              at: at(),
              kind: "handoff",
              job: { number: n, title },
              slot,
              branch,
              step: { id: "regression_verify", label: "Verify the fix" },
              attempts: 3,
              refusals: ["No test covers the retry cap, retry.rs:41", "The lint Check still fails on the loop at retry.rs:41"],
              plan: { outside: ["crates/retry/src/loop.rs"], unwritten: ["crates/retry/tests/backoff.rs"] },
              narrative: {
                trying_to: "Cap the retry backoff at five attempts",
                blocked_by: "The lint Check fails on the loop at retry.rs:41",
                tried: ["Added a retry loop with exponential backoff", "Added a sleep between attempts", "Ran the Check after each"],
              },
            },
          ],
          attachments: [
            { kind: "slot", slot, handed: { job: n } },
            { kind: "branch", name: branch, slot, handed: { job: n } },
            { kind: "job", id: jobId, number: n, title, state: "piloted", branch, slot },
          ],
        },
        ...now,
      ]);
      return id;
    },
    exit(jobId, how) {
      const holder = now.find((one) => one.attachments.some((a) => a.kind === "job" && a.id === jobId && a.state === "piloted"));
      if (holder === undefined) return;
      const id = holder.id;
      const settle = (state: "review" | "landed" | "superseded", status: string, said_: string, extra: Record<string, unknown> = {}) => {
        board.setStatus(jobId, status);
        edit(id, (one) => ({
          ...one,
          // The worktree goes back with the Job: the Session no longer holds its slot or its branch.
          attachments: one.attachments
            .filter((a) => !((a.kind === "slot" || a.kind === "branch") && a.handed !== undefined))
            .map((a) => (a.kind === "job" && a.id === jobId ? { ...a, state, ...extra } : a)),
          rows: [...one.rows, row((rid, stamp) => ({ id: rid, at: stamp, kind: "message", from: { kind: "agent" }, text: said_ }))],
          turn: idle,
          lastTurn: "14:41",
        }));
      };
      if (how === "submit") {
        board.setStatus(jobId, "running");
        edit(id, (one) => ({
          ...one,
          rows: [...one.rows, tool(`submit_for_verification ${(holder.attachments.find((a) => a.kind === "job") as { number: number }).number}`)],
          attachments: one.attachments.map((a) => (a.kind === "job" && a.id === jobId ? { ...a, state: "running" as const } : a)),
          turn: { state: "working" },
        }));
        after(1500, () => settle("review", "awaiting_review", "The step's Checks and Judge passed on the worktree. The slot is back with the Job."));
      } else if (how === "attest") {
        settle("landed", "completed_success", "Recorded as done by your word. The Job's gates did not run.", { attested: true });
      } else {
        settle("superseded", "superseded", "Closed as superseded. The Job record says the work landed outside it.");
      }
    },
    setTags: (id, tags) => edit(id, (one) => ({ ...one, pendingTags: tags })),
    send(id, sent) {
      if (sent.text.trim() === "" && sent.files.length + sent.sketches.length + sent.tags.length === 0) return;
      const taggedJobs = sent.tags.filter((t): t is SessionTag & { job: NonNullable<SessionTag["job"]> } => t.kind === "job" && t.job !== undefined);
      edit(id, (one) => {
        const { pendingTags, done, lapsed, standing, ...rest } = one;
        void pendingTags;
        void done;
        void lapsed;
        // A message from the person ends the reply and the done Fleet read off the turn before it.
        void standing;
        const looking = taggedJobs
          .filter((t) => !one.attachments.some((a) => a.kind === "job" && a.id === t.id))
          .map((t): SessionAttachment => ({ kind: "job", id: t.id, number: t.job.number, title: t.title.replace(/^\d+ /, ""), state: t.job.state, branch: t.job.branch, slot: t.job.slot, looking: true }));
        return {
          ...rest,
          ...(one.waitingFor === undefined ? {} : { waitingFor: one.waitingFor.filter((it) => it.source !== "replied" && it.source !== "done") }),
          rows: [
            ...one.rows,
            row((rid, stamp) => ({
              id: rid,
              at: stamp,
              kind: "message",
              instant: new Date().toISOString(),
              from: { kind: "you" },
              text: sent.text,
              ...(sent.files.length === 0 ? {} : { files: sent.files }),
              ...(sent.sketches.length === 0 ? {} : { sketches: sent.sketches.map((k) => ({ id: k.id, title: k.title })) }),
              ...(sent.tags.length === 0 ? {} : { tags: sent.tags }),
            })),
          ],
          // A sketch drawn for a Session is on its ledger, beside the ones the agent publishes.
          attachments: [...one.attachments, ...looking, ...sent.sketches.map((k): SessionAttachment => ({ kind: "sketch", id: k.id, title: k.title, by: "you", drawing: k.drawing }))],
          // A session in a terminal is not told to be working: Fleet reads its transcript and nothing else.
          turn: one.terminal === true ? one.turn : { state: "working" },
        };
      });
      if (now.find((one) => one.id === id)?.terminal === true) {
        // The terminal takes the message, and what it does shows in the thread a moment later.
        after(900, () => {
          addTo(id, [tool("Read crates/store/src/tests/ledger.rs:88"), said("Line 88. Waiting on the clock there instead of the sleep fixes it.")]);
          finishOf(id, "14:09");
        });
      } else if (id === SHOWER) {
        const had = asked.get(id) ?? 0;
        asked.set(id, had + 1);
        after(900, () => {
          if (had === 0) {
            addTo(id, [tool(`show_window ${SHOWN}`), said("The findings are in a window.")]);
            showWindow(id, mockPage(), SHOWN);
            finishOf(id, "14:40");
          } else {
            // Asked to change what the window shows, the Session says so on the window, and again when it is done.
            addTo(id, [said("Noted. I will change that.")]);
            markWalkWindow("working");
            after(2400, () => {
              addTo(id, [tool("Edit report/clock.tsx"), said("Changed. The window is ready again.")]);
              markWalkWindow("ready");
              finishOf(id, "14:41");
            });
          }
        });
      } else if (id === VARIER || id === WALKER) {
        const had = asked.get(id) ?? 0;
        asked.set(id, had + 1);
        after(900, () => {
          if (had === 0) {
            const walks = id === WALKER;
            addTo(id, [tool(`show_window ${walks ? WALKED : VARIED}, 3 variations`), said(walks ? "Three walks are in a window." : "Three layouts are in a window.")]);
            showWindow(id, (walks ? walkTakes() : takes())[0]!.url, walks ? WALKED : VARIED);
          } else {
            addTo(id, [said("Noted. I will change that.")]);
          }
          finishOf(id, "14:44");
        });
      } else if (id === SKETCHER) {
        const had = asked.get(id) ?? 0;
        asked.set(id, had + 1);
        after(900, () => {
          if (had === 0) {
            showSketch(id, READS_TITLE, READS);
            addTo(id, [said("Every write reads the wall clock. The test fails when the second turns between two writes.")]);
          } else if (had === 1) {
            showSketch(id, READS_TITLE, READS_AGAIN);
            addTo(id, [said("Added the sleep I tried. It hides the failure and leaves the cause.")]);
          } else if (had === 2) {
            showSketch(id, PINNED_TITLE, PINNED, { title: READS_TITLE, drawing: READS_AGAIN });
            addTo(id, [said("The clock is injected and the test pins it. The sleep is gone.")]);
          } else if (had === 3) {
            showSketch(id, SEQUENCE_TITLE, SEQUENCE);
            addTo(id, [said("Bridge starts it, Fleet runs the Drone, and the Drone writes to the store.")]);
          } else {
            showSketch(id, WORKFLOW_TITLE, WORKFLOW);
            addTo(id, [said("Build and Test run together, and the Gate waits for both.")]);
          }
          finishOf(id, "14:52");
        });
      } else if (id === HUBBER) {
        after(900, () => {
          showSketch(id, DATA_FLOW_TITLE, DATA_FLOW);
          addTo(id, [said("Bridge talks to Fleet alone. Drones send evidence back through Fleet, and the Check decides the step.")]);
          finishOf(id, "14:52");
        });
      } else if (id === MINE && !started) {
        started = true;
        after(600, firstTurn);
      } else if (now.find((one) => one.id === id)?.attachments.some((a) => a.kind === "job" && a.state === "piloted" && a.looking !== true)) {
        // A piloted Session: the person and the agent fix the Job's worktree together.
        after(900, () => {
          addTo(id, [
            tool("Read crates/retry/src/loop.rs"),
            tool("Edit crates/retry/src/loop.rs: stop after five attempts"),
            tool("cargo clippy -p retry"),
            said("The loop stops after five attempts and clippy is clean on the worktree."),
          ]);
          finishOf(id, "14:38");
        });
      } else if (id !== MINE) {
        // A Session handed a Job: the first ask is read with the fleet tools, the next acts on what it found.
        const had = asked.get(id) ?? 0;
        const held = now.find((one) => one.id === id)?.attachments.find((a) => a.kind === "job" && a.looking === true);
        if (held?.kind === "job") {
          const job = { kind: "job", id: held.id, title: held.title, job: { number: held.number, branch: held.branch, ...(held.slot === undefined ? {} : { slot: held.slot }), state: held.state } } as const;
          asked.set(id, had + 1);
          after(900, () => (had === 0 ? investigating(id, job) : redirecting(id, job)));
        }
      }
    },
    rename: (id, title) => edit(id, (one) => ({ ...one, title })),
    close: (id) => edit(id, (one) => ({ ...one, dead: "ended" as const })),
    tune: (id, tuning) =>
      edit(id, (one) => {
        const { model, effort, ...rest } = one;
        void model;
        void effort;
        return { ...rest, mode: tuning.mode, ...(tuning.model === null ? {} : { model: tuning.model }), ...(tuning.effort === null ? {} : { effort: tuning.effort }) };
      }),
    // Fleet asks the forge, which takes a moment: long enough to see the press underway.
    async act(id, number, what) {
      await new Promise<void>((resolve) => after(ACT_MS, resolve));
      const change = (one: SessionAttachment): SessionAttachment => {
        if (one.kind !== "pull_request" || one.number !== number) return one;
        if (what === "ready") return { ...one, state: "open" };
        if (what === "merge") return { ...one, state: "merged" };
        if (what === "auto_merge") return { ...one, auto: true };
        return one;
      };
      edit(id, (one) => ({
        ...one,
        attachments: [
          ...one.attachments.map(change),
          // Review is a Job on the code review workflow, and the Session holds it as it holds any it dispatched.
          ...(what === "review" ? [{ kind: "job", ...review, title: `Code review of #${number}`, state: "running" } as const] : []),
        ],
      }));
      return true;
    },
    openWindow,
    // What the mock's files hold, since none is on this machine: a picture for an image, text for the rest.
    readArtifact: async (_id, path) =>
      path.endsWith(".output")
        ? { ok: true, bytes: new TextEncoder().encode(BACKGROUND_OUTPUT[path] ?? ""), type: "text/plain" }
        : /\.(png|jpe?g|gif|webp)$/i.test(path)
        ? { ok: true, bytes: new TextEncoder().encode(PICTURE), type: "image/svg+xml" }
        : { ok: true, bytes: new TextEncoder().encode(NOTE), type: path.endsWith(".md") ? "text/markdown" : "text/plain" },
    models: MODELS,
    defaultModel: "sonnet",
    efforts: EFFORTS,
    commands: COMMANDS,
    answerWaiting: (id, itemId, given) => {
      const item = now.find((one) => one.id === id)?.waitingFor?.find((one) => one.id === itemId);
      if (item === undefined) return;
      // A reply to a replied or done Session is a message from him, which wakes it.
      if (item.source === "replied" || item.source === "done") return store.send(id, { text: given.text ?? "", files: [], sketches: [], tags: [] });
      const chosen = given.choice === undefined ? given.text : item.options?.[given.choice]?.label;
      edit(id, (one) => {
        const { asked, ...rest } = one;
        void asked;
        return { ...rest, waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== itemId) };
      });
      // An agent's own item reaches it as Fleet words it: the item quoted, then the answer.
      const sent: SessionRow[] =
        item.source === "agent" ? [row((rid, stamp) => ({ id: rid, at: stamp, kind: "message", from: { kind: "you" }, text: `Re: ${item.text}\n${chosen ?? ""}` }))] : [];
      addTo(id, [...sent, said(item.act?.kind === "run" ? "It ran clean." : `Going with ${chosen ?? "my own call"}.`)]);
    },
    // A fake terminal: the command is typed there and nothing comes back to the thread.
    command: () => undefined,
    run: (_id, command, on) => {
      // A fake runner: a few timed lines. `vitest` fails, `tail -f` never ends until it is stopped.
      const lines = RUN_OUTPUT.find(([match]) => command.includes(match))?.[1] ?? RUN_DEFAULT;
      const forever = command.includes("tail -f");
      let at = 0;
      let handle = 0;
      const step = () => {
        if (at < lines.length) {
          on({ kind: "out", text: `${lines[at]}\n` });
          at += 1;
        } else if (forever) on({ kind: "out", text: `build.log ${new Date().toLocaleTimeString("en-GB")} waiting\n` });
        else {
          const code = command.includes("vitest") ? 1 : 0;
          on({ kind: "end", code });
          // Fleet takes the item off the list on a clean exit and tells the agent; nothing in Bridge does.
          if (code === 0) {
            edit(_id, (one) => ({ ...one, waitingFor: (one.waitingFor ?? []).filter((it) => !(it.act?.kind === "run" && it.act.target.trim() === command.trim())) }));
            addTo(_id, [said("It ran clean.")]);
          }
          return;
        }
        handle = window.setTimeout(step, forever ? 700 : 450);
        timers.add(handle);
      };
      handle = window.setTimeout(step, 250);
      timers.add(handle);
      return {
        stop: () => {
          window.clearTimeout(handle);
          timers.delete(handle);
          on({ kind: "end", code: null });
        },
      };
    },
    // A repository's call has no Session: the same fake plays in its place.
    runInRepository: (_root, command, on) => store.run!("", command, on),
    answer: (id, answer, answers) => {
      const questioned = now.find((one) => one.id === id)?.asked?.questions !== undefined;
      edit(id, (one) => {
        const { asked, ...rest } = one;
        void asked;
        // The call that held the permission goes with it, as Fleet's does.
        const waitingFor = (one.waitingFor ?? []).filter((it) => it.source !== "permission");
        return questioned ? { ...rest, waitingFor, turn: { state: "working" } } : { ...rest, waitingFor };
      });
      if (!questioned) return;
      // What was chosen is the person's own message, as Fleet keeps it.
      if (answer !== "refuse" && (answers ?? []).length > 0) {
        addTo(id, [row((rid, stamp) => ({ id: rid, at: stamp, kind: "message", from: { kind: "you" }, text: (answers ?? []).map((one) => `**${one.question}**\n${one.chosen.join(", ")}`).join("\n\n") }))]);
      }
      // The agent carries on from what was chosen, or from having been told no.
      after(900, () => {
        addTo(id, [
          answer === "refuse"
            ? said("Skipped, so I will use the defaults.")
            : said(`Going with ${(answers ?? []).map((one) => one.chosen.join(" and ")).join(", ")}.`),
          tool("Edit lunch-order.md"),
        ]);
        finishOf(id, "14:31");
      });
    },
    subagent: async (_id, subagentId) => {
      const held = now.flatMap((one) => one.attachments).find((one) => one.kind === "subagent" && one.id === subagentId);
      if (held?.kind !== "subagent") return undefined;
      const script = SUBAGENT_SCRIPT;
      // A running subagent shows four rows and then two more each time it is read, until it has said its last; a done one shows all.
      const seen = held.state === "done" ? script.length : Math.min(script.length, 4 + 2 * (reads.get(subagentId) ?? 0));
      reads.set(subagentId, (reads.get(subagentId) ?? 0) + 1);
      const rows = script.slice(0, seen).map(([kind, text], index): SessionRow => {
        const id = `${subagentId}-${index}`;
        const stamp = `14:${String(10 + index).padStart(2, "0")}:00`;
        return kind === "tool"
          ? { id, at: stamp, kind: "tool", text }
          : { id, at: stamp, kind: "message", from: { kind: index === 0 ? "you" : "agent" }, text };
      });
      const finished = seen === script.length;
      return { rows, finished, ...(finished ? { report: script[script.length - 1]![1] } : {}) };
    },
    later() {
      const arrival = arrivals[arrived];
      if (arrival !== undefined) {
        arrived += 1;
        addTo(arrival[0], [said(arrival[1])]);
        return;
      }
      const next = turns[moment];
      moment += 1;
      next?.();
    },
    dispose: () => {
      timers.forEach((one) => window.clearTimeout(one));
      closeWalkWindow();
    },
    attach: (id, attachment) => addTo(id, [], [attachment]),
    edit,
    // Fleet drops an item from the list for good; one read off the pending ask goes with the ask.
    drop: (id, itemId, reason) => {
      const stated = now.find((one) => one.id === id)?.waitingFor?.find((it) => it.id === itemId && it.source === "agent");
      edit(id, (one) => {
        const { asked, ...rest } = one;
        const own = one.waitingFor?.some((it) => it.id === itemId) === true;
        // A reply or a done he dismissed is settled: the Session stands as idle, as Fleet reads it.
        const settled = itemId.startsWith("replied:") || itemId.startsWith("done:");
        return own ? { ...one, ...(settled ? { standing: "idle" as const } : {}), waitingFor: one.waitingFor!.filter((it) => it.id !== itemId) } : asked === undefined ? one : rest;
      });
      if (stated !== undefined) addTo(id, [row((rid, stamp) => ({ id: rid, at: stamp, kind: "message", from: { kind: "you" }, text: removedSaid(stated.text, reason) }))]);
    },
  };
  return store;
}
