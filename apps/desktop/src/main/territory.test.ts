// The territory main reads: each route answers for every served repository at once, and a Fleet
// without the route answers none rather than a fault.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import type { RepositoryAreas, SlotTouches } from "@armada/protocol";
import { TerritoryHost } from "./territory";

const AREAS: RepositoryAreas = {
  repositories: [{ manifest_id: "armada", root: "/r/armada", head: "abc", areas: [{ dir: "crates", files: 3, kids: [{ name: "fleet", files: 2 }] }], omitted: 0 }],
};
const TOUCHES: SlotTouches = { repositories: [{ manifest_id: "armada", holders: [{ kind: "job", id: "j1", slot: 1, paths: ["crates/fleet/a.rs"], omitted: 0 }] }] };

let listening: Server | undefined;
afterEach(() => listening?.close());

async function fleet(asked: string[], answer: (url: string) => { status: number; body: string }): Promise<number> {
  const server = createServer((request, response) => {
    asked.push(request.url ?? "");
    const { status, body } = answer(request.url ?? "");
    response.writeHead(status, { "content-type": "application/json" });
    response.end(body);
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  return (server.address() as AddressInfo).port;
}

describe("reading the territory", () => {
  it("asks each route once, naming no repository, and hands back what Fleet said", async () => {
    const asked: string[] = [];
    const port = await fleet(asked, (url) => ({ status: 200, body: JSON.stringify(url === "/repository_areas" ? AREAS : TOUCHES) }));
    const host = new TerritoryHost(() => port);

    expect(await host.areas()).toEqual({ ok: true, value: AREAS });
    expect(await host.touches()).toEqual({ ok: true, value: TOUCHES });
    expect(asked).toEqual(["/repository_areas", "/slot_touches"]);
  });

  it("treats a Fleet that has no such route as having none, and says nothing", async () => {
    const port = await fleet([], () => ({ status: 404, body: "" }));
    const host = new TerritoryHost(() => port);

    expect(await host.areas()).toEqual({ ok: true, value: { repositories: [] } });
    expect(await host.touches()).toEqual({ ok: true, value: { repositories: [] } });
  });

  it("reports a refusal that is not a missing route, so the renderer keeps what stands", async () => {
    const port = await fleet([], () => ({ status: 500, body: "boom" }));
    const host = new TerritoryHost(() => port);

    expect((await host.areas()).ok).toBe(false);
    expect((await host.touches()).ok).toBe(false);
  });

  it("is not connected without a port", async () => {
    const host = new TerritoryHost(() => null);
    const none = { ok: false, outcome: { ok: false, why: "not_connected" } };

    expect(await host.areas()).toEqual(none);
    expect(await host.touches()).toEqual(none);
  });
});
