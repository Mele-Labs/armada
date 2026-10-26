// What main asks Fleet for the agents' sign-in, and where each answer goes.

import { describe, expect, it, vi } from "vitest";

import type { AgentSignIn } from "@armada/protocol";

const ask = vi.fn();
vi.mock("./request", () => ({ ask: (...args: unknown[]) => ask(...args) }));

import { AgentSignInHost } from "./agent-sign-in";

const OUT: AgentSignIn = { signed_out_since: "2026-10-25T09:00:00Z" };

describe("the agents' sign-in, asked of Fleet", () => {
  it("reads with a GET and presses with a POST to the one route, sending no body", async () => {
    ask.mockResolvedValue({ ok: true, body: OUT });
    const host = new AgentSignInHost(() => 4100);
    await host.get();
    await host.start();
    expect(ask.mock.calls).toEqual([
      [4100, "GET", "/agent/sign_in"],
      [4100, "POST", "/agent/sign_in"],
    ]);
  });

  it("answers the state Fleet sent, and a refusal as the outcome Fleet gave", async () => {
    const host = new AgentSignInHost(() => 4100);
    ask.mockResolvedValueOnce({ ok: true, body: OUT });
    expect(await host.start()).toEqual({ ok: true, value: OUT });
    const outcome = { ok: false, why: "refused", error: { code: "fleet.no_sign_in" } };
    ask.mockResolvedValueOnce({ ok: false, outcome });
    expect(await host.start()).toEqual({ ok: false, outcome });
  });

  it("asks nothing with no connection", async () => {
    ask.mockClear();
    const host = new AgentSignInHost(() => null);
    expect(await host.start()).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
    expect(ask).not.toHaveBeenCalled();
  });

  it("tells a changed state to the windows, and one read on connecting to the notice alone", async () => {
    ask.mockResolvedValue({ ok: true, body: OUT });
    const host = new AgentSignInHost(() => 4100);
    const told = vi.fn();
    const seen = vi.fn();
    host.onChanged(told);
    host.onSeeded(seen);
    host.changed(OUT);
    expect(told).toHaveBeenCalledWith(OUT);
    expect(seen).not.toHaveBeenCalled();
    await host.seed();
    expect(seen).toHaveBeenCalledWith(OUT);
    expect(told).toHaveBeenCalledTimes(1);
  });
});
