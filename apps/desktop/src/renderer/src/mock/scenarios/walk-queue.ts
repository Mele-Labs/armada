// One walk window for every walk. Five walks are shown when the Cockpit opens, oldest first: a Job's on screen with three takes of
// its page, a walk playing in it and a question waiting, then another Job's, a pull request's review, a third Job's, and the second
// Job's second walk, which asked. The panel beside the page is the Cockpit's own Job view for the walk on screen. Draft data, nothing
// here reaches Fleet. The walk `walkQueue` plays it.

import type { Variation } from "@armada/screens/src/fixtures/walk-window";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";
import { closeWalkWindow, jumpToWalk, mockPage, queueWalk } from "@armada/screens/src/fixtures/walk-window";
import { activityOf } from "@armada/screens/src/sessions-wire";

import { askToOpenSession } from "../../open-session";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { queueReviewWalk } from "../pr-review-walk-window";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

/** The Jobs the walks belong to. */
export const BACKOFF = "wq-backoff";
export const CLOCK = "wq-clock";
export const FOLD = "wq-fold";
export const DRAFT = "wq-draft";

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

/** The mock itself on another scenario: a page each walk can be told from. */
const page = (scenario: string): string => {
  const url = new URL(mockPage());
  url.search = `?scenario=${scenario}&walked`;
  return url.toString();
};

/** The three takes of the first walk's page: a short walk each, the mock playing itself in the frame. */
const takes = (): Variation[] =>
  [
    { name: "Artifacts", walk: "ledger-in-cockpit" },
    { name: "Task marks", walk: "taskMarkTooltips" },
    { name: "Sleep cockpit", walk: "sleepCockpit" },
  ].map(({ name, walk }) => {
    const url = new URL(mockPage());
    url.search = `?walk=${walk}&walked`;
    return { name, url: url.toString() };
  });

const backoff: Session = {
  id: BACKOFF,
  title: "Retry backoff cap",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurnAt: ago(6),
  standing: "idle",
  rows: [
    you("b-1", "13:50:00", "Cap the retry backoff at thirty seconds."),
    { id: "b-2", at: "13:58:20", kind: "tool", text: "Edit crates/fleet/src/retry.rs" },
    agent("b-3", "14:02:10", "The cap is in. The backoff table is in a window."),
  ],
  attachments: [{ kind: "branch", name: "fleet/retry-backoff-cap", slot: 2 }, { kind: "artifact", form: "window", id: page("default"), title: "Backoff table" }],
};

const clock: Session = {
  id: CLOCK,
  title: "Pin the store clock",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurnAt: ago(4),
  standing: "idle",
  rows: [
    you("c-1", "13:40:00", "Pin the clock in the store tests."),
    { id: "c-2", at: "13:55:41", kind: "tool", text: "Edit crates/store/tests/store_test.rs" },
    agent("c-3", "14:04:30", "Every test reads the pinned clock. The report is in a window."),
  ],
  attachments: [{ kind: "branch", name: "fix/pin-store-clock", slot: 4 }, { kind: "artifact", form: "window", id: page("sessions"), title: "Clock report" }],
};

const fold: Session = {
  id: FOLD,
  title: "Fold Jobs into Sessions",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "working" },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [
    you("f-1", "13:30:00", "Fold Jobs into Sessions. A Session plans, he approves, and its work is the Job."),
    agent("f-2", "13:44:10", "The plan is recorded: four tasks in two groups. Approve it, or tell me what to change."),
    you("f-3", "13:50:40", "Say in the first group that the queue survives a restart."),
    { id: "f-4", at: "14:05:02", kind: "tool", text: "Edit docs/concepts/session.md" },
  ],
  attachments: [{ kind: "branch", name: "fleet/fold-jobs", slot: 5 }, { kind: "artifact", form: "window", id: page("dashboard-needs-you"), title: "Plan sketch" }],
  activity: activityOf({
    now: "Editing session.md",
    todos: [
      { text: "Keep the queue across a restart", state: "done" },
      { text: "Rewrite the Session concept", state: "doing" },
      { text: "Move the Job rows onto Sessions", state: "todo" },
    ],
  }),
  work: {
    plan: {
      version: 2,
      scope: ["crates/fleet", "docs/concepts/session.md"],
      approved: false,
      after: "f-3",
      groups: [
        { id: "p1", agent: "rust-engineer", title: "Keep the queue", paths: ["crates/fleet"], tasks: [{ id: "T1", text: "Write the queue to the store", state: "done", seen: true }, { id: "T2", text: "Read it back when Fleet starts", state: "working", seen: true }] },
        { id: "p2", agent: "docs-writer", title: "Say it", paths: ["docs/concepts/session.md"], tasks: [{ id: "T3", text: "Rewrite the Session concept", state: "open" }, { id: "T4", text: "Move the Job rows onto Sessions", state: "open" }] },
      ],
    },
  },
};

