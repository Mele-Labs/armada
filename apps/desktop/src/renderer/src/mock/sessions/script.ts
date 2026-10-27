// What the Sessions mock holds: a few Sessions, and what the walk's own Session
// does moment by moment. Mock-only. Nothing here reaches Fleet, and every field
// it holds is one Fleet would owe (`packages/screens/src/draft/sessions.ts`).
//
// **The agent is a script, not a model.** Each moment is what the person's
// Session would do next, applied when a walk's `later` step lets time pass or,
// for the first, when the person sends a message.

import { closeWalkWindow, markWalkWindow, mockPage, openWalkWindow, type Variation } from "@armada/screens/src/fixtures/walk-window";
import { askToOpenSession } from "../../open-session";
import { askToTell } from "../../tell";
import { walkCall, withWalkCall } from "../plan-call";
import { removedSaid } from "./removed";
import { AUTO_MERGE, AUTO_MERGE_TITLE } from "./auto-merge-sketch";
import { DATA_FLOW, DATA_FLOW_TITLE } from "./data-flow-sketch";
import { SHAPES, SHAPES_TITLE } from "./shapes-sketch";
import { FLOW_SCENE, FLOW_TITLE, MEMBERS_SCENE, MEMBERS_TITLE, OPEN_SCENE, OPEN_TITLE } from "./screens";
import { PINNED, PINNED_TITLE, READS, READS_AGAIN, READS_TITLE, SEQUENCE, SEQUENCE_TITLE, WORKFLOW, WORKFLOW_TITLE } from "./sketches";
import type { Scene } from "@armada/components";
import type { WorktreesHeld } from "@armada/protocol";
import type { QueuedIdea, Session, SessionAttachment, SessionCommand, SessionRow, SessionSketch, SessionWait, SessionsDraft, StartBase, StartWhen } from "@armada/screens/src/draft/sessions";
import { conditionSaid, firstLineOf, startsFrom } from "@armada/screens/src/session-waits";
import { readWait } from "./reads-waits";

/** The draft the window reads, and what only the mock does: take the next turn, stop, and take a claim Fleet would have put in the ledger. */
export type SessionsStore = SessionsDraft & {
  later: () => void;
  dispose: () => void;
  attach: (id: string, attachment: SessionAttachment) => void;
  /** Fleet drops an item for good. One the agent stated also reaches it as the person's message, with `reason` where he gave one. */
  drop: (id: string, itemId: string, reason?: string) => void;
  /** Fleet keeps an item read from a reply as an ask: it stays, and reads `confirmed` from then on. */
  confirm: (id: string, itemId: string) => void;
  /** Changes a Session as it stands, for a scenario that scripts one Session of its own. */
  edit: (id: string, change: (one: Session) => Session) => void;
  /**
   * The agent of the Session a waiting one waits on changes when it starts, as its tool call would: a tool row in its thread and a
   * row that tells the change. `pull` names the pull request a `pushed` or `lands` condition waits on.
   */
  agentChanges: (id: string, change: { when: StartWhen; part?: { id: string; title: string }; pull?: { number: number; branch: string }; base?: StartBase }) => void;
  /** A waiting Session's condition is met: it starts, its first message carries the handoff, and the Session it waited on is told. `met` is what happened. */
  starts: (id: string, met: string) => void;
  /** How slow Fleet is to answer the Cockpit's routes, and then to stop carrying what it answered: at once where absent. */
  pace?: Pace;
  /** How Fleet takes a message from the person: held until the walk lets it go, and refused where `outcomes` says so. Taken at once where absent. */
  delivery?: Delivery;
};

/** Each message waits on `gate`, then is taken or refused by the next of `outcomes`; with none left it is taken. */
export type Delivery = { gate: Gate; outcomes: ("taken" | "refused")[] };

/** Something a route waits on until it is let go: a walk's step, not a timer, so a person can sit on the wait. */
export type Gate = { wait: () => Promise<void>; release: () => void };

