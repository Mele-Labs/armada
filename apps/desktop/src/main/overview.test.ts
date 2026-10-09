// Overview's read, sent against a Fleet that records what arrived: health once.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import type { BridgeState } from "../shared/bridge";
import { OverviewReads } from "./overview";

const HEALTH = {
  probes: [{ module: "Fleet", outcome: "pass", detail: "answering" }],
  not_probed: [],
  helm_action_authority: "acting",
};

let listening: Server | null = null;
afterEach(async () => {
  const server = listening;
  listening = null;
  if (server !== null) await new Promise<void>((done) => server.close(() => done()));
});

async function fleet(asked: string[]): Promise<number> {
  const server = createServer((request, response) => {
    const url = request.url ?? "";
    asked.push(url);
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify(HEALTH));
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  return (server.address() as AddressInfo).port;
}

function reading(port: number | null) {
  let held: Partial<BridgeState> = {};
  const overview = new OverviewReads({ publish: (change) => (held = { ...held, ...change }), port: () => port });
  return { overview, held: () => held };
}

describe("Overview's reads", () => {
  it("ask nothing until a surface wants them", async () => {
    const asked: string[] = [];
    const port = await fleet(asked);
    const { overview, held } = reading(port);
    await overview.again(port);
    expect(asked).toEqual([]);
    expect(held()).toEqual({});
  });

  it("read health once", async () => {
    const asked: string[] = [];
    const port = await fleet(asked);
    const { overview, held } = reading(port);
    await overview.watch(true);
    expect(asked).toEqual(["/health"]);
    expect(held().health).toEqual({ state: "read", health: HEALTH });
  });

  it("drop what they held when the surface closes, and an answer that lands after", async () => {
    const asked: string[] = [];
    const port = await fleet(asked);
    const { overview, held } = reading(port);
    const opened = overview.watch(true);
    await overview.watch(false);
    await opened;
    expect(held()).toEqual({ health: { state: "none" } });
  });

  it("fail without a connection", async () => {
    const { overview, held } = reading(null);
    await overview.watch(true);
    expect(held().health).toEqual({ state: "failed", outcome: { ok: false, why: "not_connected" } });
  });
});
