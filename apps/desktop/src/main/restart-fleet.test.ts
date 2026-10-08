// Restart Fleet asks launchd, and only for the job that holds the Fleet the
// runtime file names. `launchctl` is a fake that records what it was asked.

import { describe, expect, it } from "vitest";

import type { Connection } from "@armada/protocol";

import { restartFleet, type Ran, type Run } from "./restart-fleet";

const FLEET = { protocolId: "3fa9c1d20b7e4a15", pid: 61372, port: 40000, startedAt: "" };
const APART: Connection = { state: "protocol_mismatch", fleet: FLEET, speaks: FLEET.protocolId, expected: "91bb07e4c25d8830" };

function launchd(answers: Record<string, Ran>): { run: Run; asked: string[][] } {
  const asked: string[][] = [];
  return {
    asked,
    run: async (args) => {
      asked.push(args);
      return answers[args[0] ?? ""] ?? { code: 0, stdout: "", stderr: "" };
    },
  };
}

const HOLDS = (pid: number): Ran => ({ code: 0, stdout: `com.armada.fleet = {\n\tpid = ${pid}\n}`, stderr: "" });

describe("restarting Fleet from the window", () => {
  it("kickstarts the job that holds the Fleet the runtime file names", async () => {
    const { run, asked } = launchd({ print: HOLDS(61372) });
    await expect(restartFleet(APART, run, {}, 501)).resolves.toEqual({ ok: true });
    expect(asked).toEqual([
      ["print", "gui/501/com.armada.fleet"],
      ["kickstart", "-k", "gui/501/com.armada.fleet"],
    ]);
  });

  it("uses the label the restart script uses", async () => {
    const { run, asked } = launchd({ print: HOLDS(61372) });
    await restartFleet(APART, run, { ARMADA_FLEET_LABEL: "com.armada.dev" }, 501);
    expect(asked[1]).toEqual(["kickstart", "-k", "gui/501/com.armada.dev"]);
  });

  it("does not restart a Fleet launchd does not hold", async () => {
    const { run, asked } = launchd({ print: HOLDS(1) });
    const answer = await restartFleet(APART, run, {}, 501);
    expect(answer).toMatchObject({ ok: false, why: "not_started_by_armada" });
    expect(asked).toHaveLength(1);
  });

  it("does not restart where no job is loaded", async () => {
    const { run } = launchd({ print: { code: 113, stdout: "", stderr: "Could not find service" } });
    await expect(restartFleet(APART, run, {}, 501)).resolves.toMatchObject({ ok: false, why: "not_started_by_armada" });
  });

  it("says what launchd refused with", async () => {
    const { run } = launchd({ print: HOLDS(61372), kickstart: { code: 1, stdout: "", stderr: "Operation not permitted\n" } });
    await expect(restartFleet(APART, run, {}, 501)).resolves.toEqual({ ok: false, why: "refused", detail: "Operation not permitted" });
  });

  it("has nothing to restart where the runtime file names no Fleet", async () => {
    const { run, asked } = launchd({});
    await expect(restartFleet({ state: "reading" }, run, {}, 501)).resolves.toMatchObject({ ok: false, why: "not_running" });
    expect(asked).toEqual([]);
  });
});
