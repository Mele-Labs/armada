// A Fleet whose proposer answers, a field at a time: `proposing-fills-in`.
//
// **It publishes `proposal.moved` the way Fleet does** (21.6): the Job at
// `proposing` first, as `job.created` carries it, then a message about once a
// second naming that Job by `job_id`, each carrying everything settled so far,
// in the owner's order — workflow, title, done-when, settings. Each one is
// folded through `movedOnto`, the fold main's `arrivals.ts` calls, so what this
// draws is what Bridge does with Fleet's stream. The call never answers, as
// nothing a Fleet would decide is guessed here.

import type { FleetHandle, Scenario } from "@armada/bridge-api";
import type { JobSummary, ProposalInFlight, ProposalMoved, ProposalSettled } from "@armada/protocol";
import type { JobsApi, JobsState } from "../api";
import type { ArcDraft } from "../fixtures/build/arc";
import { ARC_CRITERIA, ARC_TITLE } from "../fixtures/build/arc-base";
import { dispatchedFixture, dispatchedRow, PROPOSER_BUDGET_MS } from "../fixtures/build/proposing";
import { movedOnto } from "@armada/screens/src/filling";


/** How often Fleet says how far the call has got: its token estimate is throttled to one a second. */
export const PROPOSER_TICK_MS = 1_200;

const PROPOSAL_ID = "01M2E1PROPOSALFILLSIN0001";
const CLIENT_REF = "bridge-1";
/** The Job Fleet creates at the press. Not the arc's own, whose moments carry a draft of their own. */
const JOB_ID = "01M2E1DISPATCHEDFILLSIN01";
const HANDLE = "4-the-rail-says-drones-1-of-2";

const WORKFLOW: ProposalSettled = { workflow_id: "feature" };
const TITLED: ProposalSettled = { ...WORKFLOW, title: ARC_TITLE };
const ONE_LINE: ProposalSettled = { ...TITLED, done_when: [ARC_CRITERIA[0]!.text] };
const DONE_WHEN: ProposalSettled = { ...TITLED, done_when: ARC_CRITERIA.map((one) => one.text) };
const SETTINGS: ProposalSettled = { ...DONE_WHEN, settings: { urgency: "normal", model: "opus" } };

/** Each message after the first, in order: how far the call has got, and what has settled. */
const MOVES: readonly Pick<ProposalInFlight, "reached" | "thinking_tokens" | "answered_characters" | "settled">[] = [
  { reached: "requesting" },
  { reached: "thinking", thinking_tokens: 420 },
  { reached: "answering", thinking_tokens: 610, answered_characters: 40, settled: WORKFLOW },
  { reached: "answering", thinking_tokens: 610, answered_characters: 90, settled: WORKFLOW },
  { reached: "answering", thinking_tokens: 610, answered_characters: 150, settled: TITLED },
  { reached: "answering", thinking_tokens: 610, answered_characters: 210, settled: TITLED },
  { reached: "answering", thinking_tokens: 610, answered_characters: 300, settled: ONE_LINE },
  { reached: "answering", thinking_tokens: 610, answered_characters: 380, settled: DONE_WHEN },
  { reached: "answering", thinking_tokens: 610, answered_characters: 430, settled: DONE_WHEN },
  { reached: "answering", thinking_tokens: 610, answered_characters: 470, settled: SETTINGS },
  // And on, the answer still arriving, so a page opened late fills on the next message.
  ...Array.from({ length: 20 }, (_, at) => ({
    reached: "answering" as const,
    thinking_tokens: 610,
    answered_characters: 480 + at * 10,
    settled: SETTINGS,
  })),
];

/** One `proposal.moved`, exactly as Fleet sends it for a call this window started. */
function moved(since: string, at: string, step: Pick<ProposalInFlight, "reached">): ProposalMoved {
  return {
    proposal_id: PROPOSAL_ID,
    job_id: JOB_ID,
    client_ref: CLIENT_REF,
    proposing: {
      proposal_id: PROPOSAL_ID,
      client_ref: CLIENT_REF,
      model: "sonnet",
      since,
      budget_ms: PROPOSER_BUDGET_MS,
      ...step,
    },
    actor: "human",
    at,
  };
}

/** What main's arrival does with one: this window's own wait, and the fold onto the Job it names. */
function arrived(fleet: FleetHandle<FillingInState>, message: ProposalMoved): void {
  const state = fleet.state();
  const watched = state.watched;
  const open = watched.state === "read" && watched.jobId === message.job_id ? watched : undefined;
  const filling = movedOnto(state.jobs, open?.detail, message);
  fleet.publish({
    proposing: message.proposing ?? null,
    ...(filling === null ? {} : { jobs: filling.jobs }),
    ...(open === undefined || filling?.detail === undefined ? {} : { watched: { ...open, detail: filling.detail } }),
  });
}

/** The state the proposer writes, and the member it answers: both inside the app's whole state `S` and API `A`. */
export type FillingInState = Pick<JobsState, "proposing" | "watched"> & { jobs: JobSummary[] };
export type FillingInApi = Pick<JobsApi, "proposeFromRequest">;

/** `base` — the composer with a request typed — over a Fleet whose proposer fills the Job in. */
export function fillingIn<S extends FillingInState, A extends FillingInApi>(
  base: Scenario<S, A, ArcDraft>,
): Scenario<S, A, ArcDraft> {
  // Held, not copied, so the Job Fleet creates at the press is one its reads can answer for.
  const reads = { ...base.reads };
  // The moment's own proposal is a draft of the arc's answer, drawn on whichever Job is open; this
  // Job's answer is what the messages settle, so the draft keeps the composer's half and no more.
  const { proposal: _arcs, ...composing } = base.draft ?? {};
  return {
    ...base,
    draft: composing,
    name: "proposing-fills-in",
    says: "Dispatch a request and watch the proposer fill the Job in, a field at a time",
    reads,
    // `S` is the app's whole state; this Fleet writes only the fields `FillingInState` names.
    behaves: (whole) => {
      const fleet = whole as unknown as FleetHandle<FillingInState>;
      const members: FillingInApi = {
        proposeFromRequest: (request) => {
          const since = new Date().toISOString();
          const dispatched = { id: JOB_ID, handle: HANDLE, request, created_at: since, says: "proposing" };
          const row = dispatchedRow(dispatched);
          // `GET /jobs/:id` at `proposing`: the request as the title and as the brief, as
          // `crates/fleet/src/dispatched.rs` creates it.
          const read = dispatchedFixture(dispatched, Date.now());
          if (read.watched.state === "read") {
            read.watched = { ...read.watched, detail: { ...read.watched.detail, facts: request } };
          }
          reads[JOB_ID] = read;
          fleet.publish({ jobs: [...fleet.state().jobs, row] });
          arrived(fleet, moved(since, since, { reached: "starting" }));
          MOVES.forEach((step, at) =>
            setTimeout(() => arrived(fleet, moved(since, new Date().toISOString(), step)), (at + 1) * PROPOSER_TICK_MS),
          );
          // The call is still out, so nothing waits on an answer.
          return new Promise(() => {});
        },
      };
      return members as Partial<A>;
    },
  };
}
