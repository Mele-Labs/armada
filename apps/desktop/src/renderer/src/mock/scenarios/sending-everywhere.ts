// Words sent to any agent behave the same way: in the thread at once, going, and refused with a Retry. Helm's dock is open on
// armada, which has no Helm Session yet, beside the Sessions and Jobs of `route-a-thought` and one Job with a Drone working.
// Fleet takes Helm's first message slowly, refuses the next and takes it again on Retry, takes words routed to a Session slowly,
// and takes a redirect to the Drone slowly. Each wait stands until a step of the walk lets Fleet go (`walks/sending-everywhere.ts`).

import type { Outcome } from "@armada/protocol";
import { everyDroneHad } from "@armada/screens/src/fixtures/build/drones-had";

import type { BridgeApi } from "../../../../shared/api";
import { gate } from "../cockpit-routes";
import { asRow } from "../holding";
import type { Scenario } from "../moment";
import { onTimePassing } from "../time-passes";
import { s210RouteAThought } from "./route-a-thought";

type Answer = "taken" | "refused";

const drone = (() => {
  const one = asRow(everyDroneHad(), 90, "pin-the-clock-tests", "Pin the clock in the store tests");
  return { ...one, job: { ...one.job, owner_manifest_id: "armada" } };
})();

const refusal = (message: string): Outcome => ({ ok: false, why: "refused", error: { code: "fleet.refused", message, run_id: "", fields: {}, chain: [] } });

/** Fleet's answer, once a step lets it go: the next of `outcomes`, and taken where none is left. */
function slowly<Args extends unknown[], Taken>(held: ReturnType<typeof gate>, outcomes: Answer[], taken: (...args: Args) => Promise<Taken>, refused: Taken): (...args: Args) => Promise<Taken> {
  return async (...args) => {
    await held.wait();
    return outcomes.shift() === "refused" ? refused : taken(...args);
  };
}

export const s265SendingEverywhere: Scenario = {
  ...s210RouteAThought,
  name: "sending-everywhere",
  says: "Words sent to Helm, to a Session and to a Drone are in the thread at once, going, and Fleet takes the first slowly and refuses another",
  state: { ...s210RouteAThought.state, jobs: [...s210RouteAThought.state.jobs, drone.job] },
  reads: { ...s210RouteAThought.reads, [drone.job.id]: drone },
  // The Job with the Drone is open behind Helm's dock, and the walk comes back to it.
  opens: drone.job.id,
  behaves: (handle) => {
    const base = s210RouteAThought.behaves!(handle);
    // One gate for every send, let go one wait at a time by the walk's steps.
    const held = gate();
    // After the window has registered its own, so a step of the walk lets the oldest wait go.
    queueMicrotask(() => onTimePassing(() => held.release()));
    const taken = <K extends keyof BridgeApi>(name: K): NonNullable<BridgeApi[K]> => base[name] as NonNullable<BridgeApi[K]>;
    return {
      ...base,
      askHelm: slowly(held, ["taken", "refused", "taken"], taken("askHelm"), refusal("Fleet did not take it")),
      sendSessionMessage: slowly(held, ["taken"], taken("sendSessionMessage"), { ok: false, outcome: refusal("Fleet did not take it") }),
      redirectDrone: slowly<[], Outcome>(held, ["taken"], async () => ({ ok: true }), refusal("Fleet did not take it")),
    };
  },
};
