// The Dashboard with a good deal waiting on the owner: two Jobs that ask and have gone wrong, a
// Job at its review, a Session waiting on a command, main red with nobody on it, and a pull request
// the line refused. Beside them, work for the Running tab (a queued Job, a Session at work, a line
// in flight) and for Done (finished Jobs, an ended Session, what landed). The walk
// `dashboard-needs-you` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";
import { mergeLines } from "@armada/screens/src/fixtures/build/merge-line";
import type { Session } from "@armada/screens/src/draft/sessions";
import type { CallView } from "@armada/jobs/draft/calls";
import { featureRunning } from "@armada/jobs/fake";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import { FIXTURES, mainRed } from "../main-red-hub";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;

/** A Job the Drone is asking about: running, flagged as asking, no longer cleared, and the picked repository's. */
function asking(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, asking: true, owner_manifest_id: OWNER };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = { ...fixture.watched.detail.job, asking: true, owner_manifest_id: OWNER };
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const plan = asking(asRow(featureRunning(), 73, "dash-plan", "Split the writer from the clock"));
const issue = asking(asRow(featureRunning(), 74, "dash-issue", "Shorten the reconnect wait"));

const now: Record<string, CallView> = {
  [plan.job.id]: {
    request: "Fleet's event log writer and its heartbeat clock share one struct, so a slow disk stalls the heartbeat and Bridge marks Fleet unreachable. Pull the clock out so the heartbeat never waits on the writer.",
    about: [
      ["Repository", "armada"],
      ["Area", { files: [
        { path: "crates/fleet/src/log/writer.rs", lines: [
          { kind: "hunk", text: "@@ -41,9 +41,7 @@ impl Writer {" },
          { kind: "context", text: "     pub fn tick(&mut self) -> io::Result<()> {" },
          { kind: "removed", text: "-        let now = self.clock.now();" },
          { kind: "removed", text: "-        let _guard = self.buffer.lock();" },
          { kind: "added", text: "+        let _guard = self.buffer.lock();" },
          { kind: "context", text: "         self.flush_if_due(now)?;" },
        ] },
        { path: "crates/fleet/src/log/clock.rs", lines: [
          { kind: "hunk", text: "@@ -0,0 +1,6 @@" },
          { kind: "added", text: "+pub struct HeartbeatClock { started: Instant }" },
          { kind: "added", text: "+impl HeartbeatClock {" },
          { kind: "added", text: "+    pub fn now(&self) -> Duration { self.started.elapsed() }" },
          { kind: "added", text: "+}" },
        ] },
      ] }],
      ["From", { kind: "issue", number: 1742, title: "Bridge says Fleet is unreachable during a slow write", url: "https://github.com/Mele-Labs/armada/issues/1742" }],
      ["Branch", "armada/73-dash-plan"],
    ],
    running: [{ key: "r", of: "drone", name: "Drone on Plan", line: "Weighing the split against the wrap", state: "running" }],
    asks: [
      {
        key: "p",
        kind: "plan",
        context: [
          "Writer::tick() reads Clock::now() under the same lock that guards the buffer.",
          "Splitting touches 6 call sites and the Writer constructor.",
          "Wrapping keeps the lock and adds a second timer; the stall stays.",
        ],
        decisions: [
          {
            id: "shape",
            question: "Split the clock out of the writer, or wrap it in place?",
            options: [
              { id: "split", label: "Split it out" },
              { id: "wrap", label: "Wrap it in place" },
            ],
          },
        ],
      },
      {
        key: "j",
        kind: "judge",
        name: "Judge on Implement",
        text: "Is the retry cap in scope?",
        context: [
          "The diff adds MAX_RETRIES = 5 to writer/flush.rs.",
          "The request names the clock and the heartbeat, not retries.",
          "Reverting it leaves flush() unbounded.",
        ],
      },
    ],
  },
  [issue.job.id]: {
    request: "When Bridge loses its socket to Fleet, it waits a flat 30s before reconnecting. Cut the first wait to 5s and back off from there, keeping the cap at 60s.",
    about: [
      ["Repository", "armada"],
      ["Area", { files: [
        { path: "apps/desktop/src/main/connection.ts", lines: [
          { kind: "hunk", text: "@@ -88,7 +88,8 @@ function reconnect() {" },
          { kind: "removed", text: "-  const wait = 30_000;" },
          { kind: "added", text: "+  const wait = backoff(attempt);" },
          { kind: "added", text: "+  attempt += 1;" },
          { kind: "context", text: "   timer = setTimeout(open, wait);" },
        ] },
        { path: "apps/desktop/src/main/backoff.ts", lines: [
          { kind: "hunk", text: "@@ -0,0 +1,3 @@" },
          { kind: "added", text: "+export function backoff(attempt: number): number {" },
          { kind: "added", text: "+  return Math.min(5_000 * 2 ** attempt, 60_000);" },
          { kind: "added", text: "+}" },
        ] },
        { path: "crates/store/tests/reconnect.rs", lines: [
          { kind: "hunk", text: "@@ -12,4 +12,4 @@ fn waits_five_seconds() {" },
          { kind: "removed", text: "-    assert_eq!(first_wait(), Duration::from_secs(30));" },
          { kind: "added", text: "+    assert_eq!(first_wait(), Duration::from_secs(5));" },
        ] },
      ] }],
      ["From", { kind: "job", number: 61, title: "Debounce Bridge reconnects after a Fleet restart", jobId: "dash-running" }],
      ["Branch", "armada/74-dash-issue"],
    ],
    running: [{ key: "r", of: "drone", name: "Drone on Implement", line: "Waiting on the store test", state: "running" }],
    issues: [
      {
        key: "i1",
        of: "check",
        name: "store",
        text: "store: 2 failed",
        said: "Check failed",
        context: [
          "FAIL store::reconnect::waits_five_seconds",
          "  expected: 5s",
          "  actual:   30s",
          "FAIL store::reconnect::caps_at_sixty",
          "  assertion failed: backoff(9) <= 60s",
          "test result: FAILED. 41 passed; 2 failed",
        ],
      },
      {
        key: "i2",
        of: "drone",
        name: "Drone on Implement",
        text: "No output for 14m",
        said: "Drone stuck",
        context: [
          "Running cargo test -p store reconnect",
          "Reading backoff.rs again",
          "Editing backoff.rs: first delay 5s",
          "Running cargo test -p store reconnect",
        ],
      },
    ],
  },
};

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const quiet = { state: "idle" } as const;
const said = (id: string, text: string) => [{ id, at: "14:00:00", kind: "message", from: { kind: "agent" }, text } as const];

