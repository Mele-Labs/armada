// The mods Bridge reads and the three things it does to one, against a Fleet that answers each
// route: what is held, what is folded from an answer, and that a mod Fleet cannot check is `null`.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { ModList, ModSummary } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { Modding } from "./mods";

const row = (name: string, over: Partial<ModSummary> = {}): ModSummary => ({ name, kind: "theme", enabled: true, valid: true, ...over });

const servers: Server[] = [];
afterEach(() => {
  while (servers.length > 0) servers.pop()?.close();
});

/** A Fleet answering `routes` by method and path, and recording each request body it was sent. */
async function fleet(routes: Record<string, (body: unknown) => [number, unknown]>) {
  const sent: unknown[] = [];
  const server = createServer((request, answer) => {
    const chunks: Buffer[] = [];
    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("end", () => {
      const text = Buffer.concat(chunks).toString();
      const body: unknown = text === "" ? undefined : JSON.parse(text);
      sent.push(body);
      const [status, reply] = (routes[`${request.method} ${request.url}`] ?? (() => [404, { message: "no such route" }]))(body);
      answer.writeHead(status, { "content-type": "application/json" }).end(JSON.stringify(reply));
    });
  });
  servers.push(server);
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
  return { port: (server.address() as AddressInfo).port, sent };
}

function modding(port: number | null) {
  let published: Partial<BridgeState> = {};
  const mods = new Modding((change) => (published = { ...published, ...change }), () => port);
  return { mods, published: () => published.mods };
}

it("reads the list once and publishes it; a failed read keeps what was held", async () => {
  const list: ModList = { mods: [row("calm")] };
  const served = await fleet({ "GET /mods": () => [200, list] });
  const { mods, published } = modding(served.port);
  expect(published()).toBeUndefined();
  await mods.read(served.port);
  expect(published()).toEqual(list);

  const down = await fleet({});
  await mods.read(down.port);
  expect(published()).toEqual(list);
});

it("asks Fleet to check a mod by name and hands back exactly what it said", async () => {
  const checked = { name: "calm", valid: true, problems: [], css: ":root { --bg-base: navy; }" };
  const served = await fleet({
    "GET /mods/validate?name=calm": () => [200, checked],
    "GET /mods/validate?name=gone": () => [404, { code: "fleet.no_such_mod", message: "no such mod" }],
  });
  const { mods } = modding(served.port);
  expect(await mods.validate("calm")).toEqual(checked);
  expect(await mods.validate("gone")).toBeNull();
  expect(await modding(null).mods.validate("calm")).toBeNull();
});

it("folds the row a switch answered into the list, without waiting for mods.changed", async () => {
  const served = await fleet({
    "GET /mods": () => [200, { mods: [row("calm"), row("warm")] }],
    "POST /mods/enable": () => [200, row("warm", { enabled: false })],
  });
  const { mods, published } = modding(served.port);
  await mods.read(served.port);
  expect(await mods.setEnabled("warm", false)).toEqual({ ok: true });
  expect(served.sent.at(-1)).toEqual({ name: "warm", enabled: false });
  expect(published()?.mods).toEqual([row("calm"), row("warm", { enabled: false })]);
});

it("answers a promotion with the branch Fleet named, and a refusal as itself", async () => {
  const promoted = { name: "calm", branch: "armada/mod-calm-1", commit: "abc123" };
  const served = await fleet({
    "POST /mods/promote": (body) => ((body as { name: string }).name === "calm" ? [200, promoted] : [409, { code: "fleet.mod_not_promotable", message: "it is invalid" }]),
  });
  const { mods } = modding(served.port);
  expect(await mods.promote("calm")).toEqual({ ok: true, modPromoted: promoted });
  const refused = await mods.promote("broken");
  expect(refused.ok).toBe(false);
  expect(await modding(null).mods.promote("calm")).toEqual({ ok: false, why: "not_connected" });
});