/** How long Fleet takes to answer, and then to stop carrying what it answered. Where `held` is set it waits on that instead of on the clocks. */
export type Pace = { answerMs: number; settleMs: number; held?: Gate };

/** The Session that draws a sketch each time it is spoken to: the first, the same title again, and a change to it. */
export const SKETCHER = "s14";

/** The Session that draws how data moves through Fleet: a hub of boxes, converging labelled arrows, a long body. */
export const HUBBER = "s15";

/** The Session that draws screens: a desktop window, a web app in a browser frame, and two phone screens joined by an arrow. */
export const WIRER = "s16";

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
/** How long after End answers a Job reads as ended: past the pane leaving, so the row settles after it. */
const END_MS = 700;

/** What a subagent's window holds: a running one starts at the first and takes in the second with each row it has drawn. */
const SUBAGENT_TOKENS_FROM = 18_000;
const SUBAGENT_TOKENS_PER_READ = 9_000;
const SUBAGENT_TOKENS_DONE = 142_000;
const SUBAGENT_WINDOW = 200_000;

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
    { name: "Every state", scenario: "default" },
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
    { name: "Artifacts", walk: "ledger-in-cockpit" },
    { name: "Task marks", walk: "taskMarkTooltips" },
    { name: "Sleep cockpit", walk: "sleepCockpit" },
  ].map(({ name, walk }) => {
    const url = new URL(mockPage());
    url.search = `?walk=${walk}&walked`;
    return { name, url: url.toString() };
  });

/** What the mock Fleet lets a Session do to a bay: the worktree pool, read as Fleet holds it and written back where the window is reading it. */
export type BayControl = {
  pool?: { get: () => WorktreesHeld | undefined; set: (held: WorktreesHeld) => void };
};

const PICTURE = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 200"><rect width="320" height="200" fill="#2a3340"/><rect x="24" y="24" width="272" height="28" rx="4" fill="#4b5a6e"/><rect x="24" y="68" width="120" height="108" rx="4" fill="#4b5a6e"/><rect x="160" y="68" width="136" height="48" rx="4" fill="#6b7f99"/><rect x="160" y="128" width="136" height="48" rx="4" fill="#4b5a6e"/></svg>`;

/** The last lines of what a background command wrote, by the file it writes them to. */
/** The watch logs a scenario's readings name, by path, as `read_watch_log` would read them. */
export const WATCH_LOGS: Record<string, string> = {};

const BACKGROUND_OUTPUT: Record<string, string> = {
  "/tmp/armada/tasks/serve-mock.output": "VITE v6.3.5  ready in 412 ms\n\n  Local:   http://127.0.0.1:5176/\n\nGET /?scenario=sessions 200 11ms\nGET /?scenario=sessions&walked 200 9ms\nGET /?scenario=sessions&walk=ledger-background 200 8ms\n",
  "/tmp/armada/tasks/watch-tests.output": "RUN  v3.2.4 /slots/slot-6/apps/desktop\n\n Test Files  3 passed (3)\n      Tests  41 passed (41)\n   Duration  6.18s\n\nWaiting for file changes...\n",
  "/tmp/armada/tasks/watch-ci.output": "Run ci (9321177) triggered by pull_request\n* check  passed  1m 12s\n* test   in progress\n* gate   queued\n",
  "/tmp/armada/tasks/build-renderer.output": "vite v6.3.5 building for production...\ntransforming...\n2214 modules transformed.\nout/renderer/index.html            0.62 kB\nout/renderer/assets/index.css    148.30 kB\nout/renderer/assets/index.js   1,904.77 kB\nbuilt in 9.41s\n",
  "/tmp/armada/tasks/tail-fleet.output": "2026-10-16T14:02:11Z INFO fleet: session aa994f6f woke\n2026-10-16T14:02:12Z INFO fleet: lease slot 6 for aa994f6f\n2026-10-16T14:02:40Z INFO fleet: check run 41 passed\n",
};

