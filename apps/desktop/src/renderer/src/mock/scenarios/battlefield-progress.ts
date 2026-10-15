// One repository's battlefield with a Session building something on it, for the walk `battlefieldProgress`: its drone, two
// ordinary drones beside it, and a twelve-hour story the walk steps through (`battlefield-story.ts`). The Session's turn runs
// until its pull request opens, then ends; the forge shows the pull request open, queued and merged. The three takes and the
// three things a landed structure may do are the URL's: `?build=tower|pad|ring`, `?after=stand|clear|monument`. #2301.

import { repository } from "@armada/screens/src/fixtures/build/base";
import type { HubMerged, HubPullRequest, MainStanding, MergeLines, TimelineEvent, TimelineSubject, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { workingAPlan } from "@armada/jobs/fixtures/build/index";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { mockAreas } from "../cockpit-areas";
import { FEATURED, jobAt, storyAt } from "../battlefield-story";
import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { mockProgress } from "../progress";
import { sessionsStore, type SessionsStore } from "../sessions/script";

const ARMADA = repository().manifest!.id;
const FORGE = "https://git.example/armada/pull/";
const BRANCH = "armada/dock-panel";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const PR = 2310;

/** The Job's plan: two tasks done, one handed in, one being worked, one open. Its page draws them as the structure on its route. */
const PLAN = (() => {
  const task = (id: string, title: string, state: "done" | "handed_in" | "working" | "open") => ({ id, title, state });
  return [task("T1", "Read the manifest once", "done"), task("T2", "Key the cache by digest", "done"), task("T3", "Drop the cache on edit", "handed_in"), task("T4", "Warm it at start", "working"), task("T5", "Test the cold read", "open")];
})();

const cache: JobFixture = (() => {
  const base = asRow(workingAPlan(), 91, "cache", "Cache the manifest read");
  const count = (state: (typeof PLAN)[number]["state"]) => PLAN.filter((one) => one.state === state).length;
  const job = { ...base.job, owner_manifest_id: ARMADA, started_at: ago(17), current_step_id: "fix", branch: "armada/91-cache", tasks: { done: count("done"), handed_in: count("handed_in"), working: count("working"), open: count("open"), dropped: 0 } };
  if (base.watched.state !== "read" || base.watched.detail.work_plan === undefined) return { ...base, job };
  const detail = { ...base.watched.detail, job: { ...base.watched.detail.job, ...job }, work_plan: { ...base.watched.detail.work_plan, tasks: PLAN } };
  return { ...base, job, watched: { ...base.watched, detail } };
})();

const tool = (id: string, at: string, text: string) => ({ id, at, kind: "tool", text }) as const;
const ask = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "you" }, text }) as const;

const dock: Session = {
  id: FEATURED,
  title: "Rebuild the dock panel",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurn: "14:11",
  lastTurnAt: ago(1),
  standing: "working",
  rows: [ask("b1-u", "06:00:00", "Rebuild the dock panel, with the keys it has today."), tool("b1-t", "14:11:10", "Edit apps/desktop/src/renderer/src/cockpit/dock/keys.ts")],
  attachments: [{ kind: "branch", name: BRANCH, slot: 1 }],
};

const clock: Session = {
  id: "bp2",
  title: "Pin the store clock",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurn: "14:09",
  lastTurnAt: ago(2),
  standing: "working",
  rows: [ask("b2-u", "13:20:00", "Pin the clock in the store tests."), tool("b2-t", "14:09:10", "Edit crates/store/tests/clock.rs")],
  attachments: [{ kind: "branch", name: "fix/pin-store-clock", slot: 2 }],
};

/** The turn ends when the pull request opens: the Session waits on its Checks. */
const SETTLED: Partial<Session> = { turn: { state: "idle" }, standing: "idle", lastTurn: "14:52", lastTurnAt: new Date().toISOString() };

function sessions(control: Parameters<typeof sessionsStore>[3]): SessionsStore {
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  const store = sessionsStore([none, none], none, [], control, [], [dock, clock]);
  const own = new Set([dock.id, clock.id]);
  let moment = 0;
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  return {
    ...store,
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
      mockProgress.next();
      if (moment === 5) store.edit(FEATURED, (one) => ({ ...one, ...SETTLED }));
    },
  };
}

const COMMIT = "9f3c1d2e4b5a69788796a5b4c3d2e1f001122334";
const GREEN: MainStanding = { state: "green", commit: COMMIT, read_at: new Date().toISOString() };

