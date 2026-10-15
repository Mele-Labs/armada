// The whole ship at rest, in the wire shapes Fleet serves, for the flight deck: two repositories, four Jobs, seven Sessions in
// the states Fleet reads them to be in, four pull requests each owned by a Job, a Session or nobody, main green, eight bays
// (five held, one free, one stranded, one closed), Autopilot off.
//
// **Everything the deck reads is something Fleet records.** The tools a drone has used are `session.read`, `session.edited`,
// `session.ran`, `job.read` and `job.ran` bursts with the `items` they touched, some spelled under the bay's checkout and some
// from the home directory. The bays' holders are `slot.leased` and `slot.released` events with `detail "slot-N"`. The write
// clashes are the `GET /clashes` list. A bay with no slot event (the free one, the closed one) has no history, and a drone
// that edited nothing is observed to touch nothing. Two `later` moments carry the story: the Session that was working
// replies, main goes red once PR 1890 lands, and the fix lands. #2301.

import { repository } from "@armada/screens/src/fixtures/build/base";
import type { HubJob, HubMerged, HubPullRequest, MainStanding, MergeLines, PlanUsage, TimelineEvent, TimelineSubject, WorktreeSlot, WorktreesHeld, WriteClash } from "@armada/protocol";
import type { Session, SessionAttachment, SessionChecks } from "@armada/screens/src/draft/sessions";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import { queued, review, running } from "@armada/jobs/fixtures/build/index";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { mockAreas } from "../cockpit-areas";
import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore, type SessionsStore } from "../sessions/script";

const ARMADA = repository().manifest!.id;
const POCKET = "pocket";
const POCKET_ROOT = "/Users/user/pocket";
const FORGE = "https://git.example/armada/pull/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** A Job of the repository `manifest`, started `minutes` ago, on the step `step` of its workflow. */
function live(fixture: JobFixture, minutes: number, step?: string, extra: Record<string, unknown> = {}): JobFixture {
  const patch = { owner_manifest_id: ARMADA, started_at: ago(minutes), ...(step === undefined ? {} : { current_step_id: step }), ...extra };
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, ...patch };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = { ...fixture.watched.detail.job, ...patch };
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

/** Step 3 of the bug workflow's six: Fix. */
const cache = live(asRow(running(), 81, "cache", "Cache the manifest read between dispatches"), 17, "fix", { branch: "armada/81-cache" });
/** Done with its work and waiting on him; its pull request is checking. */
const pause = live(asRow(review(), 82, "pause", "Add the pause marker column"), 95, undefined, { branch: "armada/82-pause-column" });
/** Its store Check failed and it is asking what to do. */
const reconnect = live(asRow(running(), 83, "reconnect", "Shorten the reconnect wait"), 33, "fix", { asking: true, branch: "armada/83-reconnect" });
/** Not started: waiting for a bay in the pocket repository. */
const code = live(asRow(queued(), 84, "code", "Read the pairing code from the runtime file"), 4, undefined, { owner_manifest_id: POCKET });
const JOBS = [cache, pause, reconnect, code];
/** Dispatched at the flight deck's second moment: a drone that was not there lifts from its base. */
const dispatched = live(asRow(running(), 85, "lease", "Retry the claim on a stale lease"), 0, "fix", { branch: "armada/85-lease" });

const say = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "agent" }, text }) as const;
const ask = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "you" }, text }) as const;
const tool = (id: string, at: string, text: string) => ({ id, at, kind: "tool", text }) as const;

const pr = (number: number, title: string, branch: string, checks: SessionChecks, more: { queued?: true } = {}): SessionAttachment => ({
  kind: "pull_request",
  number,
  title,
  branch,
  address: `${FORGE}${number}`,
  checks,
  state: "open",
  auto: false,
  ...more,
});

/** Its question is open and it waits on him. */
const asking: Session = {
  id: "q1",
  title: "Cache key",
  manifestId: ARMADA,
  context: { tokens: 54_000, window: 200_000 },
  turn: { state: "idle" },
  lastTurn: "14:09",
  lastTurnAt: ago(9),
  standing: "idle",
  rows: [ask("q1-u", "14:00:00", "Cache the manifest read."), say("q1-a", "14:09:20", "I can key the cache on the manifest hash or on its path.")],
  attachments: [],
  asked: {
    command: "Key the cache on the manifest hash or on its path?",
    call: "q1-ask",
    offers: ["allow_once", "refuse"],
    questions: [
      {
        question: "Key the cache on the manifest hash or on its path?",
        header: "Key",
        multi_select: false,
        options: [
          { label: "Hash", description: "Stays valid when the file moves" },
          { label: "Path", description: "Simpler, and stale after a rename" },
        ],
      },
    ],
  },
};

