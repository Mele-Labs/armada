import { afterEach, describe, expect, it } from "vitest";
import type { ReviewRequest } from "@armada/protocol";

import { addressOf, runnerPage, send, startRunner, type Runner } from "./runner";

const requests: ReviewRequest[] = [
  { say: "List invoices", method: "GET", path: "/invoices", look: "A list" },
  { say: "Create one", method: "POST", path: "/invoices", body: '{"total":10}', look: "201" },
];

describe("an address on the base", () => {
  it("is the base's origin and path with the request's path after it", () => {
    expect(addressOf("http://localhost:8080/api/", "/invoices?x=1")).toBe("http://localhost:8080/api/invoices?x=1");
    expect(addressOf("https://api.preview.invalid", "/a")).toBe("https://api.preview.invalid/a");
  });
  it("is none for a path that would leave the base or a base that is not the web", () => {
    expect(addressOf("http://localhost:8080", "//elsewhere.invalid/x")).toBeNull();
    expect(addressOf("http://localhost:8080", "invoices")).toBeNull();
    expect(addressOf("ftp://localhost", "/x")).toBeNull();
    expect(addressOf("not an address", "/x")).toBeNull();
  });
});

describe("the page", () => {
  it("has a button for each request named as its step presses it, and escapes what it draws", () => {
    const html = runnerPage("http://localhost:8080", [{ ...requests[0]!, path: "/a<b" }, requests[1]!], "t");
    expect(html).toContain('aria-label="Send 1: GET /a&lt;b"');
    expect(html).toContain('aria-label="Send 2: POST /invoices"');
    expect(html).toContain('aria-label="Request body"');
    expect(html).not.toContain("/a<b");
  });
});

describe("sending", () => {
  it("says what came back, with JSON indented", async () => {
    const through = (async () => new Response('{"a":1}', { status: 201 })) as typeof fetch;
    const sent = await send("http://localhost:8080", requests[1]!, through);
    expect(sent.status).toBe(201);
    expect(sent.body).toBe('{\n  "a": 1\n}');
  });
  it("says why there was no answer", async () => {
    const through = (async () => {
      throw new Error("connection refused");
    }) as typeof fetch;
    const sent = await send("http://localhost:8080", requests[0]!, through);
    expect(sent).toMatchObject({ status: 0, error: "connection refused" });
  });
});

describe("the server", () => {
  let runner: Runner | null = null;
  afterEach(async () => {
    await runner?.stop();
    runner = null;
  });

  it("serves the page and sends only the walk's own request, for a page that holds the token", async () => {
    const seen: string[] = [];
    const through = (async (address: string) => {
      seen.push(address);
      return new Response("{}", { status: 200 });
    }) as unknown as typeof fetch;
    runner = await startRunner("http://localhost:8080/api", requests, through);
    const page = await (await fetch(runner.url)).text();
    const token = /data-token="([0-9a-f]+)"/.exec(page)?.[1];
    expect(token).toBeDefined();

    const refused = await fetch(`${runner.url}send/0`, { method: "POST" });
    expect(refused.status).toBe(404);
    const sent = await fetch(`${runner.url}send/0`, { method: "POST", headers: { "x-armada-runner": token! } });
    expect(await sent.json()).toMatchObject({ status: 200 });
    expect(seen).toEqual(["http://localhost:8080/api/invoices"]);
    expect((await fetch(`${runner.url}send/9`, { method: "POST", headers: { "x-armada-runner": token! } })).status).toBe(404);
  });

  it("answers nothing to a request that names another host", async () => {
    runner = await startRunner("http://localhost:8080", requests, (async () => new Response("")) as unknown as typeof fetch);
    const port = new URL(runner.url).port;
    const { request } = await import("node:http");
    const status = await new Promise<number>((resolve) => {
      const req = request({ host: "127.0.0.1", port, path: "/", headers: { host: "rebind.invalid" } }, (res) => resolve(res.statusCode ?? 0));
      req.end();
    });
    expect(status).toBe(403);
  });
});