/** One waiting on a command, one at work, one over. */
const SESSIONS: Session[] = [
  {
    id: "s9",
    title: "Flaky store test",
    turn: quiet,
    lastTurn: "14:02",
    lastTurnAt: ago(3),
    rows: [...said("s9-0", "The flaky test is the 200ms sleep in store_test.rs; I replaced it with a wait on the channel."), ...said("s9-1", "The branch is ready to push.")],
    attachments: [],
    asked: { command: "git push --force-with-lease origin fix/flaky-store" },
  },
  { id: "s10", title: "Migration notes", turn: { state: "working" }, lastTurn: "14:05", lastTurnAt: ago(1), rows: [...said("s10-0", "Listing the migrations since 0042."), ...said("s10-1", "Reading the migrations.")], attachments: [] },
  { id: "s11", title: "Release script", dead: "ended", turn: quiet, lastTurn: "11:40", lastTurnAt: ago(150), rows: said("s11-1", "Done."), attachments: [] },
];

function build(): Scenario {
  const base = holding("dashboard-needs-you", "The Dashboard with Jobs asking, a Session waiting and main red", [...FIXTURES, plan, issue]);
  // The fourth moment of main going red: a person merged it by hand, so nobody has the red. Its
  // line is the recorded one beside it, so a turn is in flight and three branches have landed.
  const moment = mainRed(repository().root).later[2]!;
  const red = (moment.mergeLines?.lines ?? [])[0]!;
  const lines = { lines: [{ ...mergeLines().lines[0]!, hub: red.hub }] };
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  return {
    ...base,
    state: { ...base.state, ...moment, repository: repository().root, mergeLines: lines, jobs: [...(moment.jobs ?? []), { ...plan.job, current_step_id: "scope" }, issue.job] },
    draft: { calls: now, sessions: (board) => sessionsStore([none, none], none, [], board, [], SESSIONS) },
  };
}

export const s205DashboardNeedsYou: Scenario = build();
