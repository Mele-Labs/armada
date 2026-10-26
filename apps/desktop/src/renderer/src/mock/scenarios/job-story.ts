// A Job's right column as a story. Three Jobs on the Cockpit, each with a plan it has just recorded: one that lands in a pull request, one that
// lands in a spike's findings doc and one that only investigates and lands in a report. Approving a plan starts the build (the Field opens), and
// each time the walk lets time pass the build comes closer to its landing: for the pull request, Checks passed, the queue, an issue filed, a watch
// and a follow-up; for the spike, the findings doc; for the report, the report. A fourth Job holds another pull request so the Merge line has width.
// The repository serves a merge line. Groups g1 and g2 work issues LEASE-20481 (a key from another tracker, longer than a forge number) and 2367; the issues no group works (2374, and 2402 once filed) stand in Deferred. Mock only: the stage is read in Bridge from the Job's data
// (`session-view/story.ts`), and `work.landing` names the kind. The walks `jobStory`, `jobStorySpike` and `jobStoryReport` play it.

import type { MergeLine, WatchSummary } from "@armada/protocol";
import type { Session, SessionAttachment, SessionRow, SessionWork, WorkGroup, WorkTaskState, WorkflowChoice } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { mockAreas } from "../cockpit-areas";
import { LEASE_REPORT, LEASE_REPORT_PATH } from "../files-disk";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { planCall, withoutPlanCall } from "../plan-call";
import { arrow, box, scene } from "../scenes";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const PR_JOB = "story-1";
const SPIKE_JOB = "story-2";
const REPORT_JOB = "story-3";
const OTHER_JOB = "story-4";
const BRANCH = "armada/lease-retry";