const NOTE = "# Store clock\n\nThe store reads the clock once per write.\n\n- Pinned in tests\n- Read through `Clock::now`\n";

/** What the fake runner prints for a command containing the first of each pair. */
export const RUN_OUTPUT: readonly (readonly [string, readonly string[]])[] = [
  ["typecheck", ["> @armada/desktop typecheck", "> tsc --noEmit -p tsconfig.web.json", "Found 0 errors."]],
  ["vitest", ["RUN  v3.2.4 packages/components", "✓ Prose.test.tsx (14)", "✗ SessionThread.test.tsx (1)", "AssertionError: expected 3 to equal 4", "Tests  1 failed | 41 passed"]],
  ["lsof", ["Stopping the server on 5788", "pid 41822 ended", "Port 5788 is free"]],
  ["tail -f", ["build.log: compiling", "build.log: 212 modules"]],
  ["--autostash", ["Created autostash: 4be1f3a", "Updating 6c0d2e1..a91f7b3", "Fast-forward", " armada.yml | 6 +-", " crates/fleet/src/checkouts.rs | 212 +++++++++", "Applied autostash"]],
];
export const RUN_DEFAULT = ["done"];

/** A Session as Fleet sends it: every one but Helm has work (`packages/screens/src/session-work.ts`), at least none of its own, so the Cockpit opens it as a Job. */
const asJob = (one: Session): Session => (one.helm === true || one.work !== undefined ? one : { ...one, work: {} });

