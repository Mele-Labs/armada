// The whole ship at rest, in the wire shapes Fleet serves, for the flight deck: two repositories, seven Sessions in
// the states Fleet reads them to be in, four pull requests each owned by a Session or nobody, main green, eight bays
// (five held, one free, one stranded, one closed), Autopilot off.
//
// **Everything the deck reads is something Fleet records.** The tools a drone has used are `session.read`, `session.edited`,
// `session.ran` bursts with the `items` they touched, some spelled under the bay's checkout and some
// from the home directory. The bays' holders are `slot.leased` and `slot.released` events with `detail "slot-N"`. The write
// clashes are the `GET /clashes` list. A bay with no slot event (the free one, the closed one) has no history, and a drone
// that edited nothing is observed to touch nothing. Two `later` moments carry the story: the Session that was working
// replies, main goes red once PR 1890 lands, and the fix lands. #2301.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { HubMerged, HubPullRequest, MainStanding, MergeLines, PlanUsage, TimelineEvent, TimelineSubject, WorktreeSlot, WorktreesHeld, WriteClash } from "@armada/protocol";
import type { Session, SessionAttachment, SessionChecks } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore, type SessionsStore } from "../sessions/script";

const ARMADA = repository().manifest!.id;
const POCKET = "pocket";
const POCKET_ROOT = "/Users/user/pocket";
const FORGE = "https://git.example/armada/pull/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

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
  lastTurnAt: ago(1),
  standing: "working",
  rows: [
    ask("w1-u", "14:10:00", "Summarise the migrations since 0042."),
    tool("w1-t1", "14:15:41", "Read crates/store/migrations/0042.sql"),
    tool("w1-t2", "14:16:20", "Read crates/store/migrations/0043.sql"),
    say("w1-a", "14:16:58", "0043 adds the pause marker column and backfills it to null."),
    tool("w1-t3", "14:17:02", "Grep crates/store -e pause_marker"),
  ],
  attachments: [pr(1892, "Add the pause marker column", "fleet/pause-marker-column", { state: "pending" })],
  context: { tokens: 612_000, window: 1_000_000 },
  contextBreakdown: {
    categories: [
      { name: "System prompt", tokens: 3_100, kind: "used" },
      { name: "System tools", tokens: 12_400, kind: "used" },
      { name: "MCP tools", tokens: 8_200, kind: "used" },
      { name: "Memory files", tokens: 4_600, kind: "used" },
      { name: "Messages", tokens: 583_700, kind: "used" },
      { name: "Free space", tokens: 238_000, kind: "free" },
      { name: "Autocompact buffer", tokens: 150_000, kind: "buffer" },
    ],
    memoryFiles: [
      { path: "NOTES.md", tokens: 2_100 },
      { path: "apps/NOTES.md", tokens: 700 },
    ],
    mcpTools: [
      { name: "list_jobs", server: "armada-fleet", tokens: 2_600 },
      { name: "query", server: "gitnexus", tokens: 1_500 },
    ],
    totalTokens: 612_000,
    maxTokens: 1_000_000,
    autocompactThreshold: 850_000,
  },
};

/** Its turn is running and has said nothing for 25 minutes. In the pocket repository. */
const stuck: Session = {
  id: "k1",
  title: "Service worker audit",
  manifestId: POCKET,
  turn: { state: "working" },
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
  rows: [
    ...WORKING_REPLY.rows,
    say("r1-a", "14:18:09", "The clock is pinned in all four store tests and the fixture is gone.\nTwo files changed, and the suite passes."),
  ],
};

