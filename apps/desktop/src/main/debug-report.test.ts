// `armada report` run for Copy debug report: the binary launchd starts Fleet with, no shell, a bound on the wait, and a
// sentence for each way it can fail. The spawn and launchctl are fakes that record what they were asked.

import { describe, expect, it, vi } from "vitest";

import { CHANNELS } from "../shared/bridge";
import { armadaReport, debugReportPath, locateArmada, REPORT_MS, rootOf, type Exec } from "./debug-report";
import { handleDebugReport } from "./debug-report-channels";
import type { Run } from "./restart-fleet";

const PROGRAM = "/Users/ada/.cargo/bin/armada";
const PRINTED = `com.armada.fleet = {\n\tprogram = ${PROGRAM}\n\tpid = 41\n}`;
const launchd = (stdout: string, code = 0): Run => vi.fn(async () => ({ code, stdout, stderr: "" }));

describe("locating the binary", () => {
  it("is the program launchd starts Fleet with", async () => {
    const asked = launchd(PRINTED);
    await expect(locateArmada(null, asked, {}, 501, async () => true)).resolves.toBe(PROGRAM);
    expect(asked).toHaveBeenCalledWith(["print", "gui/501/com.armada.fleet"]);
  });

  it("uses the label the restart script uses", async () => {
    const asked = launchd(PRINTED);
    await locateArmada(null, asked, { ARMADA_FLEET_LABEL: "com.armada.dev" }, 501, async () => true);
    expect(asked).toHaveBeenCalledWith(["print", "gui/501/com.armada.dev"]);
  });

  it("is the one inside the app where launchd holds no job", async () => {
    const bundled = "/Applications/Armada.app/Contents/Resources/bin/armada";
    await expect(locateArmada(bundled, launchd("", 113), {}, 501, async (path) => path === bundled)).resolves.toBe(bundled);
  });

  it("is none where nothing found runs, and never a bare armada", async () => {
    await expect(locateArmada(null, launchd(PRINTED), {}, 501, async () => false)).resolves.toBeNull();
    await expect(locateArmada(null, launchd("", 113), {}, 501, async () => true)).resolves.toBeNull();
  });
});

describe("running it", () => {
  it("runs the located binary with report, in the folder given, and returns what it printed", async () => {
    const run = vi.fn<Exec>(async () => ({ stdout: "Armada debug report\n\ntaken 2026-11-02T09:31:12Z\n" }));
    await expect(armadaReport(PROGRAM, "/Users/ada/web-app", run)).resolves.toBe("Armada debug report\n\ntaken 2026-11-02T09:31:12Z");
    expect(run).toHaveBeenCalledWith(PROGRAM, ["report"], { cwd: "/Users/ada/web-app", timeout: 15_000 });
    expect(REPORT_MS).toBe(15_000);
  });

  it("rejects where the run is killed for taking too long", async () => {
    const run: Exec = async () => Promise.reject(Object.assign(new Error("Command failed"), { killed: true, signal: "SIGTERM" }));
    await expect(armadaReport(PROGRAM, "/tmp", run)).rejects.toThrow("armada report did not finish in 15 seconds");
  });

  it("rejects with the first line of what a failed run said", async () => {
    const run: Exec = async () => Promise.reject(Object.assign(new Error("Command failed"), { stderr: "error: no home\nmore" }));
    await expect(armadaReport(PROGRAM, "/tmp", run)).rejects.toThrow("armada report failed: error: no home");
  });

  it("rejects without running anything where there is no binary", async () => {
    const run = vi.fn<Exec>();
    await expect(armadaReport(null, "/tmp", run)).rejects.toThrow("no armada binary was found");
    expect(run).not.toHaveBeenCalled();
  });

  it("rejects where it printed nothing", async () => {
    await expect(armadaReport(PROGRAM, "/tmp", async () => ({ stdout: "\n" }))).rejects.toThrow("armada report printed nothing");
  });
});

describe("what crosses from the renderer", () => {
  it("is a root only where it is an absolute path", () => {
    expect(rootOf("/Users/ada/web-app")).toBe("/Users/ada/web-app");
    for (const bad of ["web-app", "", null, undefined, 3, {}]) expect(rootOf(bad)).toBeNull();
  });

  it("is asked of Fleet with the root encoded, or without one for All", () => {
    expect(debugReportPath("/Users/ada/my app")).toBe("/debug_report?root=%2FUsers%2Fada%2Fmy%20app");
    expect(debugReportPath(null)).toBe("/debug_report");
  });
});

describe("the channel", () => {
  const handlers = new Map<string, (event: unknown, root: unknown) => unknown>();
  const ipc = { handle: (channel: string, handler: (event: unknown, root: unknown) => unknown) => void handlers.set(channel, handler) };
  const call = (root: unknown) => handlers.get(CHANNELS.runArmadaReport)!(null, root);

  it("answers the text, run in the repository's folder", async () => {
    const run = vi.fn<Exec>(async () => ({ stdout: "text\n" }));
    handleDebugReport({ ipc: ipc as never, connection: () => null, bundled: null, locate: async () => PROGRAM, run });
    await expect(call("/Users/ada/web-app")).resolves.toEqual({ ok: true, text: "text" });
    expect(run).toHaveBeenCalledWith(PROGRAM, ["report"], { cwd: "/Users/ada/web-app", timeout: 15_000 });
  });

  it("answers why where it could not run, and reads Fleet as not connected with no connection", async () => {
    handleDebugReport({ ipc: ipc as never, connection: () => null, bundled: null, locate: async () => null });
    await expect(call(null)).resolves.toEqual({ ok: false, why: "no armada binary was found" });
    await expect(handlers.get(CHANNELS.getDebugReport)!(null, "/Users/ada/web-app")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});
