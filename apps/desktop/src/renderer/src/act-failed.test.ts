// A background act that failed after its reply reaches the window as the toast a refusal shows: Fleet's words,
// and the act with what it was for in the fields Copy debug info carries.

import { expect, test } from "vitest";
import type { ActFailed, BridgeIdentity } from "@armada/protocol";
import { actFailedToTell, tellActFailures } from "./act-failed";
import { raisedFailure } from "./failing";
import { actFields } from "./raised";
import type { RefusedAct } from "./tell";

const BRIDGE: BridgeIdentity = { auditPath: null, fleetProtocol: null };

const FAILED: ActFailed = {
  act: "checkout_to_pull_request",
  manifest_id: "armada",
  error: { code: "fleet.push_refused", message: "The push was refused", run_id: "run-1", fields: {}, chain: [] },
};

test("an act.failed event tells the refusal's words with the act and what it was for", () => {
  let heard: ((failed: ActFailed) => void) | null = null;
  const told: { sentence: string; act: RefusedAct }[] = [];
  const off = tellActFailures(
    { onActFailed: (on) => ((heard = on), () => (heard = null)) },
    (sentence, act) => told.push({ sentence, act }),
  );
  expect(heard).not.toBeNull();
  heard!(FAILED);
  expect(told).toEqual([
    { sentence: "The push was refused", act: { act: "checkout_to_pull_request", request: { code: "fleet.push_refused", manifest_id: "armada" } } },
  ]);
  off();
  expect(heard).toBeNull();
});

test("the told sentence stands as the refusal toast, and a part the act lacked is not a field", () => {
  const { sentence, act } = actFailedToTell(FAILED);
  const error = { code: "bridge.not_sent", message: sentence, run_id: "", chain: [], fields: actFields(act) };
  const failure = raisedFailure({ key: 0, outcome: { ok: false, why: "refused", error } }, BRIDGE);
  expect(failure?.headline).toBe("The push was refused");
  expect(error.fields).toEqual({ act: "checkout_to_pull_request", code: "fleet.push_refused", manifest_id: "armada" });
});
