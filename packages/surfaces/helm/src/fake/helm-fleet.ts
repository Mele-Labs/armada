// A Fleet with Helm pointed at a repository and a session to report — the one
// moment every other scenario is missing. Every scenario before this
// published `helm: { state: "none" }`, so the dock's own controls — the
// composer in its pointed state, *Start fresh*, and the record's split
// button — could not be reached by opening the mock at all. Three agents in one
// night built the state inline to screenshot it, and the last of them threw the
// scenario away again (#1488).
//
// **Helm's thread is the Helm Session's**, which the window holds with every other Session; the
// scenarios that draw it are `desktop/mock/scenarios/helm-is-a-session.ts` and `route-a-thought.ts`.
// **The record is about one Session**, and Helm's reply renders that check run's own `expected` and
// `produced` rather than paraphrasing them — `docs/contracts/agent-copy.md`, *Render a record, never
// paraphrase it*.

import { connected } from "@armada/bridge-api";
import type { PublishedCore, Scenario } from "@armada/bridge-api";
import { PROTOCOL_ID } from "@armada/protocol";
import type { HelmDebugInfo } from "@armada/protocol";
import { MANIFEST_ID, repository } from "@armada/screens/src/fixtures/repository";

import type { HelmApi, HelmState } from "../api";

/**
 * What Helm answered, whole. **Every fact in it is the check's own**: its
 * `expected` and `produced` strings verbatim. No "I" — reading a record is not
 * something Helm did.
 */
const ANSWER = [
  "Session 77 stopped on a failing check. cargo_nextest failed on all three runs.",
  "Expected — cargo nextest run --workspace exits 0",
  "Produced — exit 101 — 3 of 2034 tests failed",
  "The worktree is still on disk.",
].join("\n\n");

/**
 * The brief, as this session was given it. **Fleet's own opening and its
 * repository block**, from `crates/fleet/src/helm/brief.rs`; the whole brief
 * runs to nine blocks and the rest of them say nothing this scenario is for.
 */
const NOTE = [
  "You are Helm, in Armada. A person asks you about the work in one repository, and about the " +
    "repository itself. You answer from what your tools return: Fleet's, for Sessions and " +
    "what is waiting on a person, and the ordinary ones for reading, searching, editing and " +
    "running things. Saying that you did something does not do it. Only a tool call does.",
  `THIS REPOSITORY\n\nEvery question in this conversation is about Manifest ${MANIFEST_ID}, read ` +
    "from armada. The Fleet tools answer inside it and reach nothing outside it, so when you are " +
    "asked about another repository, say it cannot be answered here.",
].join("\n\n");

/**
 * The record `GET /helm/debug` answers for this session — the same conversation
 * from Fleet's side, so what the split button copies and what the thread draws
 * are about one session rather than two unrelated fixtures.
 */
const RECORD: HelmDebugInfo = {
  manifest_id: MANIFEST_ID,
  checkout: "/Users/user/armada",
  authority: "acting",
  model: "sonnet",
  note: NOTE,
  door: "armada-fleet",
  tools: ["list_sessions", "get_check_output", "get_diff", "get_events_since"],
  session: "01M2C1TJ8G00HELMSESSION01",
  thread: [
    { at: "2026-09-10T14:31:06.000Z", line: "asked", text: { text: "why did 77 stop" } },
    { at: "2026-09-10T14:31:11.000Z", line: "called", tool: "list_sessions", detail: "01M2C1TJ8G0077SESSION0001" },
    { at: "2026-09-10T14:31:19.000Z", line: "said", text: { text: ANSWER } },
    {
      at: "2026-09-10T14:33:48.000Z",
      line: "asked",
      text: { text: "which test is it, and does it fail on main too" },
    },
  ],
  cut: 0,
  polled: {
    from: 812,
    upto: 851,
    kinds: [
      { kind: "session.row", count: 3 },
    ],
  },
  run_id: "01M2C1TJ8G00FLEETRUN0001",
  protocol_id: PROTOCOL_ID,
  at: "2026-09-10T14:34:11.402Z",
};

/**
 * Helm pointed at `armada`, with a session to report on.
 *
 * **One repository set up, so the composer draws no switch**: pointed at a
 * repository, one is nothing to switch between, and a second served here would
 * only be there to make a control appear. `fleet-not-running` through
 * `default` are where the unpointed dock and its switch are read.
 *
 * `behaves` answers `GET /helm/debug` alone. **Asking still moves nothing** —
 * a reply is Fleet's to decide and the fake invents none — so Send clears the
 * field and the thread stands, which is `running-locally.md`'s rule for every
 * other act a Fleet would have to answer.
 */
export function talking<S extends PublishedCore & HelmState, A extends HelmApi>(nothingYet: S): Scenario<S, A> {
  return {
    name: "helm-talking",
    says: "Helm pointed at a repository, with a job to report",
    state: {
      ...connected(nothingYet, [], [repository()]),
      helm: { state: "pointed", manifestId: MANIFEST_ID },
    },
    // `A` is the app's whole API, so the one member this answers is narrowed to it by hand.
    behaves: () => ({ helmDebugInfo: async () => ({ ok: true, record: RECORD }) }) as Partial<A>,
  };
}