/** The Sessions, with the moment that moves them, and Stop, which Fleet answers and the mock's fixtures do not. */
function sessions(list: readonly Session[] = SESSIONS): SessionsStore {
  const store = sessionsStore(list);
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
        open(1892, "Add the pause marker column", "fleet/pause-marker-column", "running", { opened_at: ago(30) }),
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

/** Eight bays: held by five Sessions, one free, one stranded, one closed. */
function pool(): WorktreesHeld {
  const session = (manifest: string, slot: number, one: Session, branch: string, minutes: number) =>
    bay(manifest, slot, { state: "hosted_session", session_id: one.id, session_title: one.title ?? one.id }, { branch, since: ago(minutes) });
  return {
    slots: [
      session(ARMADA, 1, queuedOwner, "fleet/gate-policy-every-run", 80),
      session(ARMADA, 2, asking, "fix/cache-key", 17),
      session(ARMADA, 3, red, "fleet/reconnect-wait", 33),
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



// ---- What Fleet did, for the flight deck's timeline ---------------------------------------------------

let count = 0;
type Who = { subject: TimelineSubject; title: string };
const sessionWho = (one: Session): Who => ({ subject: { kind: "session", id: one.id }, title: one.title ?? one.id });
const pullWho = (number: number, title: string): Who => ({ subject: { kind: "pull_request", number, url: `${FORGE}${number}` }, title });
function event(minutes: number, who: Who, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, rest: { detail?: string; items?: string[]; live?: boolean; over?: boolean; repo?: string } = {}): TimelineEvent {
  count += 1;
  const { repo, ...more } = rest;
  return { id: count, at: ago(minutes), kind, subject: who.subject, title: who.title, tone, says, manifest_id: repo ?? ARMADA, ...more };
}

const BASE_EVENTS: TimelineEvent[] = [
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
  burst(12, sessionWho(WORKING_REPLY), "session.edited", "run", "Edited 2 files", [`${slotPath(ARMADA_ROOT, 4)}/crates/store/tests/clock.rs`, `${slotPath(ARMADA_ROOT, 4)}/crates/store/tests/pool.rs`]),
  burst(6, sessionWho(WORKING_REPLY), "session.ran", "run", "Ran 2 commands", ["cargo test -p store", "cargo clippy -p store"]),
  burst(5, sessionWho(queuedOwner), "session.edited", "run", "Edited 3 files", ["~/armada/.armada/slots/slot-1/crates/fleet/src/gate.rs", "~/armada/.armada/slots/slot-1/crates/gate/src/run.rs", "~/armada/.armada/slots/slot-1/crates/gate/src/policy.rs"]),
  burst(30, sessionWho(red), "session.edited", "run", "Edited 2 files", ["apps/desktop/src/renderer/src/cockpit/reconnect.ts", "crates/fleet/src/reconnect.rs"]),
  burst(50, sessionWho(stuck), "session.edited", "run", "Edited 1 file", ["/Users/user/pocket/.armada/slots/slot-1/src/sw/cache.ts"], POCKET),
];

/** Who held a bay, and when, as Fleet notes it: `slot.leased` when a holder takes the slot, `slot.released` when it gives it back. */
const lease = (minutes: number, who: Who, slot: number, kind: "slot.leased" | "slot.released" = "slot.leased", repo?: string): TimelineEvent =>
  event(minutes, who, kind, "idle", kind === "slot.leased" ? `Slot ${slot} leased` : `Slot ${slot} given back`, { detail: `slot-${slot}`, ...(repo === undefined ? {} : { repo }) });
const gone = (id: string, title: string): Who => ({ subject: { kind: "session", id }, title });

const SLOTS: TimelineEvent[] = [
  // Bay 1: an earlier Session, given back, then the one that holds it.
  lease(230, gone("01SESSIONRELEASENOTES0001", "Release notes"), 1),
  lease(205, gone("01SESSIONRELEASENOTES0001", "Release notes"), 1, "slot.released"),
  lease(80, sessionWho(queuedOwner), 1),
  lease(33, sessionWho(red), 3),
  lease(17, sessionWho(asking), 2),
  lease(16, sessionWho(WORKING_REPLY), 4),
  // Bay 5 is free and bay 2 of pocket is closed, and no event names either: they have no history.
  // Bay 6: its holder took it and was never heard to give it back; it stands stranded.
  lease(180, gone("01SESSIONCLOSEDFRAME0000", "Frame the reply"), 6),
  lease(40, sessionWho(stuck), 1, "slot.leased", POCKET),
];

const EVENTS: TimelineEvent[] = [...BASE_EVENTS, ...TOOLS, ...SLOTS];

/** The write clashes Fleet reports, as `GET /clashes` returns them: two Sessions in one file, and a Session against a main that moved. */
const CLASHES: WriteClash[] = [
  {
    one: { kind: "session", id: asking.id, label: asking.title ?? asking.id },
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
  const base = holding("cockpit-ship-flightdeck", "The whole ship at rest: two repositories, seven Sessions, four pull requests, main green, eight bays, Autopilot off", [
    repository(),
    { root: POCKET_ROOT, records_root: `${POCKET_ROOT}/.armada`, manifest: pocketManifest },
  ]);
  return {
    ...base,
    state: { ...base.state, repository: null, mergeLines: lines("rest") },
    held: pool(),
    // The Session replies, main goes red as 1890 lands, the fix lands.
    later: [{}, { mergeLines: lines("red") }, { mergeLines: lines("fixed") }],
    draft: { sessions: (_control, more = []) => sessions([...SESSIONS, ...more]), events: EVENTS, clashes: CLASHES, areas: mockAreas },
    behaves: () => ({ getPlanUsage: async () => ({ ok: true as const, value: PLAN() }), onPlanUsageChanged: () => () => undefined }),
  };
}

export const s252CockpitShipFlightdeck: Scenario = build();