export function sessionsStore(
  /** Sessions open beside the usual ones. */
  more: readonly Session[] = [],
  /** What an agent says to a Session, one message each time a walk lets time pass, before the usual turns. */
  arrivals: readonly (readonly [id: string, text: string])[] = [],
  /** The Manifest id of each repository by its root, as Fleet names the repository a Session starts in. A root not here leaves the Session with none. */
  owners: ReadonlyMap<string, string> = new Map(),
  /** The Sessions open beside `more`, which a scenario with none of its own passes empty. */
  usual: readonly Session[] = others(),
): SessionsStore {
  // A Session in `more` with a usual one's id stands in for it.
  let now: readonly Session[] = [...usual.filter((one) => !more.some((it) => it.id === one.id)), ...more].map(asJob);
  let clock = 0;
  let rowId = 0;
  let moment = 0;
  let arrived = 0;
  let started = false;
  let made = 0;
  let queued: readonly QueuedIdea[] = [];
  let queuing = 0;
  /** How many times a Session has been asked, to know which of its turns is next. */
  const asked = new Map<string, number>();
  const listeners = new Set<() => void>();
  const timers = new Set<number>();
  /** How many times each subagent's thread has been read, which is how far a running one has got. */
  const reads = new Map<string, number>();

  const set = (next: readonly Session[]) => {
    now = next.map(asJob);
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

  /** An instant that only moves forward, from 14:03 today, so a row's stamp never reads out of order. */
  const at = () => {
    clock += 7;
    return new Date(new Date().setHours(14, 3 + Math.floor(clock / 60), clock % 60, 0)).toISOString();
  };
  const row = (make: (id: string, stamp: string) => SessionRow): SessionRow => make(`r${(rowId += 1)}`, at());
  const tool = (text: string): SessionRow => row((id, stamp) => ({ id, at: stamp, kind: "tool", text }));
  const said = (text: string): SessionRow => row((id, stamp) => ({ id, at: stamp, kind: "message", from: { kind: "agent" }, text }));
  const addTo = (id: string, rows: SessionRow[], attachments: SessionAttachment[] = []) =>
    edit(id, (one) => ({ ...one, rows: [...one.rows, ...rows], attachments: [...one.attachments, ...attachments] }));
  const add = (rows: SessionRow[], attachments: SessionAttachment[] = []) => addTo(MINE, rows, attachments);
  const finishOf = (id: string) => edit(id, (one) => ({ ...one, turn: idle, lastTurnAt: new Date().toISOString() }));
  const finish = () => finishOf(MINE);

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
        // His Approve is what takes the page off Needs you.
        edit(id, (one) => ({ ...one, waitingFor: withWalkCall(one.waitingFor, url) }));
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
    holdWalk(id, url, title);
  };
  /** A shown page waits in Needs you until he approves it, whether or not its window is open. */
  const holdWalk = (id: string, url: string, title: string) =>
    edit(id, (one) => ({ ...one, waitingFor: withWalkCall(one.waitingFor, url, walkCall(url, title, new Date().toISOString())) }));
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
    finish();
  };

  const researching = () => {
    add(
      [
        tool("subagent: Read the CI history of store_flaky"),
        tool("subagent: Find other tests that read the wall clock"),
        tool("studio: Flaky store shapes"),
        tool("publish_sketch: CI history of store_flaky"),
        said("Two subagents reading."),
      ],
      [
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
        row((id, stamp) => ({ id, at: stamp, kind: "release", slot: 3, branch: "fix/flaky-store" })),
      ]);
      edit(MINE, (one) => ({ ...one, asked: { command: "git push --force-with-lease origin fix/flaky-store" } }));
      finish();
    });
  };

  const turns = [researching, opening, woken];

  const store: SessionsStore = {
    get: () => now,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    start(tag, root, request, tuning, after) {
      made += 1;
      const id = made === 1 && tag === undefined && after === undefined ? MINE : `s${7 + made}`;
      const manifestId = root === undefined ? undefined : owners.get(root);
      if (after !== undefined) {
        // Dispatched to wait: it holds its words and no slot, and the Session it waits on is told.
        const brief = (request ?? "").trim();
        const waiting: Session = {
          id,
          title: readWait(brief, now)?.title ?? firstLineOf(brief).split(/\s+/).slice(0, 4).join(" "),
          ...(manifestId === undefined ? {} : { manifestId }),
          attachments: [],
          rows: [],
          turn: idle,
          standing: "idle",
          ...(tuning?.model === undefined ? {} : { model: tuning.model }),
          ...(tuning?.effort === undefined ? {} : { effort: tuning.effort }),
          waits: { on: after.on, when: after.when, ...(after.part === undefined ? {} : { part: after.part }), base: after.base, needs: after.needs, brief, by: "you" },
        };
        set([waiting, ...now.map((one) => (one.id === after.on.id ? { ...one, rows: [...one.rows, row((rid, stamp): SessionRow => ({ id: rid, at: stamp, kind: "waiter", session: { id, title: waiting.title! }, event: "waits", says: after.needs === "" ? firstLineOf(brief) : after.needs }))] } : one))]);
        return id;
      }
      set([
        {
          id,
          ...(manifestId === undefined ? {} : { manifestId }),
          attachments: [],
          rows: [],
          turn: idle,
          ...(tag === undefined ? {} : { pendingTags: [tag] }),
          ...(tuning?.model === undefined ? {} : { model: tuning.model }),
          ...(tuning?.effort === undefined ? {} : { effort: tuning.effort }),
        },
        ...now,
      ]);
      return id;
    },
    readsWait: (words) => readWait(words, now),
    queued: () => queued,
    queue(idea) {
      queuing += 1;
      queued = [...queued, { id: `q${queuing}`, ...idea }];
      set([...now]);
    },
    startQueued(id) {
      const idea = queued.find((one) => one.id === id);
      if (idea === undefined) return Promise.resolve(undefined);
      const session = store.start(undefined, idea.root, idea.text, { ...(idea.model === undefined ? {} : { model: idea.model }), ...(idea.effort === undefined ? {} : { effort: idea.effort }) });
      if (typeof session !== "string") return session;
      store.dropQueued!(id);
      store.send(session, { text: idea.text, files: [], sketches: [], tags: [] });
      return session;
    },
    dropQueued(id) {
      queued = queued.filter((one) => one.id !== id);
      set([...now]);
    },
    startWhen(id, when, part) {
      const waiting = now.find((one) => one.id === id);
      if (waiting?.waits === undefined) return;
      const was = waiting.waits;
      const { part: _part, pull: _pull, ...rest } = was;
      void _part;
      void _pull;
      const next: SessionWait = { ...rest, when, ...(part === undefined ? {} : { part }), ...(when === was.when && when !== "plan" && was.pull !== undefined ? { pull: was.pull } : {}), by: "you" };
      edit(id, (one) => ({ ...one, waits: next }));
      addTo(was.on.id, [row((rid, stamp) => ({ id: rid, at: stamp, kind: "waiter", session: { id, title: waiting.title ?? id }, event: "changed", from: conditionSaid(was), to: conditionSaid(next), by: "you" }))]);
    },
    startNow: (id) => store.starts(id, "Started by you"),
    setTags: (id, tags) => edit(id, (one) => ({ ...one, pendingTags: tags })),
    confirmLater: (id, later) => edit(id, (one) => ({ ...one, later: (one.later ?? []).map((it) => (it.id === later ? { ...it, confirmed: true } : it)) })),
    dropLater: (id, later) => edit(id, (one) => ({ ...one, later: (one.later ?? []).filter((it) => it.id !== later) })),
    // As Fleet does: the watch ends stopped, and a Job it alone held open is held no more.
    stopWatch: (id, watch) =>
      edit(id, (one) => {
        const watches = (one.watches ?? []).map((it) => (it.id === watch ? { ...it, state: "stopped" as const, live: false } : it));
        const { heldOpenBy, ...rest } = one;
        const held = watches.find((it) => it.live && it.name === heldOpenBy);
        return { ...rest, watches, ...(held === undefined ? {} : { heldOpenBy: held.name }) };
      }),
    // As Fleet does: an ended watch goes, and a live one stays, since Fleet refuses it.
    deleteWatch: (id, watch) => edit(id, (one) => ({ ...one, watches: (one.watches ?? []).filter((it) => it.id !== watch || it.live) })),
    // A watch log is a file under Fleet's state on the real machine; here a scenario names its text.
    readWatchLog: async (_watch, log) => {
      const text = WATCH_LOGS[log];
      return text === undefined ? { ok: false, said: `No kept reading has the log ${log}.` } : { ok: true, text };
    },
    send(id, sent) {
      if (sent.text.trim() === "" && sent.files.length + sent.sketches.length + sent.tags.length === 0) return;
      edit(id, (one) => {
        const { pendingTags, done, lapsed, standing, ...rest } = one;
        void pendingTags;
        void done;
        void lapsed;
        // A message from the person ends the reply and the done Fleet read off the turn before it.
        void standing;
        return {
          ...rest,
          ...(one.waitingFor === undefined ? {} : { waitingFor: one.waitingFor.filter((it) => it.source !== "replied" && it.source !== "done") }),
          rows: [
            ...one.rows,
            row((rid, stamp) => ({
              id: rid,
              at: stamp,
              kind: "message",
              from: { kind: "you" },
              text: sent.text,
              ...(sent.files.length === 0 ? {} : { files: sent.files }),
              ...(sent.sketches.length === 0 ? {} : { sketches: sent.sketches.map((k) => ({ id: k.id, title: k.title })) }),
              ...(sent.tags.length === 0 ? {} : { tags: sent.tags }),
            })),
          ],
          // A sketch drawn for a Session is on its ledger, beside the ones the agent publishes.
          attachments: [...one.attachments, ...sent.sketches.map((k): SessionAttachment => ({ kind: "sketch", id: k.id, title: k.title, by: "you", drawing: k.drawing }))],
          // A session in a terminal is not told to be working: Fleet reads its transcript and nothing else.
          turn: one.terminal === true ? one.turn : { state: "working" },
        };
      });
      if (now.find((one) => one.id === id)?.terminal === true) {
        // The terminal takes the message, and what it does shows in the thread a moment later.
        after(900, () => {
          addTo(id, [tool("Read crates/store/src/tests/ledger.rs:88"), said("Line 88. Waiting on the clock there instead of the sleep fixes it.")]);
          finishOf(id);
        });
      } else if (id === SHOWER) {
        const had = asked.get(id) ?? 0;
        asked.set(id, had + 1);
        after(900, () => {
          if (had === 0) {
            addTo(id, [tool(`show_window ${SHOWN}`), said("The findings are in a window.")]);
            showWindow(id, mockPage(), SHOWN);
            finishOf(id);
          } else {
            // Asked to change what the window shows, the Session says so on the window, and again when it is done.
            addTo(id, [said("Noted. I will change that.")]);
            markWalkWindow("working");
            // A page being changed is not put to him until it is ready.
            edit(id, (one) => ({ ...one, waitingFor: withWalkCall(one.waitingFor, mockPage()) }));
            after(2400, () => {
              addTo(id, [tool("Edit report/clock.tsx"), said("Changed. The window is ready again.")]);
              markWalkWindow("ready");
              holdWalk(id, mockPage(), SHOWN);
              finishOf(id);
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
          finishOf(id);
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
          finishOf(id);
        });
      } else if (id === HUBBER) {
        const had = asked.get(id) ?? 0;
        asked.set(id, had + 1);
        after(900, () => {
          if (had === 0) {
            showSketch(id, DATA_FLOW_TITLE, DATA_FLOW);
            addTo(id, [said("Bridge talks to Fleet alone. Drones send evidence back through Fleet, and the Check decides the step.")]);
          } else if (had === 1) {
            showSketch(id, AUTO_MERGE_TITLE, AUTO_MERGE);
            addTo(id, [said("The queue reading never took the wait off, so the row stayed.")]);
          } else {
            showSketch(id, SHAPES_TITLE, SHAPES);
            addTo(id, [said("A request is read, checked against the store, and either built or sent to sign in.")]);
          }
          finishOf(id);
        });
      } else if (id === WIRER) {
        after(900, () => {
          showSketch(id, OPEN_TITLE, OPEN_SCENE);
          showSketch(id, MEMBERS_TITLE, MEMBERS_SCENE);
          showSketch(id, FLOW_TITLE, FLOW_SCENE);
          addTo(id, [said("Session screen, team settings and sign in flow.")]);
          finishOf(id);
        });
      } else if (id === MINE && !started) {
        started = true;
        after(600, firstTurn);
      }
    },
    rename: (id, title) => edit(id, (one) => ({ ...one, title })),
    // Fleet keeps the flag on the record, so the tile reads it back from the Session as it does on the wire.
    autoLand: async (id, on) => {
      edit(id, ({ autoLand: _, ...one }) => (on ? { ...one, autoLand: true as const } : one));
      return true;
    },
    // Fleet answers End at once and parks the Job in the background: it reads as ended a moment later.
    close: (id) => after(END_MS, () => void edit(id, (one) => ({ ...one, dead: "ended" as const }))),
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
      // A watch that ended not ours is Fleet's own ask: Ignore tells the agent nothing, Fix it is a message naming what broke.
      if (item.source === "watch_not_ours") {
        edit(id, (one) => ({ ...one, waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== itemId) }));
        if (chosen === "Fix it") {
          const watch = now.find((one) => one.id === id)?.watches?.find((it) => it.id === item.act?.target);
          const aside = watch?.set_aside ?? [];
          const parts = aside.map((it) => it.part).join(", ");
          const link = watch?.last?.kind === "read" ? watch.last.link : undefined;
          const text = [`Fix ${parts} in ${watch?.name ?? item.text}. The person pressed Fix it on what you set aside as not ours.`, ...aside.map((it) => `${it.part}, set aside: ${it.sentence}`), ...(link === undefined ? [] : [`The failing run: ${link}`])].join("\n");
          addTo(id, [row((rid, stamp) => ({ id: rid, at: stamp, kind: "message", from: { kind: "you" }, text })), said(`Looking at ${parts}.`)]);
        }
        return;
      }
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
            edit(_id, (one) => ({ ...one, waitingFor: (one.waitingFor ?? []).filter((it) => !(it.act?.kind === "run" && it.act.target.trim() === command.trim() && (it.steps ?? []).length === 0)) }));
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
        finishOf(id);
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
      // Its window fills as it is read: each read of a running one has taken in more.
      const tokens = held.state === "done" ? SUBAGENT_TOKENS_DONE : SUBAGENT_TOKENS_FROM + SUBAGENT_TOKENS_PER_READ * seen;
      return {
        rows,
        finished,
        ...(finished ? { report: script[script.length - 1]![1] } : {}),
        model: "vendor-opus-5-5",
        context: { tokens, window: SUBAGENT_WINDOW },
      };
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
    agentChanges(id, change) {
      const waiting = now.find((one) => one.id === id);
      if (waiting?.waits === undefined) return;
      const was = waiting.waits;
      const next: SessionWait = { on: was.on, when: change.when, ...(change.part === undefined ? {} : { part: change.part }), ...(change.pull === undefined ? {} : { pull: change.pull }), base: change.base ?? was.base, needs: was.needs, brief: was.brief, by: "agent" };
      edit(id, (one) => ({ ...one, waits: next }));
      addTo(was.on.id, [
        tool(`set_start_condition ${JSON.stringify({ session: id, when: change.when, ...(change.part === undefined ? {} : { part: change.part.id }) })}`),
        row((rid, stamp) => ({ id: rid, at: stamp, kind: "waiter", session: { id, title: waiting.title ?? id }, event: "changed", from: conditionSaid(was), to: conditionSaid(next), by: "agent" })),
      ]);
    },
    starts(id, met) {
      const waiting = now.find((one) => one.id === id);
      const wait = waiting?.waits;
      if (waiting === undefined || wait === undefined) return;
      const other = now.find((one) => one.id === wait.on.id);
      const last = [...(other?.rows ?? [])].reverse().find((one) => one.kind === "message" && one.from.kind === "agent");
      const lines = [
        `Handoff from ${wait.on.title}.`,
        `${met}.`,
        `You start from ${startsFrom(wait)}.`,
        ...(last?.kind === "message" ? [`It last said: ${firstLineOf(last.text)}`] : []),
        `Your brief: ${firstLineOf(wait.brief)}`,
      ];
      edit(id, (one) => {
        const { waits: _waits, ...rest } = one;
        void _waits;
        return {
          ...rest,
          rows: [row((rid, stamp) => ({ id: rid, at: stamp, kind: "message", from: { kind: "session", id: wait.on.id, title: wait.on.title }, text: lines.join("\n") }))],
          turn: { state: "working" },
          standing: "working",
          lastTurnAt: new Date().toISOString(),
        };
      });
      addTo(wait.on.id, [row((rid, stamp) => ({ id: rid, at: stamp, kind: "waiter", session: { id, title: waiting.title ?? id }, event: "started", says: `From ${startsFrom(wait)}` }))]);
    },
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
    confirm: (id, itemId) => edit(id, (one) => ({ ...one, waitingFor: one.waitingFor?.map((it) => (it.id === itemId ? { ...it, confirmed: true } : it)) })),
  };
  // A message goes through `delivery` where a scenario sets one: the thread then shows it going, and refused.
  const sendNow = store.send;
  store.send = (id, sent) => {
    const held = store.delivery;
    if (held === undefined) return sendNow(id, sent);
    return held.gate.wait().then(() => {
      if (held.outcomes.shift() === "refused") {
        const title = now.find((one) => one.id === id)?.title ?? "the Session";
        askToTell(`Message to ${title} not sent. Fleet did not take it`);
        return false;
      }
      sendNow(id, sent);
      return true;
    });
  };
  return store;
}