const draft: Session = {
  id: DRAFT,
  title: "Take a draft out",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurnAt: ago(9),
  standing: "idle",
  rows: [
    you("d-1", "13:20:00", "Let him take a draft out of the queue."),
    agent("d-2", "13:58:00", "The change is up as #2412."),
  ],
  attachments: [
    { kind: "branch", name: "bridge/take-draft-out", slot: 6 },
    { kind: "pull_request", number: 2412, title: "Take a draft out", branch: "bridge/take-draft-out", address: "https://example.com/pull/2412", checks: { state: "passed" }, state: "open", auto: false },
  ],
};

function sessions() {
  const store = sessionsStore([backoff, clock, fold, draft], [], new Map([[ROOT, OWNER]]));
  const told = (id: string, text: string) => store.send(id, { text, files: [], sketches: [], tags: [] });
  /** A Job's walk: its id, and what approving it tells the Job. */
  const jobWalk = (job: Session, n: number, address: string, title: string, since: string, more: { variations?: Variation[]; asking?: string } = {}) => {
    const id = `${job.id}:${n}`;
    queueWalk(id, title, address, {
      title,
      job: job.title ?? job.id,
      into: "Session",
      entry: { name: job.title ?? job.id },
      since,
      panelFor: job.id,
      onOpenOwner: () => askToOpenSession(job.id),
      ...(more.variations === undefined ? {} : { variations: more.variations }),
      ...(more.asking === undefined ? {} : { asking: more.asking }),
      onAnswer: (said) => void told(job.id, said === "" ? "No answer" : said),
      onNote: (note, picked) => told(job.id, `${note}\nOn ${picked.element}, ${picked.location}`),
      onApprove: async (where) => {
        await told(job.id, `Approved: ${where}`);
        return { ok: true };
      },
      readiness: "ready",
    });
  };
  /** The walks shown before the owner looks, oldest first. A window is only there for the app mounted beside it. */
  if (typeof document !== "undefined") {
    closeWalkWindow();
    const first = takes();
    jobWalk(backoff, 1, first[0]!.url, "Backoff table", ago(30), { variations: first, asking: "Should the cap apply to the first retry too?" });
    jobWalk(clock, 1, page("sessions"), "Clock report", ago(25));
    queueReviewWalk(
      { walkId: "w-2412", manifestId: OWNER, number: 2412, title: "#2412 Take a draft out", address: "http://localhost:41311", steps: [{ say: "The bays", look: { role: "region", name: "Bays" } }], holder: { id: DRAFT, title: "Take a draft out" } },
      undefined,
      {
        since: ago(20),
        onNote: () => {},
        onClose: () => {},
        onApprove: async () => {
          await told(DRAFT, "Approved: #2412 Take a draft out");
          return { ok: true };
        },
      },
    );
    jobWalk(fold, 1, page("dashboard-needs-you"), "Plan sketch", ago(15));
    jobWalk(clock, 2, page("pages-look"), "Store tests", ago(5), { asking: "Keep the old fixture or drop it?" });
  }
  return {
    ...store,
    // A walk pressed open on the Job's own page is the walk already in the window.
    openWindow: (id: string, url: string) => (url === page("sessions") ? jumpToWalk(`${id}:1`) : undefined),
  };
}

export const s297WalkQueue: Scenario = {
  ...holding("walk-queue", "One walk window for every walk: Jobs' and a pull request's, queued oldest first, with the Job's own view beside the page"),
  draft: { sessions },
};
