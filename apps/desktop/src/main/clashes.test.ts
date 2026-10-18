// The write clashes main reads: one request per served Manifest, merged, and a Fleet before 23.81
// (which has no `/clashes`) answering none rather than a fault.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import type { WriteClash } from "@armada/protocol";
import { ClashesHost } from "./clashes";

const clash = (one: string, other: string): WriteClash => ({
  one: { kind: "session", id: one, label: `Session ${one}` },
  other: { kind: "session", id: other, label: `Session ${other}` },
  paths: ["src/a.rs"],
  since: "2026-10-13T09:00:00Z",
});

let listening: Server | undefined;
afterEach(() => listening?.close());

/** A Fleet that answers each request by what `answer` says of its URL. */
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

const clashesIn = (...of: WriteClash[]) => ({ status: 200, body: JSON.stringify({ clashes: of }) });

describe("reading the clashes", () => {
  it("asks for each served Manifest by name, and merges what they answer", async () => {
    const asked: string[] = [];
    const port = await fleet(asked, (url) => (url.includes("manifest_id=armada") ? clashesIn(clash("s0", "s1")) : clashesIn(clash("s3", "s2"))));
    const read = await new ClashesHost(() => port, () => ["armada", "store front"]).get();
    expect(asked.sort()).toEqual(["/clashes?manifest_id=armada", "/clashes?manifest_id=store%20front"]);
    expect(read).toEqual({ ok: true, value: [clash("s0", "s1"), clash("s3", "s2")] });
  });

  it("asks bare before Fleet has listed any repository, as it always did", async () => {
    const asked: string[] = [];
    const port = await fleet(asked, () => clashesIn());
    expect(await new ClashesHost(() => port, () => null).get()).toEqual({ ok: true, value: [] });
    expect(asked).toEqual(["/clashes"]);
  });

  it("asks nothing where no listed repository has a Manifest", async () => {
    const asked: string[] = [];
    const port = await fleet(asked, () => clashesIn());
    expect(await new ClashesHost(() => port, () => []).get()).toEqual({ ok: true, value: [] });
    expect(asked).toEqual([]);
  });

  it("treats a Fleet that has no such route as having none, and says nothing", async () => {
    const port = await fleet([], () => ({ status: 404, body: "" }));
    expect(await new ClashesHost(() => port, () => ["armada"]).get()).toEqual({ ok: true, value: [] });
  });

  it("keeps the clashes of a Manifest that answered when another's route is missing", async () => {
    const port = await fleet([], (url) => (url.includes("armada") ? clashesIn(clash("s0", "s1")) : { status: 404, body: "" }));
    expect(await new ClashesHost(() => port, () => ["armada", "other"]).get()).toEqual({ ok: true, value: [clash("s0", "s1")] });
  });

  it("reports a refusal that is not a missing route, so the renderer keeps what stands", async () => {
    const port = await fleet([], () => ({ status: 500, body: "boom" }));
    const read = await new ClashesHost(() => port, () => ["armada"]).get();
    expect(read.ok).toBe(false);
  });

  it("is not connected without a port", async () => {
    expect(await new ClashesHost(() => null, () => ["armada"]).get()).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});

describe("reading one file of a clash", () => {
  const patches = { ours: "diff --git a/src/a.rs b/src/a.rs\n@@ -1 +1 @@\n-a\n+b\n", theirs: "" };

  it("names the Session, the other side and the path, encoded, and no Manifest", async () => {
    const asked: string[] = [];
    const port = await fleet(asked, () => ({ status: 200, body: JSON.stringify(patches) }));
    const read = await new ClashesHost(() => port, () => ["armada"]).file("s0", { kind: "session", id: "s1" }, "src/[id]/a & b#.rs");
    expect(read).toEqual({ ok: true, value: patches });
    const url = new URL(asked[0] ?? "", "http://fleet");
    expect(url.pathname).toBe("/clashes/file");
    expect(Object.fromEntries(url.searchParams)).toEqual({ session_id: "s0", other_kind: "session", other_id: "s1", path: "src/[id]/a & b#.rs" });
  });

  it("answers a refusal as a refusal, so the sheet can say why", async () => {
    const error = { code: "fleet.clash_not_standing", message: "the clash over `src/a.rs` no longer stands", run_id: "r", fields: {}, chain: [] };
    const port = await fleet([], () => ({ status: 409, body: JSON.stringify(error) }));
    const read = await new ClashesHost(() => port, () => ["armada"]).file("s0", { kind: "main", id: "main" }, "src/a.rs");
    expect(read).toMatchObject({ ok: false, outcome: { why: "refused", error: { code: "fleet.clash_not_standing" } } });
  });

  it("reports a Fleet that has no such route as a transport 404, which Bridge words", async () => {
    const port = await fleet([], () => ({ status: 404, body: "" }));
    const read = await new ClashesHost(() => port, () => ["armada"]).file("s0", { kind: "session", id: "s1" }, "src/a.rs");
    expect(read).toMatchObject({ ok: false, outcome: { why: "transport", fault: { why: "unanswerable", status: 404 } } });
  });

  it("is not connected without a port", async () => {
    expect(await new ClashesHost(() => null, () => []).file("s0", { kind: "main", id: "main" }, "a")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });
});
