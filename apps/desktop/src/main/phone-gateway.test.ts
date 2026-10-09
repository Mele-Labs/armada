// The Phone Gateway call: the routes it reaches, that it carries no `Origin`, and how each answer reads.

import { createServer, type IncomingHttpHeaders, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { afterEach, expect, test } from "vitest";

import { askGateway } from "./phone-gateway";

let server: Server | undefined;
afterEach(() => new Promise<void>((done) => (server === undefined ? done() : server.close(() => done()))));

type Seen = { method?: string; url?: string; headers: IncomingHttpHeaders; body: string };

/** A Gateway that answers every request with `status` and `text`, and records what it was sent. */
async function gateway(status: number, text: string): Promise<{ base: string; seen: Seen[] }> {
  const seen: Seen[] = [];
  server = createServer((request, response) => {
    let body = "";
    request.on("data", (chunk) => (body += chunk));
    request.on("end", () => {
      seen.push({ method: request.method, url: request.url, headers: request.headers, body });
      response.writeHead(status).end(text);
    });
  });
  await new Promise<void>((done) => server!.listen(0, "127.0.0.1", done));
  return { base: `http://127.0.0.1:${(server.address() as AddressInfo).port}`, seen };
}

test("a call carries no Origin, which the Gateway's admin routes refuse", async () => {
  const { base, seen } = await gateway(204, "");
  await askGateway({ op: "status" }, base);
  expect(seen[0]!.headers).not.toHaveProperty("origin");
});

test("each operation reaches its own route", async () => {
  const { base, seen } = await gateway(200, "[]");
  await askGateway({ op: "start" }, base);
  await askGateway({ op: "pending" }, base);
  await askGateway({ op: "confirm", code: "ab12" }, base);
  await askGateway({ op: "devices" }, base);
  await askGateway({ op: "unpair", id: "d 1" }, base);
  await askGateway({ op: "status" }, base);
  expect(seen.map((one) => `${one.method} ${one.url}`)).toEqual([
    "POST /admin/pair/start",
    "GET /admin/pair/pending",
    "POST /admin/pair/confirm",
    "GET /admin/devices",
    "DELETE /admin/devices/d%201",
    "GET /admin/status",
  ]);
  expect(JSON.parse(seen[2]!.body)).toEqual({ code: "ab12" });
});

test("a good answer is its JSON, and a 204 is null", async () => {
  const good = await gateway(200, '{"code":"c","address":"https://m.ts.net","expires_at":5}');
  expect(await askGateway({ op: "start" }, good.base)).toEqual({ ok: true, body: { code: "c", address: "https://m.ts.net", expires_at: 5 } });
  await new Promise<void>((done) => server!.close(() => done()));
  const none = await gateway(204, "");
  expect(await askGateway({ op: "status" }, none.base)).toEqual({ ok: true, body: null });
});

test("a refusal is the Gateway's sentence as it came", async () => {
  const { base } = await gateway(503, "Tailscale is not signed in on this Mac.\n");
  expect(await askGateway({ op: "status" }, base)).toEqual({ ok: false, why: "refused", said: "Tailscale is not signed in on this Mac." });
});

test("a refusal with no sentence names the status", async () => {
  const { base } = await gateway(502, "");
  expect(await askGateway({ op: "status" }, base)).toEqual({ ok: false, why: "refused", said: "The Gateway answered 502." });
});

test("a refused connection is the Gateway not running", async () => {
  const { base } = await gateway(204, "");
  await new Promise<void>((done) => server!.close(() => done()));
  server = undefined;
  expect(await askGateway({ op: "status" }, base)).toEqual({ ok: false, why: "unreachable" });
});