const open = (ci: string, more: Partial<HubPullRequest> = {}): HubPullRequest => ({ number: PR, title: dock.title!, branch: BRANCH, url: `${FORGE}${PR}`, ci, opened_at: ago(20), ...more });
const merged = (): HubMerged => ({ number: PR, title: dock.title!, branch: BRANCH, url: `${FORGE}${PR}`, merged_at: ago(1), main_run: { state: "passed" } });

/** The forge at a moment: nothing yet, the pull request open, queued, merged. */
function lines(moment: number): MergeLines {
  const pull_requests = moment < 5 ? [] : moment === 5 ? [open("running")] : moment === 6 ? [open("running", { queue: { state: "queued", position: 1 } })] : [];
  return { lines: [{ root: repository().root, line: [], off: [], landed: [], sent_back: [], hub: { main: GREEN, pull_requests, merged: moment >= 7 ? [merged()] : [] } }] };
}

const bay = (slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({ manifest_id: ARMADA, slot, path: `/Users/user/armada/.armada/slots/slot-${slot}`, base: "main", warm: true, behind: 0, held, ...rest });

function pool(): WorktreesHeld {
  return {
    worktrees: [],
    slots: [
      bay(1, { state: "hosted_session", session_id: dock.id, session_title: dock.title! }, { branch: BRANCH, since: ago(720) }),
      bay(2, { state: "hosted_session", session_id: clock.id, session_title: clock.title! }, { branch: "fix/pin-store-clock", since: ago(40) }),
      bay(3, { state: "job", job_id: cache.job.id, job_title: cache.job.title }, { branch: "armada/91-cache", since: ago(17) }),
      bay(4, { state: "free" }, { warm: false }),
      bay(5, { state: "free" }),
    ],
  };
}

let count = 0;
type Who = { subject: TimelineSubject; title: string };
const sessionWho = (one: Session): Who => ({ subject: { kind: "session", id: one.id }, title: one.title ?? one.id });
function event(minutes: number, who: Who, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, rest: { detail?: string; items?: string[]; live?: boolean } = {}): TimelineEvent {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: who.subject, title: who.title, tone, says, manifest_id: ARMADA, ...rest };
}
const edited = (minutes: number, who: Who, items: string[]) => event(minutes, who, "session.edited", "run", `Edited ${items.length} files`, { detail: items[0]!, items, live: true });
const lease = (minutes: number, who: Who, slot: number) => event(minutes, who, "slot.leased", "idle", `Slot ${slot} leased`, { detail: `slot-${slot}` });

/** What places the drones: the directories each has edited, as the checkout spells them, and who holds each bay. The story's own record is the construction. */
function placed(): TimelineEvent[] {
  const job: Who = { subject: { kind: "job", id: cache.job.id }, title: cache.job.title };
  return [
    event(720, sessionWho(dock), "session.started", "run", "Started", { live: true }),
    edited(60, sessionWho(dock), ["apps/desktop/src/renderer/src/cockpit/dock/Page.tsx", "apps/desktop/src/renderer/src/cockpit/dock/keys.ts"]),
    event(40, sessionWho(clock), "session.started", "run", "Started", { live: true }),
    edited(20, sessionWho(clock), ["crates/store/tests/clock.rs", "crates/store/tests/pool.rs"]),
    event(17, job, "job.started", "run", "Started", { live: true }),
    event(10, job, "job.files", "run", "Changed 1 file", { detail: "packages/components/src/Tabs.tsx", live: true }),
    lease(720, sessionWho(dock), 1),
    lease(40, sessionWho(clock), 2),
    lease(17, job, 3),
  ];
}

function build(): Scenario {
  const base = holding("battlefield-progress", "One repository's battlefield with a Session building a dock panel over twelve hours, as a tower, a gantry or a ring", [cache]);
  const jobs = [cache.job];
  return {
    ...base,
    state: { ...base.state, repository: null, jobs, mergeLines: lines(0) },
    held: pool(),
    // Moments 1 to 8: the Session's turn ends and its pull request opens at the fifth, is queued at the sixth and merges at the seventh.
    later: Array.from({ length: 8 }, (_, i) => ({ mergeLines: lines(i + 1) })),
    draft: { sessions, events: placed(), areas: mockAreas, progress: (moment) => [...storyAt(moment), jobAt(moment, cache.job.id)] },
  };
}

export const s253BattlefieldProgress: Scenario = build();
