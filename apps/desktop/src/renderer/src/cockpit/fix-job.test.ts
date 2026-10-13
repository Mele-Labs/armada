// The fixes a Job's call card offers, against a fake preload: Message goes to one Drone by its id and
// starts the step again with the words where none is live, Retry step is `restart_step`, and the Check
// fixes (which main has no route for) are drawn only where a window registers them.

import { afterEach, expect, it, vi } from "vitest";

import type { Outcome } from "@armada/protocol";

import { checkFixesOf, fixJob, registerCheckFixes } from "./fix-job";

const OK: Outcome = { ok: true };
const refusedWith = (code: string): Outcome => ({ ok: false, why: "refused", error: { code, message: `refused: ${code}`, run_id: "", fields: {}, chain: [] } });

function preload(redirect: Outcome = OK, restart: Outcome = OK) {
  const armada = { redirectDrone: vi.fn(async () => redirect), restartStep: vi.fn(async () => restart) };
  vi.stubGlobal("window", { armada });
  return armada;
}
afterEach(() => vi.unstubAllGlobals());

it("tells the live Drone the words by its id, and does not restart anything", async () => {
  const armada = preload();
  expect(await fixJob({ kind: "redirect_one_drone", jobId: "j1", words: "Use the counter", droneId: "d7" })).toEqual({ ok: true });
  expect(armada.redirectDrone).toHaveBeenCalledWith("j1", "Use the counter", "d7");
  expect(armada.restartStep).not.toHaveBeenCalled();
});

it.each(["fleet.drone_not_live", "fleet.illegal_move"])("starts the step again with the words where Fleet says no Drone is live (%s)", async (code) => {
  const armada = preload(refusedWith(code));
  expect(await fixJob({ kind: "redirect_one_drone", jobId: "j1", words: "Use the counter" })).toEqual({ ok: true });
  expect(armada.restartStep).toHaveBeenCalledWith("j1", "Use the counter");
});

it("says Fleet's words for any other refusal, and restarts nothing", async () => {
  const armada = preload(refusedWith("fleet.fault"));
  expect(await fixJob({ kind: "redirect_one_drone", jobId: "j1", words: "x", droneId: "d7" })).toEqual({ ok: false, said: "refused: fleet.fault" });
  expect(armada.restartStep).not.toHaveBeenCalled();
});

it("retries the step with no note, and with the words where there are some", async () => {
  const armada = preload();
  await fixJob({ kind: "restart_step", jobId: "j1" });
  expect(armada.restartStep).toHaveBeenLastCalledWith("j1", undefined);
  await fixJob({ kind: "restart_step", jobId: "j1", words: "Start from the failing case" });
  expect(armada.restartStep).toHaveBeenLastCalledWith("j1", "Start from the failing case");
});

it("has no Check fixes on a preload that registered none, and refuses if one is asked for anyway", async () => {
  const armada = preload();
  expect(checkFixesOf(armada)).toBeUndefined();
  expect(await fixJob({ kind: "retry_check", jobId: "j1", check: "store" })).toEqual({ ok: false, said: "Fleet cannot do that yet" });
});

it("runs a Check fix where one was registered", async () => {
  const armada = preload();
  const fixes = { retryCheck: vi.fn(async () => OK), skipCheck: vi.fn(async () => OK), skipChecks: vi.fn(async () => OK) };
  registerCheckFixes(armada, fixes);
  await fixJob({ kind: "skip_check", jobId: "j1", check: "store" });
  expect(fixes.skipCheck).toHaveBeenCalledWith("j1", "store");
  await fixJob({ kind: "skip_checks", jobId: "j1" });
  expect(fixes.skipChecks).toHaveBeenCalledWith("j1");
});
