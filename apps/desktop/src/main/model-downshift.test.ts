// What main asks Fleet for the default model's step down, and where each answer goes.

import { describe, expect, it, vi } from "vitest";

import type { ModelDownshiftState } from "@armada/protocol";

const ask = vi.fn();
vi.mock("./request", () => ({ ask: (...args: unknown[]) => ask(...args) }));

import { ModelDownshiftHost } from "./model-downshift";

const OFFERED: ModelDownshiftState = { downshift: { phase: "offered", percent: 83, was: "opus", to: "sonnet" } };

describe("the default model's step down, asked of Fleet", () => {
  it("names each route and sends no body", async () => {
    ask.mockResolvedValue({ ok: true, body: OFFERED });
    const host = new ModelDownshiftHost(() => 4100);
    await host.get();
    await host.accept();
    await host.restore();
    await host.keep();
    await host.settle();
    expect(ask.mock.calls).toEqual([
      [4100, "GET", "/model_downshift"],
      [4100, "POST", "/model_downshift/accept"],
      [4100, "POST", "/model_downshift/restore"],
      [4100, "POST", "/model_downshift/keep"],
      [4100, "POST", "/model_downshift/settle"],
    ]);
  });

  it("answers the state Fleet sent, and a refusal as the outcome Fleet gave", async () => {
    const host = new ModelDownshiftHost(() => 4100);
    ask.mockResolvedValueOnce({ ok: true, body: OFFERED });
    expect(await host.get()).toEqual({ ok: true, value: OFFERED });
    const outcome = { ok: false, why: "refused", error: { code: "fleet.downshift_wrong_phase" } };
    ask.mockResolvedValueOnce({ ok: false, outcome });
    expect(await host.accept()).toEqual({ ok: false, outcome });
  });

  it("asks nothing with no connection", async () => {
    ask.mockClear();
    const host = new ModelDownshiftHost(() => null);
    expect(await host.accept()).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
    expect(ask).not.toHaveBeenCalled();
  });

  it("tells a changed state to the windows, and one read on connecting to the notice alone", async () => {
    ask.mockResolvedValue({ ok: true, body: OFFERED });
    const host = new ModelDownshiftHost(() => 4100);
    const told = vi.fn();
    const seen = vi.fn();
    host.onChanged(told);
    host.onSeeded(seen);
    host.changed(OFFERED);
    expect(told).toHaveBeenCalledWith(OFFERED);
    expect(seen).not.toHaveBeenCalled();
    await host.seed();
    expect(seen).toHaveBeenCalledWith(OFFERED);
    expect(told).toHaveBeenCalledTimes(1);
  });
});
