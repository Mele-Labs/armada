// A `window.armada` with no main process behind it, answering from a scenario.
//
// **Composed from the slices in `slices.ts`, typed as `BridgeApi` member by member, never a
// `Proxy`**, so a capability added to the preload fails typecheck there until a slice answers it.
// What each kind of call answers, and why no more: `docs/practices/running-locally.md`,
// *Bridge on a mock Fleet*.

import type { ArcDraft } from "@armada/jobs/fixtures/build/arc";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState } from "../../../shared/bridge";
import { fleetOf } from "./fake-context";
import type { Fleet, SliceName } from "./fake-context";
import { unanswered } from "./moment";
import type { Scenario } from "./moment";
import { SLICES } from "./slices";
import type { AnySlice } from "./slices";
import { cockpitRoutes } from "./cockpit-routes";
import type { SessionsStore } from "./sessions/script";
import { onTimePassing } from "./time-passes";

/** Which surfaces a fake answers for. Core is always among them. */
export type FakeOptions = { slices?: readonly SliceName[] };

/** A fake's draft as it stands, and a way to hear it change. */
export type LiveDraft = {
  current: () => ArcDraft | undefined;
  subscribe: (onDraft: () => void) => () => void;
};

/**
 * Each fake's own draft, by the api it answers as. **Beside `BridgeApi` rather
 * than on it**: the draft is the mock's stand-in for reads Fleet does not serve
 * (`moment.ts`' `draft`), and the preload has no such member to type it by.
 */
const DRAFTS = new WeakMap<BridgeApi, LiveDraft>();

/** The Sessions a window was mounted holding, by the api it talks to. */
const SESSIONS = new WeakMap<BridgeApi, SessionsStore>();

/** The Sessions `api`'s fake holds, where it holds any. `mount.tsx` hands them to the window. */
export function heldSessions(api: BridgeApi): SessionsStore | undefined {
  return SESSIONS.get(api);
}

/** The draft `api`'s fake holds, where `api` is one. `mount.tsx` draws from it. */
export function liveDraft(api: BridgeApi): LiveDraft | undefined {
  return DRAFTS.get(api);
}

/**
 * A window's own `window.armada`, over one scenario. Each call makes a fresh one.
 * **A surface left out of `options.slices`** keeps its state fields empty and answers each of its
 * calls as a route the mock has none for — `unanswered`.
 */
export function fakeBridge(scenario: Scenario, options: FakeOptions = {}): BridgeApi {
  const listed = options.slices;
  const kept = listed === undefined ? SLICES : SLICES.filter((one) => one.name === "core" || listed.includes(one.name));
  const left = SLICES.filter((one) => !kept.includes(one));
  const unseeded: BridgeState = left.length === 0 ? scenario.state : { ...scenario.state, ...Object.assign({}, ...left.map((one) => one.state)) };
  const initial: BridgeState = Object.assign({}, unseeded, ...kept.map((one) => one.seeded?.(unseeded) ?? {}));
  const fleet = fleetOf(scenario, initial);
  const api = compose(kept, left, scenario, fleet);
  const own = scenario.behaves?.({ state: fleet.state, publish: fleet.publish });
  const fake = { ...api, ...own };
  let passed = 0;
  // What a Session may do to the Board: a Job's status moves on its row, on its detail and on the read a later open gets.
  const board = {
    add: (rows: readonly unknown[]) => {
      const held = fleet.state().jobs;
      fleet.publish({ jobs: [...held, ...(rows as typeof held).filter((row) => !held.some((one) => one.id === row.id))] } as Partial<BridgeState>);
    },
    setStatus: (jobId: string, status: string) => {
      const now = fleet.state();
      const watched = now.watched;
      fleet.publish({
        jobs: now.jobs.map((one) => (one.id === jobId ? { ...one, status, reason: undefined } : one)),
        ...(watched.state === "read" && watched.jobId === jobId ? { watched: { ...watched, detail: { ...watched.detail, job: { ...watched.detail.job, status, reason: undefined } } } } : {}),
      } as Partial<BridgeState>);
      const read = scenario.reads[jobId];
      if (read !== undefined) {
        read.job = { ...read.job, status, reason: undefined } as typeof read.job;
        if (read.watched.state === "read") read.watched = { ...read.watched, detail: { ...read.watched.detail, job: { ...read.watched.detail.job, status, reason: undefined } } } as typeof read.watched;
      }
    },
  };
  const sessions = scenario.draft?.sessions?.(board);
  onTimePassing(
    scenario.later === undefined && sessions === undefined
      ? undefined
      : () => {
          const change = scenario.later?.[passed++];
          if (change !== undefined) fleet.publish(change);
          sessions?.later();
        },
  );
  if (sessions !== undefined) {
    SESSIONS.set(fake, sessions);
    // The Cockpit's two session routes over those Sessions, unless the scenario serves its own or leaves Sessions out.
    if (kept.some((one) => one.name === "sessions")) {
      const served = cockpitRoutes(sessions, fleet.state, fleet.publish);
      if (own?.answerWaiting === undefined) fake.answerWaiting = served.answerWaiting;
      if (own?.claimPullRequest === undefined) fake.claimPullRequest = served.claimPullRequest;
      if (own?.dismissWaiting === undefined) fake.dismissWaiting = served.dismissWaiting;
    }
  }
  DRAFTS.set(fake, { current: fleet.draft.get, subscribe: fleet.draft.subscribe });
  return fake;
}

/**
 * Each slice's members in one object. A left-out slice is still built, for the names of what it
 * answers: each of its calls that returns a promise answers `unanswered` instead, as an outcome and
 * as the `{ ok, outcome }` of a read. Calls that return at once keep the slice's own, which is inert.
 */
function compose(kept: readonly AnySlice[], left: readonly AnySlice[], scenario: Scenario, fleet: Fleet): BridgeApi {
  const unanswering = (slice: AnySlice): Partial<BridgeApi> =>
    Object.fromEntries(
      Object.entries(slice.api(scenario, fleet)).map(([member, real]) => [
        member,
        typeof real === "function" && real.constructor.name === "AsyncFunction"
          ? async () => {
              const outcome = unanswered(`/${slice.name}/${member}`);
              return { ...outcome, outcome };
            }
          : real,
      ]),
    );
  return Object.assign({}, ...kept.map((one) => one.api(scenario, fleet)), ...left.map(unanswering)) as BridgeApi;
}