const FEATURE: WorkflowChoice = { id: "Feature", name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"] };
const SPIKE: WorkflowChoice = { id: "Spike", name: "Spike", steps: ["Plan", "Measure", "Write up"] };
const INVESTIGATE: WorkflowChoice = { id: "Investigate", name: "Investigate", steps: ["Plan", "Investigate", "Report"] };
const WORKFLOWS = [FEATURE, SPIKE, INVESTIGATE];

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

const PLAN_SKETCH_ID = "sketch:Plan";
const PLAN_SKETCH = scene(
  [
    box("fleet", 0, 0, "Fleet", "Retry the lease with a back off", { icon: "server", step: 1 }),
    box("bridge", 360, 0, "Bridge", "The bay tile shows the attempt", { icon: "globe", step: 2 }),
  ],
  [arrow("fleet", "bridge", "attempts", undefined, 2)],
);

const task = (id: string, text: string, state: WorkTaskState) => ({ id, text, state, seen: state !== "open" });

// ---- The pull request Job ------------------------------------------------------------------------------------

/** The plan's groups at a stage: 0 recorded, 1 under way, 2 built. */
function prGroups(stage: number): WorkGroup[] {
  const at = (open: WorkTaskState, working: WorkTaskState, built: WorkTaskState) => (stage === 0 ? open : stage === 1 ? working : built);
  return [
    { id: "g1", agent: "rust-engineer", title: "Retry the lease when the pool is contended", paths: ["crates/fleet"], issue: "LEASE-20481", tasks: [task("T1", "Retry the lease when the pool is contended", at("open", "done", "done")), task("T2", "Back off between attempts", at("open", "working", "done"))] },
    { id: "g2", agent: "bridge-engineer", title: "Show the attempt on the bay tile", paths: ["apps/desktop"], issue: "2367", tasks: [task("T3", "Show the attempt on the bay tile", at("open", "working", "done")), task("T4", "Name the attempt in the tooltip", at("open", "open", "done"))] },
  ];
}

const PR_WORK: SessionWork = {
  proposal: { workflow: FEATURE.name },
  landing: "pull_request",
};

const WATCH: WatchSummary = {
  id: "w-ci",
  session_id: PR_JOB,
  name: "ci on #2390",
  command: ["scripts/watch/pull-request", "2390"],
  every_seconds: 60,
  gives_up_after_seconds: 3600,
  follows: 2390,
  state: "running",
  live: true,
  started_at: ago(4),
  readings: [{ kind: "read", at: ago(1), state: "running", finished: false, says: "ci is in the merge queue" }],
  last: { kind: "read", at: ago(1), state: "running", finished: false, says: "ci is in the merge queue" },
};

/** The issues the Job was started on, more than a band could hold. */
const issue = (number: string, title: string, state: "open" | "closed"): SessionAttachment => ({ kind: "issue", number, title, address: `https://forge.invalid/issues/${number}`, state });
const ISSUES: SessionAttachment[] = [
  { ...issue("LEASE-20481", "Two Jobs can lease one bay at once when the pool is contended", "open"), pull_requests: [2390] } as SessionAttachment,
  issue("2367", "The bay tile does not show a retry", "open"),
  issue("2374", "The lease gives up without saying why", "closed"),
];

/** What the pull request Job holds at a stage. */
function prAt(stage: number): Partial<Session> {
  const approved = stage > 0;
  const plan = {
    scope: ["crates/fleet", "apps/desktop"],
    approved,
    after: "s1-2",
    sketch: { id: PLAN_SKETCH_ID, title: "Plan", scene: PLAN_SKETCH },
    summary: "The lease retries with a back off when two Jobs ask for one bay, and the bay tile shows which attempt it is on.",
    groups: prGroups(stage),
  };
  const pr = (extra: Pick<Extract<SessionAttachment, { kind: "pull_request" }>, "checks" | "state" | "auto"> & { queued?: true; number?: number; title?: string; closes?: number[] }): SessionAttachment => ({ kind: "pull_request", number: 2390, title: "Retry the slot lease", branch: BRANCH, address: `https://git.example/armada/pull/${extra.number ?? 2390}`, closes: [2367], ...extra });
  return {
    turn: { state: stage === 1 ? "working" : "idle" },
    standing: stage === 1 ? "working" : "idle",
    attachments: [
      ...ISSUES,
      ...(stage === 0 ? [] : [{ kind: "branch" as const, name: BRANCH, slot: 3 }]),
      ...(stage === 1 ? [pr({ checks: { state: "pending" }, state: "draft", auto: false })] : []),
      ...(stage === 2
        ? [
            pr({ checks: { state: "passed" }, state: "open", auto: true, queued: true }),
            pr({ number: 2391, title: "Name the attempt in the bay tooltip", closes: [], checks: { state: "pending" }, state: "open", auto: false }),
            { kind: "issue" as const, number: "2402", title: "A slow attempt can outlive the lease", address: "https://forge.invalid/issues/2402", state: "open" as const },
          ]
        : []),
      { kind: "sketch", id: PLAN_SKETCH_ID, title: "Plan", by: "agent", drawing: PLAN_SKETCH },
    ],
    waitingFor: stage === 0 ? [planCall(undefined, ago(1))] : [],
    ...(stage === 2 ? { watches: [WATCH], later: [{ id: "later-1", text: "Name the attempt in the bay tooltip's second line.", at: ago(1) }] } : {}),
    work: {
      ...PR_WORK,
      workflow: { ...FEATURE, at: stage === 0 ? 0 : stage === 1 ? 1 : 3 },
      plan,
      ...(stage === 1
        ? {
            field: [
              { id: "main", path: "crates/fleet", doing: "read" as const, file: "crates/fleet/src/pool.rs" },
              { id: "rust", agent: "rust-engineer", task: "Back off between attempts", path: "crates/fleet", doing: "write" as const, file: "crates/fleet/src/backoff.rs" },
              { id: "bridge", agent: "bridge-engineer", task: "Show the attempt on the bay tile", path: "apps/desktop", doing: "write" as const, file: "apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx" },
            ],
            files: [{ path: "crates/fleet/src/backoff.rs", act: "write" as const }, { path: "apps/desktop/src/renderer/src/cockpit/deck/BayRows.tsx", act: "edit" as const }],
          }
        : {}),
    },
    ...(stage === 1 ? { account: { done: ["Retried the lease when the pool is contended"], now: "Backing off between attempts", next: "Show the attempt on the bay tile", for: "s1-1", group: "g1", at: ago(1) } } : {}),
  };
}

const prJob: Session = {
  id: PR_JOB,
  title: "Retry the slot lease",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: ago(1),
  standing: "idle",
  rows: [
    you("s1-1", "14:02:00", "Make the slot lease retry when two Jobs ask for the same bay at once."),
    agent("s1-2", "14:05:10", "The plan is recorded: four tasks in two groups. Approve it, or tell me what to change."),
  ],
  attachments: [],
};

// ---- The spike Job -------------------------------------------------------------------------------------------

function spikeGroups(stage: number): WorkGroup[] {
  const state = (built: WorkTaskState): WorkTaskState => (stage === 0 ? "open" : stage === 1 ? built : "done");
  return [{ id: "k1", agent: "general-purpose", title: "Measure the contention", paths: ["crates/fleet"], tasks: [task("K1", "Count lease retries over a day of Jobs", state("done")), task("K2", "Write down what it found", state("working"))] }];
}

function spikeAt(stage: number): Partial<Session> {
  return {
    turn: { state: stage === 1 ? "working" : "idle" },
    standing: stage === 1 ? "working" : "idle",
    attachments: stage === 2 ? [{ kind: "artifact", form: "doc", id: "docs/spikes/lease-contention.md", title: "Lease contention findings" }] : [],
    waitingFor: stage === 0 ? [planCall(undefined, ago(1))] : [],
    work: {
      proposal: { workflow: SPIKE.name },
      landing: "spike",
      workflow: { ...SPIKE, at: stage === 0 ? 0 : stage === 1 ? 1 : 2 },
      plan: { scope: ["crates/fleet"], approved: stage > 0, after: "s2-2", groups: spikeGroups(stage) },
      ...(stage === 1 ? { field: [{ id: "main", path: "crates/fleet", doing: "read" as const, file: "crates/fleet/src/lease.rs" }] } : {}),
    },
  };
}

const spikeJob: Session = {
  id: SPIKE_JOB,
  title: "Measure lease contention",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  standing: "idle",
  rows: [
    you("s2-1", "13:50:00", "How often do two Jobs ask for the same bay? Measure it and write down what you find."),
    agent("s2-2", "13:52:30", "The plan is recorded: two tasks in one group. Approve it, or tell me what to change."),
  ],
  attachments: [],
};

// ---- The report Job ------------------------------------------------------------------------------------------

function reportGroups(stage: number): WorkGroup[] {
  const state = (built: WorkTaskState): WorkTaskState => (stage === 0 ? "open" : stage === 1 ? built : "done");
  return [{ id: "r1", agent: "general-purpose", title: "Trace the lease clock", paths: ["crates/fleet"], tasks: [task("R1", "Find where the lease clock starts", state("done")), task("R2", "Compare it with when a Job first writes", state("working")), task("R3", "Write down what it found", state("open"))] }];
}

function reportAt(stage: number): Partial<Session> {
  return {
    turn: { state: stage === 1 ? "working" : "idle" },
    standing: stage === 1 ? "working" : "idle",
    attachments: stage === 2 ? [{ kind: "artifact", form: "file", id: LEASE_REPORT_PATH, title: "Why do slot leases expire early" }] : [],
    waitingFor: stage === 0 ? [planCall(undefined, ago(1))] : [],
    work: {
      proposal: { workflow: INVESTIGATE.name },
      landing: "report",
      workflow: { ...INVESTIGATE, at: stage === 0 ? 0 : stage === 1 ? 1 : 2 },
      plan: { scope: ["crates/fleet"], approved: stage > 0, after: "s3-2", groups: reportGroups(stage) },
      ...(stage === 1 ? { field: [{ id: "main", path: "crates/fleet", doing: "read" as const, file: "crates/fleet/src/lease.rs" }] } : {}),
    },
  };
}

const reportJob: Session = {
  id: REPORT_JOB,
  title: "Why do slot leases expire early",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: ago(3),
  standing: "idle",
  rows: [
    you("s3-1", "13:30:00", "Why do slot leases expire early? Find the cause and write it up. Change nothing."),
    agent("s3-2", "13:33:20", "The plan is recorded: three tasks in one group. Approve it, or tell me what to change."),
  ],
  attachments: [],
};

// ---- Another Job's pull request, so the Merge line has more than this Job's own -----------------------------

const otherJob: Session = {
  id: OTHER_JOB,
  title: "Cap the log reader",
  manifestId: OWNER,
  cwd: ROOT,
  model: "sonnet",
  effort: "medium",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: ago(9),
  standing: "idle",
  rows: [you("s4-1", "13:10:00", "Cap the log reader's buffer."), agent("s4-2", "13:25:00", "The pull request is in the merge queue.")],
  attachments: [
    { kind: "branch", name: "armada/log-cap", slot: 4 },
    { kind: "pull_request", number: 2371, title: "Cap the log reader's buffer", branch: "armada/log-cap", address: "https://git.example/armada/pull/2371", checks: { state: "passed" }, state: "open", auto: true, queued: true },
  ],
};

/** The repository's merge line: this Job's pull request joins it once the build lands, behind the other Job's and beside one nobody here holds. */
function line(joined: boolean): MergeLine {
  const pull = (number: number, title: string, branch: string, extra: object) => ({ number, title, branch, url: `https://git.example/armada/pull/${number}`, ...extra });
  return {
    root: ROOT,
    line: [],
    off: [],
    landed: [],
    sent_back: [],
    hub: {
      pull_requests: [
        pull(2371, "Cap the log reader's buffer", "armada/log-cap", { ci: "passed", queue: { state: "queued", position: 1 } }),
        ...(joined ? [pull(2390, "Retry the slot lease", BRANCH, { ci: "passed", queue: { state: "queued", position: 2 } })] : []),
        pull(2386, "Pin the store clock", "fix/pin-store-clock", { ci: "running" }),
      ],
    },
  };
}

// ---- The store -----------------------------------------------------------------------------------------------

const AT: Record<string, (stage: number) => Partial<Session>> = { [PR_JOB]: prAt, [SPIKE_JOB]: spikeAt, [REPORT_JOB]: reportAt };

function sessions() {
  const base = sessionsStore([{ ...prJob, ...prAt(0) }, { ...spikeJob, ...spikeAt(0) }, { ...reportJob, ...reportAt(0) }, otherJob], [], new Map(), []);
  const stage = new Map<string, number>([[PR_JOB, 0], [SPIKE_JOB, 0], [REPORT_JOB, 0]]);
  let rowId = 0;
  const row = (text: string, from: "you" | "agent"): SessionRow => {
    rowId += 1;
    const at = `14:${String(10 + rowId).padStart(2, "0")}:00`;
    return from === "you" ? you(`s-r${rowId}`, at, text) : agent(`s-r${rowId}`, at, text);
  };
  const go = (id: string, to: number, rows: SessionRow[]) => {
    stage.set(id, to);
    base.edit(id, (one) => {
      const next = AT[id]!(to);
      return { ...one, ...next, rows: [...one.rows, ...rows], waitingFor: next.waitingFor ?? withoutPlanCall(one.waitingFor) };
    });
  };
  return {
    ...base,
    // The report is a file on the pretend disk, which the Landing panel reads the first lines of.
    readArtifact: async (id: string, path: string) => (path === LEASE_REPORT_PATH ? { ok: true as const, bytes: new TextEncoder().encode(LEASE_REPORT), type: "text/markdown" } : (base.readArtifact?.(id, path) ?? { ok: false as const, why: "unreadable" as const })),
    work: {
      workflows: WORKFLOWS,
      choose: () => undefined,
      unpick: () => undefined,
      approve: (id: string) => go(id, 1, [row("Approved the plan.", "you"), row("Starting on the first group.", "agent")]),
      suggest: () => undefined,
      sendConcern: () => undefined,
      dismissConcern: () => undefined,
    },
    // Each moment the walk lets pass moves an approved Job on to its landing.
    later() {
      for (const [id, at] of stage) {
        if (at !== 1) continue;
        go(id, 2, [row(id === PR_JOB ? "The checks pass and the pull request is in the merge queue." : id === REPORT_JOB ? "The report is written." : "The findings are written up.", "agent")]);
      }
    },
  };
}

function build(): Scenario {
  const base = holding("job-story", "A Job's right column as a story: Blueprint while the plan is drafted, Field while drones work, Landing at the end, by what the Job lands in");
  return {
    ...base,
    state: { ...base.state, repository: null, mergeLines: { lines: [line(false)] } },
    // The first moment the walk lets pass lands the pull request's build: its pull request joins the line.
    later: [{ mergeLines: { lines: [line(true)] } }],
    draft: { sessions, areas: () => mockAreas("/armada") },
  };
}

export const sJobStory: Scenario = build();
