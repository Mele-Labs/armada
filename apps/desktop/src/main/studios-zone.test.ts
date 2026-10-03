// A node put down inside a Zone, read off a real listener — the owner, 2 Oct 2026: "Pressing
// inside a Zone places the armed kind there and puts it in that Zone."
//
// **What crosses the wire is under test**: one `add_node` naming the Zone, with the spot measured
// from its corner, and nothing after it.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { Studio, StudioNode } from "@armada/protocol";
import type { StudioRead } from "@armada/screens/src/studio-reads";
import { StudioReads } from "./studios";

let listening: Server | null = null;

afterEach(async () => {
  const server = listening;
  listening = null;
  if (server === null) return;
  await new Promise<void>((done) => server.close(() => done()));
});

const STUDIO = "01STUDIO";

function studio(): Studio {
  return {
    id: STUDIO,
    manifest_id: "01MANIFEST",
    created_at: "2026-10-02T09:00:00Z",
    touched_at: "2026-10-02T09:00:00Z",
    nodes: [{ id: "zone-1", kind: "zone", position: { x: 400, y: 200 }, created_at: "2026-10-02T09:00:00Z", added_by: "person" }],
    edges: [],
  };
}

type Sent = { path: string; body: Record<string, unknown> };

/** A Fleet that keeps one Studio and adds nodes to it, and refuses an add into a frame when told to. */
async function fleet(sent: Sent[], refuseFrame = false): Promise<number> {
  const held = studio();
  const server = createServer((request, response) => {
    let raw = "";
    request.on("data", (chunk) => (raw += String(chunk)));
    request.on("end", () => {
      const path = request.url ?? "";
      const body = raw === "" ? {} : (JSON.parse(raw) as Record<string, unknown>);
      sent.push({ path, body });
      if (path.endsWith("/add_node")) {
        if (refuseFrame && body.within !== undefined) {
          response.writeHead(422, { "content-type": "application/json" });
          response.end(JSON.stringify({ code: "fleet.studio_frame_cannot_hold", message: "that frame takes no such node" }));
          return;
        }
        const { position, ...node } = body as { position: { x: number; y: number } };
        held.nodes.push({ ...(node as object), id: `node-${held.nodes.length}`, position, created_at: "2026-10-02T09:01:00Z", added_by: "person" } as StudioNode);
      }
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(held));
    });
  });
  listening = server;
  await new Promise<void>((up) => server.listen(0, "127.0.0.1", up));
  return (server.address() as AddressInfo).port;
}

async function opened(port: number): Promise<{ reads: StudioReads; shown: () => Studio | null }> {
  let last: StudioRead | undefined;
  const reads = new StudioReads((change) => void (change.studio === undefined ? null : (last = change.studio)), () => port);
  await reads.watchStudio(STUDIO);
  return { reads, shown: () => (last?.state === "read" ? last.studio : null) };
}

it("a node put down in a Zone is one write, naming the Zone and its spot in it", async () => {
  const sent: Sent[] = [];
  const { reads, shown } = await opened(await fleet(sent));

  const outcome = await reads.addNode(STUDIO, { kind: "note", said: "Inside" }, { x: 40, y: 60 }, "zone-1");

  expect(outcome).toEqual({ ok: true });
  const writes = sent.filter((one) => one.path.startsWith(`/studios/${STUDIO}/`));
  expect(writes).toEqual([
    { path: `/studios/${STUDIO}/add_node`, body: { kind: "note", said: "Inside", within: "zone-1", position: { x: 40, y: 60 } } },
  ]);
  expect(shown()?.nodes.find((one) => one.id === "node-1")).toMatchObject({ within: "zone-1", position: { x: 40, y: 60 } });
});

it("a node put down in a Zone that Fleet refuses is not added, and says why", async () => {
  const sent: Sent[] = [];
  const { reads, shown } = await opened(await fleet(sent, true));

  const outcome = await reads.addNode(STUDIO, { kind: "zone" }, { x: 40, y: 60 }, "zone-1");

  expect(outcome.ok).toBe(false);
  expect(shown()?.nodes.map((one) => one.id)).toEqual(["zone-1"]);
});

it("a node put down on the board is one write", async () => {
  const sent: Sent[] = [];
  const { reads } = await opened(await fleet(sent));

  await reads.addNode(STUDIO, { kind: "zone" }, { x: 10, y: 20 }, null);

  expect(sent.filter((one) => one.path.startsWith(`/studios/${STUDIO}/`)).map((one) => one.body)).toEqual([
    { kind: "zone", position: { x: 10, y: 20 } },
  ]);
});