/** At work now; at the first moment its turn ends on a reply to him. */
const WORKING_REPLY: Session = {
  id: "r1",
  title: "Pin the store clock",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurn: "14:11",
  lastTurnAt: ago(1),
  standing: "working",
  rows: [ask("r1-u", "14:02:00", "Pin the clock in the store tests."), tool("r1-t", "14:11:10", "Edit crates/store/tests/clock.rs")],
  attachments: [{ kind: "branch", name: "fix/pin-store-clock", slot: 4 }],
};

const done: Session = {
  id: "d1",
  title: "Changelog script",
  manifestId: ARMADA,
  turn: { state: "idle" },
  lastTurn: "14:14",
  lastTurnAt: ago(4),
  standing: "done",
  done: { since: ago(4) },
  rows: [ask("d1-u", "14:01:00", "Write the changelog script and merge it."), say("d1-a", "14:14:30", "The script is merged. There is nothing left to do.")],
  attachments: [],
  waitingFor: [{ id: `done:${ago(4)}`, text: "Done", since: ago(4), source: "done" }],
};

const working: Session = {
  id: "w1",
  title: "Migration notes",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurn: "14:17",
  lastTurnAt: ago(1),
  standing: "working",
  rows: [
    ask("w1-u", "14:10:00", "Summarise the migrations since 0042."),
    tool("w1-t1", "14:15:41", "Read crates/store/migrations/0042.sql"),
    tool("w1-t2", "14:16:20", "Read crates/store/migrations/0043.sql"),
    say("w1-a", "14:16:58", "0043 adds the pause marker column and backfills it to null."),
    tool("w1-t3", "14:17:02", "Grep crates/store -e pause_marker"),
  ],
  attachments: [],
};

/** Its turn is running and has said nothing for 25 minutes. In the pocket repository. */
const stuck: Session = {
  id: "k1",
  title: "Service worker audit",
  manifestId: POCKET,
  turn: { state: "working" },
  lastTurn: "13:50",
  lastTurnAt: ago(25),
  standing: "stuck",
  rows: [ask("k1-u", "13:40:00", "List what the service worker caches."), tool("k1-t", "13:50:11", "Grep apps/pocket/src -e caches.open")],
  attachments: [{ kind: "branch", name: "pocket/sw-audit", slot: 2 }],
};

/** At work, its pull request in the merge queue. */
const queuedOwner: Session = {
  id: "p1",
  title: "Gate policy on every run",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurn: "14:16",
  lastTurnAt: ago(2),
  standing: "working",
  rows: [
    ask("p1-u", "13:20:00", "Run the gate on every run, not only the last."),
    tool("p1-t1", "14:14:02", "cargo test -p fleet gate_policy"),
    say("p1-a", "14:15:30", "The gate runs on every run. The pull request is in the queue."),
    tool("p1-t2", "14:16:10", "gh pr view 1890 --json mergeStateStatus"),
  ],
  attachments: [
    { kind: "branch", name: "fleet/gate-policy-every-run", slot: 1 },
    pr(1890, "Gate policy on every run", "fleet/gate-policy-every-run", { state: "passed" }, { queued: true }),
  ],
};

/** Idle, and its pull request's Check is red. */
const red: Session = {
  id: "f1",
  title: "Fix the reconnect wait",
  manifestId: ARMADA,
  turn: { state: "idle" },
  lastTurn: "13:58",
  lastTurnAt: ago(22),
  standing: "idle",
  rows: [
    ask("f1-u", "13:30:00", "Cut the first reconnect wait to five seconds."),
    tool("f1-t", "13:55:40", "gh pr create"),
    say("f1-a", "13:58:12", "The pull request is open. desktop_test is running."),
  ],
  attachments: [pr(1896, "Fix the reconnect wait", "fleet/reconnect-wait", { state: "failed", failing: "desktop_test" })],
};

