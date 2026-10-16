// The bay history main reads: one request naming the repository and the slot, and a renderer that is
// not trusted to send either as anything else.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import type { BayHistory } from "@armada/protocol";
import { BayHistoryHost, slotHistoryPath } from "./bay-history";
import { bayOf } from "./bay-history-channels";

const HISTORY: BayHistory = {
  manifest_id: "armada",
  slot: 10,
  entries: [
    { kind: "checkout", at: "2026-10-15T10:10:00.000Z", from: "bug-fix", to: "main", sha: "a".repeat(40) },
    { kind: "leased", at: "2026-10-15T10:00:00.000Z", holder: { kind: "agent", id: "sess-1", title: "Checks --changed Bug" }, branch: "bug-fix" },
  ],
};

let listening: Server | undefined;
afterEach(() => listening?.close());

async function fleet(asked: string[], status: number, body: string): Promise<number> {
  const server = createServer((request, response) => {
    asked.push(request.url ?? "");
    response.writeHead(status, { "content-type": "application/json" });
    response.end(body);
  });
  listening = server;
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  return (server.address() as AddressInfo).port;
}

describe("reading a bay's history", () => {
  it("names the repository and the slot, encoded, and hands back what Fleet said", async () => {
    const asked: string[] = [];
    const port = await fleet(asked, 200, JSON.stringify(HISTORY));
    const host = new BayHistoryHost(() => port);

    expect(await host.get("armada", 10)).toEqual({ ok: true, value: HISTORY });
    expect(asked).toEqual(["/slot_history?manifest_id=armada&slot=10"]);
    expect(slotHistoryPath("a b/c", 2)).toBe("/slot_history?manifest_id=a%20b%2Fc&slot=2");
  });

  it("reports a refusal so the renderer keeps what stands", async () => {
    const port = await fleet([], 404, "");
    expect((await new BayHistoryHost(() => port).get("armada", 10)).ok).toBe(false);
  });

  it("says it is not connected where there is no Fleet", async () => {
    expect(await new BayHistoryHost(() => null).get("armada", 10)).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});

describe("what crosses from the renderer", () => {
  it("is a repository and a whole slot number, and nothing else", () => {
    expect(bayOf("armada", 10)).toEqual({ manifestId: "armada", slot: 10 });
    for (const [manifest, slot] of [["", 1], [3, 1], ["armada", "1"], ["armada", 1.5], ["armada", -1], [undefined, undefined]]) {
      expect(bayOf(manifest, slot)).toBeNull();
    }
  });
});
