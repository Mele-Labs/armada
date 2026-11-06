// Each failure card is sent to the Operator once, with the facts the card carries, and a send that fails is never a card.

import { afterEach, expect, test, vi } from "vitest";
import type { BridgeIdentity, ReportBridgeError } from "@armada/protocol";
import { buildFailedFailure, failureShown, refusalFailure } from "@armada/shell";
import { failureReporter, reportedError, SAME_CARD_MS, wireFailureReports } from "./report-failure";
import type { OperatorErrorSent } from "../../shared/api/operator-errors";

const BRIDGE: BridgeIdentity = {
  auditPath: "/Users/o/Library/Application Support/Armada/audit.jsonl",
  fleetProtocol: "32a42be6",
};
const NOW = new Date("2026-11-06T14:00:00.000Z");
const TAKEN: OperatorErrorSent = {
  ok: true,
  value: {
    id: "error:abc:2026-11-06",
    repeated: false,
  },
};

const buildFailed = () =>
  buildFailedFailure(
    {
      build: "main",
      running: "main",
      commit: "8944d2fca1b2c3d4e5f6",
      line: "error[E0432]: unresolved import crate::drones",
    },
    BRIDGE,
  ).payload;

const refused = () =>
  refusalFailure(
    {
      code: "bridge.not_sent",
      message: "Nothing was sent",
      run_id: "",
      fields: {
        act: "start_job",
      },
      chain: [
        "no repository",
      ],
    },
    BRIDGE,
  ).payload;

let unwire = () => {};
afterEach(() => {
  unwire();
  unwire = () => {};
});

test("a card drawn sends one report with the card's facts", () => {
  const sent: ReportBridgeError[] = [];
  unwire = wireFailureReports(async (error) => {
    sent.push(error);
    return TAKEN;
  });
  failureShown(buildFailed());
  expect(sent).toHaveLength(1);
  expect(sent[0]).toMatchObject({
    code: "bridge.fleet.build_failed",
    message: "Main build failed: error[E0432]: unresolved import crate::drones",
    commit: "8944d2fca1b2c3d4e5f6",
    logs: [
      "~/Library/Logs/Armada/restart-build.log",
      "/Users/o/Library/Application Support/Armada/audit.jsonl",
    ],
  });
  expect(sent[0]?.fields).toContainEqual({
    key: "running",
    value: "main",
  });
  expect(Number.isNaN(Date.parse(sent[0]?.at ?? ""))).toBe(false);
});

test("the chain and the Job ride along, and an absent part is absent", () => {
  const error = reportedError(
    {
      ...refused(),
      job_id: "job-7",
    },
    NOW,
  );
  expect(error).toMatchObject({
    code: "bridge.not_sent",
    chain: [
      "no repository",
    ],
    job_id: "job-7",
    at: "2026-11-06T14:00:00.000Z",
  });
  expect(error).not.toHaveProperty("commit");
  const bare = reportedError(
    {
      code: "bridge.x",
      message: "m",
      bridgeProtocol: "1.0",
    },
    NOW,
  );
  expect(Object.keys(bare).sort()).toEqual([
    "at",
    "code",
    "message",
  ]);
});

test("the same card in quick succession is sent once, and again after the window", () => {
  let clock = NOW.getTime();
  const send = vi.fn(async (_error: ReportBridgeError) => TAKEN);
  const shown = failureReporter(send, () => new Date(clock));
  shown(refused());
  clock += 1_000;
  shown(refused());
  expect(send).toHaveBeenCalledTimes(1);
  shown({
    ...refused(),
    message: "Another thing was not sent",
  });
  expect(send).toHaveBeenCalledTimes(2);
  clock += SAME_CARD_MS;
  shown(refused());
  expect(send).toHaveBeenCalledTimes(3);
});

test("a send that fails raises nothing: no throw, and a rejection is taken here, not left to reach the window", async () => {
  const rejected = Promise.reject(new Error("Fleet went away"));
  const taken = vi.spyOn(rejected, "catch");
  const refusedSend: OperatorErrorSent = {
    ok: false,
    outcome: {
      ok: false,
      why: "not_connected",
    },
  };
  const rejects = failureReporter(
    () => rejected,
    () => NOW,
  );
  const throws = failureReporter(
    () => {
      throw new Error("no bridge");
    },
    () => NOW,
  );
  const refuses = failureReporter(
    async () => refusedSend,
    () => NOW,
  );
  unwire = wireFailureReports(() => {
    throw new Error("no bridge");
  });
  expect(() => rejects(refused())).not.toThrow();
  expect(() => throws(refused())).not.toThrow();
  expect(() => refuses(refused())).not.toThrow();
  expect(() => failureShown(buildFailed())).not.toThrow();
  expect(taken).toHaveBeenCalledTimes(1);
  await rejected.catch(() => {});
});