const SESSIONS: Session[] = [asking, WORKING_REPLY, done, working, stuck, queuedOwner, red];

/** Fleet's reading of the Session at the first moment: its turn ended on a message and a question to him. A reply alone raises no call, so the question is what does. */
const REPLIED: Partial<Session> = {
  turn: { state: "idle" },
  standing: "replied",
  lastTurn: "14:18",
  rows: [
    ...WORKING_REPLY.rows,
    say("r1-a", "14:18:09", "The clock is pinned in all four store tests and the fixture is gone.\nTwo files changed, and the suite passes."),
  ],
};

/** The Sessions, with the moment that moves them, and Stop, which Fleet answers and the mock's fixtures do not. */
function sessions(control: Parameters<typeof sessionsStore>[3], list: readonly Session[] = SESSIONS): SessionsStore {
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  const store = sessionsStore([none, none], none, [], control, [], list);
  let moment = 0;
  // The store carries the usual Sessions beside these; the ship holds only its own seven.
  const own = new Set(list.map((one) => one.id));
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  // Its turn is running, so rows keep arriving once something watches it: a Peek that is open sees them land.
  let fed = false;
  const feed = [
    tool("w1-t4", "14:17:40", "Read crates/store/migrations/0044.sql"),
    say("w1-b", "14:18:05", "0044 drops the legacy index and adds no columns."),
    tool("w1-t5", "14:18:30", "Grep crates/store -e legacy_index"),
  ] as const;
  return {
    ...store,
    watch: (id: string) => {
      if (id !== "w1" || fed) return;
      fed = true;
      feed.forEach((row, at) => window.setTimeout(() => store.edit("w1", (one) => (one.turn.state === "working" ? { ...one, rows: [...one.rows, row] } : one)), 1500 * (at + 1)));
    },
    get: () => {
      const all = store.get();
      if (all !== from) {
        from = all;
        only = all.filter((one) => own.has(one.id));
      }
      return only;
    },
    stop: (id) => store.edit(id, (one) => ({ ...one, turn: { state: "idle" }, standing: "idle", lastTurnAt: new Date().toISOString() })),
    later() {
      moment += 1;
      if (moment !== 1) return;
      const at = new Date().toISOString();
      store.edit("r1", (one) => ({ ...one, ...REPLIED, lastTurnAt: at, waitingFor: [{ id: `q:${at}`, text: "Delete the fixture file, or keep it for the other suites?", since: at, source: "ask_card", options: [{ label: "Delete it" }, { label: "Keep it" }] }] }));
    },
  };
}

const hubJob = (one: JobFixture): HubJob => ({ id: one.job.id, title: one.job.title });
const COMMIT = "9f3c1d2e4b5a69788796a5b4c3d2e1f001122334";
const READ_AT = new Date().toISOString();

const GREEN: MainStanding = { state: "green", commit: COMMIT, read_at: READ_AT };
const RED: MainStanding = {
  state: "red",
  commit: COMMIT,
  read_at: READ_AT,
  red_since: READ_AT,
  failed: [{ name: "desktop_test", check: "desktop_test", tests: ["resize.test.ts > keeps the sidebar width"] }],
  merge: { number: 1890, url: `${FORGE}1890`, branch: "fleet/gate-policy-every-run", session: { id: queuedOwner.id, title: queuedOwner.title } },
};

const open = (number: number, title: string, branch: string, ci: string, more: Partial<HubPullRequest> = {}): HubPullRequest => ({ number, title, branch, url: `${FORGE}${number}`, ci, ...more });
const merged = (number: number, title: string, branch: string, minutes: number, run: "passed" | "failed"): HubMerged => ({
  number,
  title,
  branch,
  url: `${FORGE}${number}`,
  merged_at: ago(minutes),
  main_run: run === "passed" ? { state: "passed" } : { state: "failed", failed: ["desktop_test"] },
});

