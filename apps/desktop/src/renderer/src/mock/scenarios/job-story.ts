// A Job's right column as a story. Two Jobs on the Cockpit, each with a plan it has just recorded: one that lands in a pull request and one that
// lands in a spike's findings doc. Approving a plan starts the build (the Field opens), and each time the walk lets time pass the build comes
// closer to its landing: for the pull request, Checks passed, the queue, an issue filed, a watch and a follow-up; for the spike, the findings doc.
// Mock only: the stage is read in Bridge from the Job's data (`session-view/story.ts`), and `work.landing` names the kind. The walks `jobStory`
// and `jobStorySpike` play it.

import type { WatchSummary } from "@armada/protocol";
import type { Session, SessionAttachment, SessionRow, SessionWork, WorkGroup, WorkTaskState, WorkflowChoice } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { mockAreas } from "../cockpit-areas";
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
const BRANCH = "armada/lease-retry";

const FEATURE: WorkflowChoice = { id: "Feature", name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"] };
const SPIKE: WorkflowChoice = { id: "Spike", name: "Spike", steps: ["Plan", "Measure", "Write up"] };
const WORKFLOWS = [FEATURE, SPIKE];

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
    { id: "g1", agent: "rust-engineer", title: "Retry the lease when the pool is contended", paths: ["crates/fleet"], tasks: [task("T1", "Retry the lease when the pool is contended", at("open", "done", "done")), task("T2", "Back off between attempts", at("open", "working", "done"))] },
    { id: "g2", agent: "bridge-engineer", title: "Show the attempt on the bay tile", paths: ["apps/desktop"], tasks: [task("T3", "Show the attempt on the bay tile", at("open", "working", "done")), task("T4", "Name the attempt in the tooltip", at("open", "open", "done"))] },
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
  const pr = (extra: Pick<Extract<SessionAttachment, { kind: "pull_request" }>, "checks" | "state" | "auto"> & { queued?: true; closes?: number[] }): SessionAttachment => ({ kind: "pull_request", number: 2390, title: "Retry the slot lease", branch: BRANCH, address: "https://git.example/armada/pull/2390", ...extra });
  return {
    turn: { state: stage === 1 ? "working" : "idle" },
    standing: stage === 1 ? "working" : "idle",
    attachments: [
      ...(stage === 0 ? [] : [{ kind: "branch" as const, name: BRANCH, slot: 3 }]),
      ...(stage === 1 ? [pr({ checks: { state: "pending" }, state: "draft", auto: false })] : []),
      ...(stage === 2
        ? [
            pr({ checks: { state: "passed" }, state: "open", auto: true, queued: true, closes: [2402] }),
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

// ---- The store -----------------------------------------------------------------------------------------------

const AT: Record<string, (stage: number) => Partial<Session>> = { [PR_JOB]: prAt, [SPIKE_JOB]: spikeAt };

function sessions() {
  const base = sessionsStore([{ ...prJob, ...prAt(0) }, { ...spikeJob, ...spikeAt(0) }], [], new Map(), []);
  const stage = new Map<string, number>([[PR_JOB, 0], [SPIKE_JOB, 0]]);
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
        go(id, 2, [row(id === PR_JOB ? "The checks pass and the pull request is in the merge queue." : "The findings are written up.", "agent")]);
      }
    },
  };
}

function build(): Scenario {
  const base = holding("job-story", "A Job's right column as a story: Blueprint while the plan is drafted, Field while drones work, Landing at the end, by what the Job lands in");
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: { sessions, areas: () => mockAreas("/armada") },
  };
}

export const sJobStory: Scenario = build();
