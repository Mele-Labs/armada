// The Cockpit while answers go to Fleet. A Session asks two questions: Fleet takes the first, slowly,
// and drops it from the Session a moment after it answered; Fleet refuses the second. Beside them a
// Job whose workflow is still being settled, which lands later. The walk `cockpit-pending` plays it,
// and each wait stands until one of its steps lets Fleet go (`cockpit-routes.ts`).

import { featureRunning } from "@armada/jobs/fake";
import { proposing, running } from "@armada/jobs/fixtures/build/index";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { REFUSES, SLOW_FLEET, gate } from "../cockpit-routes";
import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** The picked repository's, started `minutes` ago. */
function live(fixture: JobFixture, minutes: number, extra: Record<string, unknown> = {}): JobFixture {
  const patch = { owner_manifest_id: OWNER, started_at: ago(minutes), ...extra };
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, ...patch };
  return { ...fixture, job };
}

const cache = live(asRow(featureRunning(), 81, "cache", "Cache the manifest read between dispatches"), 17, { current_step_id: "tests" });
const mainChecks = live(asRow(running(), 82, "fix-main", "Fix components_test on main"), 9);
/** Dispatched a moment ago: the proposer has not settled a workflow, so its tile holds the steps' place. */
const dispatched = asRow(proposing(), 83, "settling", "Split the settings reducer so the selectors can be tested");
const settling: JobFixture = { ...dispatched, job: { ...dispatched.job, owner_manifest_id: OWNER, workflow_id: "", created_at: ago(1) } };

const SESSION: Session = {
  id: "s14",
  title: "Pin the store clock",
  turn: { state: "idle" },
  lastTurn: "14:00",
  lastTurnAt: ago(2),
  rows: [
    { id: "p-u", at: "13:58:00", kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests, and tidy the retry cap while you are there." },
    { id: "p-a", at: "14:00:11", kind: "message", from: { kind: "agent" }, text: "Two things before I go on." },
  ],
  attachments: [],
  waitingFor: [
    { id: "q1", text: "Which clock should the store tests use?", since: ago(3), source: "ask_card", act: { kind: "answer", target: "q-1" }, options: [{ label: "A fake the test sets" }, { label: "The real one, stopped" }] },
    { id: "q2", text: "How many retries should the cap allow?", since: ago(2), source: "ask_card", act: { kind: "answer", target: "q-2" }, options: [{ label: "Keep three" }, { label: "Raise it to five" }] },
  ],
};

// Fleet turns the second question down: the Session moved on while it sat there.
REFUSES.set("q2", { code: "fleet.session_busy", message: "Session Pin the store clock is already working on that" });

function build(): Scenario {
  const board = [cache, mainChecks, settling];
  const base = holding("cockpit-pending", "The Cockpit with two questions waiting on a Session, answered slowly by Fleet, and a Job whose workflow is still being settled", board);
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  const jobs = board.map((one) => one.job);
  const landed = { ...settling.job, workflow_id: cache.job.workflow_id };
  return {
    ...base,
    state: { ...base.state, repository: null, jobs },
    // Each moment is a step of the walk. The first three only let Fleet go (the answer, then the item
    // dropping, then the refusal); the fourth is the proposer settling the workflow, which brings the
    // tile's steps onto it.
    later: [{}, {}, {}, { jobs: [cache.job, mainChecks.job, landed] }],
    draft: {
      sessions: (control) => {
        const store = sessionsStore([none, none], none, [], control, [], [SESSION]);
        const { held, ...clocks } = SLOW_FLEET;
        // Fleet answers when the walk lets it, so the waiting state stands as long as a person wants to look at it.
        const hold = held ? gate() : undefined;
        return {
          ...store,
          pace: { ...clocks, ...(hold === undefined ? {} : { held: hold }) },
          later: () => (store.later(), hold?.release()),
        };
      },
    },
  };
}

export const s208CockpitPending: Scenario = build();