/** The forge as the ship has it at one moment: 1890 queued or landed, and main green, red, or green again. */
function lines(moment: "rest" | "red" | "fixed"): MergeLines {
  const landed = moment !== "rest";
  const armada = {
    root: repository().root,
    line: [],
    off: [],
    landed: [],
    sent_back: [],
    hub: {
      main: moment === "red" ? RED : GREEN,
      pull_requests: [
        ...(landed ? [] : [open(1890, "Gate policy on every run", "fleet/gate-policy-every-run", "passed", { queue: { state: "queued", position: 1 }, opened_at: ago(70), updated_at: ago(3) })]),
        open(1896, "Fix the reconnect wait", "fleet/reconnect-wait", "failed", { opened_at: ago(40) }),
        open(1892, "Add the pause marker column", pause.job.branch ?? "armada/82-pause-column", "running", { job: hubJob(pause), opened_at: ago(30) }),
      ],
      merged: [
        ...(landed ? [merged(1890, "Gate policy on every run", "fleet/gate-policy-every-run", 1, moment === "red" ? "failed" : "passed")] : []),
        merged(1888, "Bump the lockfile", "chore/bump-the-lockfile", 50, "passed"),
      ],
    },
  };
  const pocket = {
    root: POCKET_ROOT,
    line: [],
    off: [],
    landed: [],
    sent_back: [],
    hub: {
      main: GREEN,
      pull_requests: [open(212, "Bump the manifest icons", "mia/manifest-icons", "passed", { author: "mia", opened_at: ago(120) })],
      merged: [],
    },
  };
  return { lines: [armada, pocket] };
}

