// Copy debug report's reader: Fleet's text where it answers, `armada report` through main where it does not, Bridge's
// protocol on the last line either way, and the why where neither gives text.

import { describe, expect, it, vi } from "vitest";

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { createFleetDebugReport, withBridgeProtocol } from "./fleet-debug-report";
import type { StateSource } from "./state-feed";

const WITH_FLEET = "Armada debug report\n\nInstall\n  armada   0.14.2\n\nfleet protocol 5.2  taken 2026-11-02T09:31:12Z";
const SILENT = "Armada debug report\n\nInstall\n  armada   0.14.2\n\nFleet   did not answer\n\ntaken 2026-11-02T09:31:12Z";

const on = (repository: string | null): StateSource => ({
  state: async () => ({ repository }) as BridgeState,
  subscribe: () => () => undefined,
});

const unanswered = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

function api(reads: { fleet: Awaited<ReturnType<BridgeApi["getDebugReport"]>>; ran?: Awaited<ReturnType<BridgeApi["runArmadaReport"]>> }) {
  return {
    getDebugReport: vi.fn(async (_root: string | null) => reads.fleet),
    runArmadaReport: vi.fn(async (_root: string | null) => reads.ran ?? { ok: false as const, why: "unreached" }),
  };
}

describe("the last line", () => {
  it("gets bridge protocol once, ahead of fleet protocol", () => {
    const text = withBridgeProtocol(WITH_FLEET, "91bb07e4");
    expect(text.split("\n").at(-1)).toBe("bridge protocol 91bb07e4  fleet protocol 5.2  taken 2026-11-02T09:31:12Z");
    expect(text.match(/bridge protocol/g)).toHaveLength(1);
    expect(text.startsWith("Armada debug report\n\nInstall")).toBe(true);
  });

  it("gets it ahead of taken where Fleet was silent", () => {
    const text = withBridgeProtocol(`${SILENT}\n`, "91bb07e4");
    expect(text.split("\n").at(-1)).toBe("bridge protocol 91bb07e4  taken 2026-11-02T09:31:12Z");
    expect(text.match(/bridge protocol/g)).toHaveLength(1);
    expect(text.endsWith("\n")).toBe(false);
  });
});

describe("reading", () => {
  it("is Fleet's text for the selected repository, and runs nothing", async () => {
    const wire = api({ fleet: { ok: true, value: { text: WITH_FLEET } } });
    const read = createFleetDebugReport(wire, on("/Users/user/web-app"), "91bb07e4");
    await expect(read()).resolves.toBe(withBridgeProtocol(WITH_FLEET, "91bb07e4"));
    expect(wire.getDebugReport).toHaveBeenCalledWith("/Users/user/web-app");
    expect(wire.runArmadaReport).not.toHaveBeenCalled();
  });

  it("names no repository where All repositories is selected", async () => {
    const wire = api({ fleet: { ok: true, value: { text: WITH_FLEET } } });
    await createFleetDebugReport(wire, on(null), "91bb07e4")();
    expect(wire.getDebugReport).toHaveBeenCalledWith(null);
  });

  it("falls back to armada report, in the same repository, where Fleet's read fails", async () => {
    const wire = api({ fleet: unanswered, ran: { ok: true, text: SILENT } });
    const read = createFleetDebugReport(wire, on("/Users/user/web-app"), "91bb07e4");
    await expect(read()).resolves.toBe(withBridgeProtocol(SILENT, "91bb07e4"));
    expect(wire.runArmadaReport).toHaveBeenCalledWith("/Users/user/web-app");
  });

  it("rejects with why where both reads fail", async () => {
    const wire = api({ fleet: unanswered, ran: { ok: false, why: "no armada binary was found" } });
    await expect(createFleetDebugReport(wire, on(null), "91bb07e4")()).rejects.toThrow("no armada binary was found");
  });
});
