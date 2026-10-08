// Which failure a command Fleet answered with a bare 404 becomes — no window,
// no Fleet socket.
//
// **The pending list decides it, and only for the routes it names.** A 404 on a
// route `packages/protocol/src/pending.ts` lists is the act arriving before its
// route, `bridge.not_implemented`. A 404 anywhere else is still the two sides
// disagreeing about a route, which is what `unanswerable` has always said.

import { expect, test } from "vitest";
import type { BridgeIdentity, Connection } from "@armada/protocol";
import { refusedWith } from "@armada/protocol";
import { failingIn } from "./failing";

const BRIDGE: BridgeIdentity = { auditPath: null, fleetProtocol: null };

const CONNECTED: Connection = {
  state: "connected",
  fleet: {
    protocolId: "0000000000000000",
    pid: 61372,
    port: 40000,
    startedAt: "Mon Sep 14 14:22:06 2026",
  },
  cursor: 0,
};

/** What the window shows for a command that met a bare 404 on `path`. */
function failedAt(path: string) {
  const outcome = refusedWith(404, "", { method: "POST", path });
  return failingIn({
    connection: CONNECTED,
    bridge: BRIDGE,
    readAt: 0,
    outcome,
    raised: [],
    now: 0,
  }).commandFailure;
}

test("a 404 on a pending route is not implemented, and names its issue", () => {
  const failure = failedAt("/jobs/01J/tasks/T1/pilot");

  expect(failure?.payload.code).toBe("bridge.not_implemented");
  expect(failure?.headline).toBe("Not implemented");
});

test("a 404 on a route nothing lists as pending still reads as the two sides disagreeing", () => {
  const failure = failedAt("/jobs/01J/kill_drone");

  expect(failure?.payload.code).toBe("bridge.command.unanswerable");
  expect(failure?.headline).toBe("Fleet answered 404 on POST /jobs/01J/kill_drone, and Bridge could not read it");
});

test("a pending route's other status is not claimed as unbuilt: only the router's 404 is", () => {
  const outcome = refusedWith(500, "", { method: "POST", path: "/jobs/01J/tasks/T1/pilot" });
  const failure = failingIn({
    connection: CONNECTED,
    bridge: BRIDGE,
    readAt: 0,
    outcome,
    raised: [],
    now: 0,
  }).commandFailure;

  expect(failure?.payload.code).toBe("bridge.command.unanswerable");
});