const bay = (manifest: string, slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({
  manifest_id: manifest,
  slot,
  path: `/Users/user/${manifest === ARMADA ? "armada" : manifest}/.armada/slots/slot-${slot}`,
  base: "main",
  warm: true,
  behind: 0,
  held,
  ...rest,
});

/** Eight bays: held by two Jobs and three Sessions, one free, one stranded, one closed. */
function pool(): WorktreesHeld {
  const session = (manifest: string, slot: number, one: Session, branch: string, minutes: number) =>
    bay(manifest, slot, { state: "hosted_session", session_id: one.id, session_title: one.title ?? one.id }, { branch, since: ago(minutes) });
  return {
    worktrees: [],
    slots: [
      session(ARMADA, 1, queuedOwner, "fleet/gate-policy-every-run", 80),
      bay(ARMADA, 2, { state: "job", job_id: cache.job.id, job_title: cache.job.title }, { branch: "armada/81-cache", since: ago(17) }),
      bay(ARMADA, 3, { state: "job", job_id: reconnect.job.id, job_title: reconnect.job.title }, { branch: "armada/83-reconnect", since: ago(33) }),
      session(ARMADA, 4, WORKING_REPLY, "fix/pin-store-clock", 16),
      bay(ARMADA, 5, { state: "free" }, { behind: 2, warm: false }),
      bay(
        ARMADA,
        6,
        { state: "stranded", why: "2 uncommitted, first crates/ipc/src/frame.rs" },
        {
          branch: "session/frame-the-reply",
          since: ago(3 * 60),
          behind: 4,
          stranded: {
            uncommitted: ["crates/ipc/src/frame.rs", "notes/frame.md"],
            commits: [{ sha: "e08c4d1a77", subject: "Frame the reply once", home: "only_here" }],
            unpushed: 1,
            last_holder: { kind: "session", id: "01SESSIONCLOSEDFRAME0000", title: "Frame the reply", open: false },
            told: ago(95),
          },
          rescue: { state: "answered", commit: "e08c4d1a77", uncommitted: true, read: ["crates/ipc/src/frame.rs"], searched: ["frame in crates/ipc/src/"], verdict: "unfinished", items: ["Wire the frame into the reply", "Add the test for a short frame"] },
        },
      ),
      session(POCKET, 1, stuck, "pocket/sw-audit", 40),
      bay(POCKET, 2, { state: "free" }, { behind: 1, closed: true }),
    ].map((one) => (one.manifest_id === POCKET ? { ...one, path: one.path.replace(/slots\/slot-\d+/, `slots/slot-${one.slot}`) } : one)),
  };
}

/** What each Job's Drone is doing: the lines a Peek reads. */
const NOWS: Record<string, NowView> = {
  [cache.job.id]: {
    running: [
      { key: "d", of: "drone", name: "Drone on Fix", line: "Editing manifest-read.ts", step: { id: "fix", name: "Fix" }, state: "running", tail: ["Read crates/manifest/src/read.rs", "Edit crates/manifest/src/read.rs: key the cache on the hash", "cargo test -p manifest read_again"] },
      { key: "c", of: "check", name: "manifest_test", state: "running" },
      { key: "l", of: "check", name: "lint", state: "running" },
    ],
  },
  [reconnect.job.id]: {
    running: [
      { key: "d", of: "drone", name: "Drone on Fix", line: "Waiting on the store test", state: "running", tail: ["Edit crates/fleet/src/reconnect.rs", "cargo test -p store reconnect", "FAIL store::reconnect::waits_five_seconds"] },
      { key: "c", of: "check", name: "store", state: "failed" },
    ],
  },
  [code.job.id]: { waiting: [{ key: "w", kind: "resource", text: "A bay in pocket" }] },
};

const CALLS: Record<string, CallView> = {
  [reconnect.job.id]: {
    request: "When Bridge loses its socket to Fleet, it waits a flat 30s before reconnecting. Cut the first wait to 5s and back off from there, keeping the cap at 60s.",
    running: [{ key: "r", of: "drone", name: "Drone on Fix", line: "Waiting on the store test", state: "running" }],
    issues: [
      {
        key: "i1",
        of: "check",
        name: "store",
        text: "store: 2 failed",
        said: "Check failed",
        context: ["FAIL store::reconnect::waits_five_seconds", "  expected: 5s", "  actual:   30s", "FAIL store::reconnect::caps_at_sixty", "  assertion failed: backoff(9) <= 60s"],
      },
    ],
  },
};

// ---- What Fleet did, for the flight deck's timeline ---------------------------------------------------

let count = 0;
type Who = { subject: TimelineSubject; title: string };
const jobWho = (one: JobFixture): Who => ({ subject: { kind: "job", id: one.job.id }, title: one.job.title });
const sessionWho = (one: Session): Who => ({ subject: { kind: "session", id: one.id }, title: one.title ?? one.id });
const pullWho = (number: number, title: string): Who => ({ subject: { kind: "pull_request", number, url: `${FORGE}${number}` }, title });
function event(minutes: number, who: Who, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, rest: { detail?: string; items?: string[]; live?: boolean; over?: boolean; repo?: string } = {}): TimelineEvent {
  count += 1;
  const { repo, ...more } = rest;
  return { id: count, at: ago(minutes), kind, subject: who.subject, title: who.title, tone, says, manifest_id: repo ?? ARMADA, ...more };
}

const BASE_EVENTS: TimelineEvent[] = [
  event(19, jobWho(cache), "job.created", "idle", "Dispatched"),
  event(18, jobWho(cache), "job.started", "run", "Started", { live: true }),
  event(14, jobWho(cache), "job.step", "ok", "Scope passed", { live: true }),
  event(9, jobWho(cache), "job.step", "run", "Fix started", { live: true }),
  event(97, jobWho(pause), "job.created", "idle", "Dispatched"),
  event(96, jobWho(pause), "job.started", "run", "Started", { live: true }),
  event(52, jobWho(pause), "job.step", "ok", "Implement passed", { live: true }),
  event(33, jobWho(pause), "job.step", "ask", "Awaiting review"),
  event(35, jobWho(reconnect), "job.created", "idle", "Dispatched"),
  event(34, jobWho(reconnect), "job.started", "run", "Started", { live: true }),
  event(20, jobWho(reconnect), "job.step", "ok", "Scope passed", { live: true }),
  event(6, jobWho(reconnect), "job.step", "bad", "store failed", { detail: "store::reconnect::waits_five_seconds", live: true }),
  event(5, jobWho(reconnect), "job.asked", "ask", "Asked: what to do about the store Check"),
  event(4, jobWho(code), "job.queued", "idle", "Queued, waits for a bay in pocket", { repo: POCKET }),
  event(70, sessionWho(queuedOwner), "session.started", "run", "Started", { live: true }),
  event(3, pullWho(1890, "Gate policy on every run"), "pr.opened", "idle", "Opened", { detail: "fleet/gate-policy-every-run" }),
  event(2, pullWho(1890, "Gate policy on every run"), "pr.checks", "ok", "Checks passed"),
  event(60, sessionWho(WORKING_REPLY), "session.started", "run", "Started", { live: true }),
  event(25, sessionWho(WORKING_REPLY), "session.turn", "run", "Editing the store tests", { live: true }),
  event(80, sessionWho(asking), "session.started", "run", "Started", { live: true }),
  event(9, sessionWho(asking), "session.turn", "ask", "Replied"),
  event(40, sessionWho(red), "session.started", "run", "Started", { live: true }),
  event(40, pullWho(1896, "Fix the reconnect wait"), "pr.opened", "idle", "Opened", { detail: "fleet/reconnect-wait" }),
  event(22, pullWho(1896, "Fix the reconnect wait"), "pr.checks", "bad", "Checks failed", { detail: "desktop_test" }),
  event(30, jobWho(pause), "pr.opened", "idle", "Opened"),
  event(50, sessionWho(done), "session.started", "run", "Started", { live: true }),
  event(4, sessionWho(done), "session.ended", "ok", "Done", { over: true }),
  event(12, sessionWho(working), "session.started", "run", "Started", { live: true }),
  event(1, sessionWho(working), "session.turn", "run", "Reading migrations", { live: true }),
  event(45, sessionWho(stuck), "session.started", "run", "Started", { live: true, repo: POCKET }),
  event(25, sessionWho(stuck), "session.turn", "idle", "Quiet", { repo: POCKET }),
];

/** Where a bay's checkout is, as a tool burst spells a path under it. */
const slotPath = (root: string, slot: number) => `${root}/.armada/slots/slot-${slot}`;
const ARMADA_ROOT = "/Users/user/armada";

/**
 * What a tool did, as Fleet records it from the hooks that observe the tools: one entry for a burst, dated by its first call, with
 * the files or commands behind it in `items`, newest first. A path may be absolute under the bay, or from the home directory with
 * `~`, or already relative. **A drone's doing is read from these and never from what an agent says about itself.**
 */
const burst = (minutes: number, who: Who, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, items: string[], repo?: string): TimelineEvent =>
  event(minutes, who, kind, tone, says, { detail: items[0]!, items, live: true, ...(repo === undefined ? {} : { repo }) });

const TOOLS: TimelineEvent[] = [
  // At rest, within the last two minutes: Migration notes reads, Cache the manifest runs. The rest are older, so they say what was done.
  burst(1, sessionWho(working), "session.read", "run", "Read 3 files", ["crates/store/migrations/0044.sql", "crates/store/migrations/0043.sql", "crates/store/migrations/0042.sql"]),
  burst(1, jobWho(cache), "job.ran", "run", "Ran 1 command", ["cargo test -p fleet manifest"]),
  burst(12, sessionWho(WORKING_REPLY), "session.edited", "run", "Edited 2 files", [`${slotPath(ARMADA_ROOT, 4)}/crates/store/tests/clock.rs`, `${slotPath(ARMADA_ROOT, 4)}/crates/store/tests/pool.rs`]),
  burst(6, sessionWho(WORKING_REPLY), "session.ran", "run", "Ran 2 commands", ["cargo test -p store", "cargo clippy -p store"]),
  burst(5, sessionWho(queuedOwner), "session.edited", "run", "Edited 3 files", ["~/armada/.armada/slots/slot-1/crates/fleet/src/gate.rs", "~/armada/.armada/slots/slot-1/crates/gate/src/run.rs", "~/armada/.armada/slots/slot-1/crates/gate/src/policy.rs"]),
  event(7, jobWho(cache), "job.files", "run", "Changed 2 files", { detail: "crates/fleet/src/manifest.rs", live: true }),
  event(6, jobWho(reconnect), "job.files", "run", "Changed 1 file", { detail: "apps/desktop/src/renderer/src/cockpit/reconnect.ts", live: true }),
  burst(30, sessionWho(red), "session.edited", "run", "Edited 1 file", ["apps/desktop/src/renderer/src/cockpit/reconnect.ts"]),
  burst(50, sessionWho(stuck), "session.edited", "run", "Edited 1 file", ["/Users/user/pocket/.armada/slots/slot-1/src/sw/cache.ts"], POCKET),
];

/** Who held a bay, and when, as Fleet notes it: `slot.leased` when a holder takes the slot, `slot.released` when it gives it back. */
const lease = (minutes: number, who: Who, slot: number, kind: "slot.leased" | "slot.released" = "slot.leased", repo?: string): TimelineEvent =>
  event(minutes, who, kind, "idle", kind === "slot.leased" ? `Slot ${slot} leased` : `Slot ${slot} given back`, { detail: `slot-${slot}`, ...(repo === undefined ? {} : { repo }) });
const gone = (kind: "job" | "session", id: string, title: string): Who => ({ subject: { kind, id }, title });

const SLOTS: TimelineEvent[] = [
  // Bay 1: an earlier Session, given back, then the one that holds it.
  lease(230, gone("session", "01SESSIONRELEASENOTES0001", "Release notes"), 1),
  lease(205, gone("session", "01SESSIONRELEASENOTES0001", "Release notes"), 1, "slot.released"),
  lease(80, sessionWho(queuedOwner), 1),
  // Bay 2: a Job given back, then the one that holds it.
  lease(150, gone("job", "01JOBBUMPLOCKFILE000001", "Bump the lockfile"), 2),
  lease(120, gone("job", "01JOBBUMPLOCKFILE000001", "Bump the lockfile"), 2, "slot.released"),
  lease(17, jobWho(cache), 2),
  lease(33, jobWho(reconnect), 3),
  lease(16, sessionWho(WORKING_REPLY), 4),
  // Bay 5 is free and bay 2 of pocket is closed, and no event names either: they have no history.
  // Bay 6: its holder took it and was never heard to give it back; it stands stranded.
  lease(180, gone("session", "01SESSIONCLOSEDFRAME0000", "Frame the reply"), 6),
  lease(40, sessionWho(stuck), 1, "slot.leased", POCKET),
];

const EVENTS: TimelineEvent[] = [...BASE_EVENTS, ...TOOLS, ...SLOTS];

/** The write clashes Fleet reports, as `GET /clashes` returns them: a Job and a Session in one file, and a Session against a main that moved. */
const CLASHES: WriteClash[] = [
  {
    one: { kind: "job", id: reconnect.job.id, label: reconnect.job.title },
    other: { kind: "session", id: red.id, label: red.title ?? red.id },
    paths: ["apps/desktop/src/renderer/src/cockpit/reconnect.ts"],
    since: ago(9),
  },
  {
    one: { kind: "session", id: WORKING_REPLY.id, label: WORKING_REPLY.title ?? WORKING_REPLY.id },
    other: { kind: "main", id: "main", label: "main" },
    paths: ["crates/store/tests/clock.rs"],
    since: ago(3),
  },
];

/** The plan's windows the title bar's meters read. */
const PLAN = (): PlanUsage => ({
  windows: [
    { kind: "five_hour", percent_used: 34, resets_at: new Date(Date.now() + (2 * 60 + 14) * 60_000).toISOString() },
    { kind: "seven_day", percent_used: 61, resets_at: new Date(Date.now() + (2 * 24 + 17) * 3_600_000).toISOString() },
  ],
  read_at: new Date().toISOString(),
});

function build(): Scenario {
  const pocketManifest = { ...repository().manifest!, id: POCKET, repository: POCKET, path: POCKET_ROOT, records_root: `${POCKET_ROOT}/.armada` };
  const base = holding("cockpit-ship-flightdeck", "The whole ship at rest: two repositories, four Jobs, seven Sessions, four pull requests, main green, eight bays, Autopilot off", JOBS, {
    alsoServed: [{ root: POCKET_ROOT, records_root: `${POCKET_ROOT}/.armada`, manifest: pocketManifest }],
  });
  const jobs = JOBS.map((one) => one.job);
  return {
    ...base,
    state: { ...base.state, repository: null, jobs, mergeLines: lines("rest") },
    held: pool(),
    // The Session replies, main goes red as 1890 lands, the fix lands. The Job dispatched at the second moment lifts from its bay.
    later: [{}, { mergeLines: lines("red"), jobs: [...jobs, dispatched.job] }, { mergeLines: lines("fixed"), jobs: [...jobs, dispatched.job] }],
    draft: { calls: CALLS, now: NOWS, sessions: (control) => sessions(control, SESSIONS), events: EVENTS, clashes: CLASHES, areas: mockAreas },
    behaves: () => ({ getPlanUsage: async () => ({ ok: true as const, value: PLAN() }), onPlanUsageChanged: () => () => undefined }),
  };
}

export const s252CockpitShipFlightdeck: Scenario = build();
