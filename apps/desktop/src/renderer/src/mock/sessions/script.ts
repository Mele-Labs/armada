// What the Sessions mock holds: a few Sessions, and what the walk's own Session
// does moment by moment. Mock-only. Nothing here reaches Fleet, and every field
// it holds is one Fleet would owe (`packages/screens/src/draft/sessions.ts`).
//
// **The agent is a script, not a model.** Each moment is what the person's
// Session would do next, applied when a walk's `later` step lets time pass or,
// for the first, when the person sends a message.

import type { Session, SessionTag, SessionAttachment, SessionCommand, SessionRow, SessionSketch, SessionsDraft } from "@armada/screens/src/draft/sessions";

/** The draft the window reads, and the two things only the mock does: take the next turn, and stop. */
export type SessionsStore = SessionsDraft & { later: () => void; dispose: () => void };

/** A Job the walk's Session dispatches. The real Board holds it, so its id is the Board's. */
export type DispatchedJob = { id: string; number: number; title: string; branch: string; slot: number };

const idle = { state: "idle" } as const;

/** The models Fleet lists, as `list_models` answers them, and the efforts a Drone's settings offer. */
const MODELS = ["haiku", "sonnet", "opus"];
const EFFORTS = ["low", "medium", "high"];

/** What `/` offers, as a terminal session lists them: skills first, then the commands. */
const COMMANDS: readonly SessionCommand[] = [
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
        { kind: "pull_request", number: 1847, title: "Release notes from merged pull requests", branch: "rel/notes-script", address: "https://example.com/pull/1847", checks: { state: "passed" }, state: "open", auto: false },
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

export const MINE = "s7";

/** A Job on the Board that a person can tag: where it stands, and how the Session would hold it. */
export type TaggableJob = DispatchedJob & { state: "escalated" | "review" };

export function sessionsStore(jobs: readonly [DispatchedJob, DispatchedJob], review: DispatchedJob, taggableJobs: readonly TaggableJob[]): SessionsStore {
  let now: readonly Session[] = others();
  let clock = 0;
  let rowId = 0;
  let moment = 0;
  let started = false;
  let made = 0;
  /** How many times a Session that was handed a Job has been asked, to know which of its two turns is next. */
  const asked = new Map<string, number>();
  const listeners = new Set<() => void>();
  const timers = new Set<number>();

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
  const finishOf = (id: string, stamp: string) => edit(id, (one) => ({ ...one, turn: idle, lastTurn: stamp }));
  const finish = (stamp: string) => finishOf(MINE, stamp);
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

  return {
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
      const fromSessions = now.flatMap((one) => [
        ...one.attachments.flatMap((a): SessionTag[] =>
          a.kind === "pull_request"
            ? [{ kind: "pull_request", id: String(a.number), title: `#${a.number} ${a.title}` }]
            : a.kind === "branch"
              ? [{ kind: "branch", id: a.name, title: a.name }]
              : [],
        ),
      ]);
      const jobsOnBoard = taggableJobs.map((j): SessionTag => ({
        kind: "job",
        id: j.id,
        title: `${j.number} ${j.title}`,
        job: { number: j.number, branch: j.branch, slot: j.slot, state: j.state === "escalated" ? "escalated" : "review" },
      }));
      return [...jobsOnBoard, ...fromSessions];
    },
    setTags: (id, tags) => edit(id, (one) => ({ ...one, pendingTags: tags })),
    send(id, sent) {
      if (sent.text.trim() === "" && sent.files.length + sent.sketches.length + sent.tags.length === 0) return;
      const taggedJobs = sent.tags.filter((t): t is SessionTag & { job: NonNullable<SessionTag["job"]> } => t.kind === "job" && t.job !== undefined);
      edit(id, (one) => {
        const { pendingTags, ...rest } = one;
        void pendingTags;
        const looking = taggedJobs
          .filter((t) => !one.attachments.some((a) => a.kind === "job" && a.id === t.id))
          .map((t): SessionAttachment => ({ kind: "job", id: t.id, number: t.job.number, title: t.title.replace(/^\d+ /, ""), state: t.job.state, branch: t.job.branch, slot: t.job.slot, looking: true }));
        return {
          ...rest,
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
          attachments: [...one.attachments, ...looking, ...sent.sketches.map((k): SessionAttachment => ({ kind: "sketch", id: k.id, title: k.title, by: "you", drawing: k.drawing }))],
          turn: { state: "working" },
        };
      });
      if (id === MINE && !started) {
        started = true;
        after(600, firstTurn);
      } else if (id !== MINE) {
        // A Session handed a Job: the first ask is read with the fleet tools, the next acts on what it found.
        const had = asked.get(id) ?? 0;
        const held = now.find((one) => one.id === id)?.attachments.find((a) => a.kind === "job" && a.looking === true);
        if (held?.kind === "job") {
          const job = { kind: "job", id: held.id, title: held.title, job: { number: held.number, branch: held.branch, slot: held.slot, state: held.state } } as const;
          asked.set(id, had + 1);
          after(900, () => (had === 0 ? investigating(id, job) : redirecting(id, job)));
        }
      }
    },
    tune: (id, tuning) =>
      edit(id, (one) => {
        const { model, effort, ...rest } = one;
        void model;
        void effort;
        return { ...rest, mode: tuning.mode, ...(tuning.model === null ? {} : { model: tuning.model }), ...(tuning.effort === null ? {} : { effort: tuning.effort }) };
      }),
    act(id, number, what) {
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
    },
    models: MODELS,
    efforts: EFFORTS,
    commands: COMMANDS,
    answer: (id) =>
      edit(id, (one) => {
        const { asked, ...rest } = one;
        void asked;
        return rest;
      }),
    later() {
      const next = turns[moment];
      moment += 1;
      next?.();
    },
    dispose: () => timers.forEach((one) => window.clearTimeout(one)),
  };
}
