import { readFileSync } from "node:fs";
import { afterEach, describe, expect, it, vi } from "vitest";

const ORIGIN = "https://phone.test";
const SOURCE = readFileSync(new URL("../public/sw.js", import.meta.url), "utf8");
const PAGE = `${ORIGIN}/index.html`;

type Listener = (event: FakeEvent) => void;
type FakeEvent = { request: { url: string; method: string; mode: string }; answer?: Promise<Response>; waits: Promise<unknown>[]; respondWith(answer: Promise<Response>): void; waitUntil(wait: Promise<unknown>): void };

/** The worker's file, run against a stub `self`, an in-memory `caches` and a `fetch` the test scripts. */
function worker(fetchFor: (url: string) => Promise<Response>) {
  const listeners: Record<string, Listener> = {};
  const stored = new Map<string, Response>();
  const keyOf = (request: string | { url: string }) => new URL(typeof request === "string" ? request : request.url, ORIGIN).href;
  const cache = {
    match: async (request: string | { url: string }) => stored.get(keyOf(request))?.clone(),
    put: async (request: string | { url: string }, response: Response) => void stored.set(keyOf(request), response),
    keys: async () => [...stored.keys()].map((url) => ({ url })),
    delete: async (request: string | { url: string }) => stored.delete(keyOf(request)),
  };
  const fetched: string[] = [];
  const stubFetch = (request: { url: string }) => (fetched.push(request.url), fetchFor(request.url));
  const self = { addEventListener: (type: string, listener: Listener) => void (listeners[type] = listener), location: { origin: ORIGIN }, skipWaiting: () => {}, clients: {}, registration: {} };
  new Function("self", "caches", "fetch", SOURCE)(self, { open: async () => cache }, stubFetch);
  const ask = async (path: string, extra: { method?: string; mode?: string } = {}) => {
    const event: FakeEvent = { request: { url: ORIGIN + path, method: "GET", mode: "cors", ...extra }, waits: [], respondWith(answer) { this.answer = answer; }, waitUntil(wait) { this.waits.push(wait); } };
    listeners["fetch"]?.(event);
    return { event, answer: () => event.answer, answered: event.answer !== undefined, done: async () => (await Promise.all(event.waits), event.answer === undefined ? undefined : await event.answer) };
  };
  return { listeners, stored, fetched, ask };
}

const html = (body: string) => new Response(body, { status: 200, headers: { "content-type": "text/html" } });
const text = (response: Response | undefined) => response?.clone().text();

afterEach(() => vi.useRealTimers());

describe("the service worker", () => {
  it("keeps its push and click handlers", () => {
    expect(Object.keys(worker(async () => html("")).listeners)).toEqual(expect.arrayContaining(["push", "notificationclick"]));
  });

  it("answers a navigation with the cached page when the network fails", async () => {
    const sw = worker(async () => { throw new TypeError("offline"); });
    sw.stored.set(PAGE, html("<p>kept</p>"));
    const { answered, done } = await sw.ask("/jobs/_/b2", { mode: "navigate" });
    expect(answered).toBe(true);
    expect(await text(await done())).toBe("<p>kept</p>");
  });

  it("answers a navigation with the cached page when the network takes over 3 s", async () => {
    vi.useFakeTimers();
    const sw = worker(() => new Promise(() => {}));
    sw.stored.set(PAGE, html("<p>kept</p>"));
    const { answer } = await sw.ask("/", { mode: "navigate" });
    await vi.advanceTimersByTimeAsync(3000);
    expect(await text(await answer())).toBe("<p>kept</p>");
  });

  it("caches a page that arrives, under one key whatever the route", async () => {
    const sw = worker(async () => html("<p>fresh</p>"));
    const { done } = await sw.ask("/jobs/_/b2", { mode: "navigate" });
    expect(await text(await done())).toBe("<p>fresh</p>");
    expect([...sw.stored.keys()]).toEqual([PAGE]);
    expect(await text(sw.stored.get(PAGE))).toBe("<p>fresh</p>");
  });

  it("drops cached assets the fresh page does not reference", async () => {
    const sw = worker(async () => html('<script src="/assets/index-new.js"></script><link href="/assets/index-new.css">'));
    for (const name of ["index-old.js", "index-new.js"]) sw.stored.set(`${ORIGIN}/assets/${name}`, new Response("x"));
    const { done } = await sw.ask("/", { mode: "navigate" });
    await done();
    expect([...sw.stored.keys()].sort()).toEqual([PAGE, `${ORIGIN}/assets/index-new.js`].sort());
  });

  it("keeps the assets of the page it kept before, for a navigation answered with that page", async () => {
    const sw = worker(async () => html('<script src="/assets/index-new.js"></script>'));
    sw.stored.set(PAGE, html('<script src="/assets/index-old.js"></script>'));
    for (const name of ["index-older.js", "index-old.js"]) sw.stored.set(`${ORIGIN}/assets/${name}`, new Response("x"));
    const { done } = await sw.ask("/", { mode: "navigate" });
    await done();
    expect(await text(sw.stored.get(PAGE))).toBe('<script src="/assets/index-new.js"></script>');
    expect(sw.stored.has(`${ORIGIN}/assets/index-old.js`)).toBe(true);
    expect(sw.stored.has(`${ORIGIN}/assets/index-older.js`)).toBe(false);
  });

  it("answers an asset from the cache without fetching", async () => {
    const sw = worker(async () => { throw new Error("must not fetch"); });
    sw.stored.set(`${ORIGIN}/assets/index-a.js`, new Response("kept"));
    const { done } = await sw.ask("/assets/index-a.js");
    expect(await text(await done())).toBe("kept");
    expect(sw.fetched).toEqual([]);
  });

  it("fetches an asset it lacks and keeps it", async () => {
    const sw = worker(async () => new Response("built"));
    const { done } = await sw.ask("/assets/index-b.js");
    expect(await text(await done())).toBe("built");
    expect(await text(sw.stored.get(`${ORIGIN}/assets/index-b.js`))).toBe("built");
  });

  it("leaves the API, the admin routes, a POST and the worker itself alone", async () => {
    const sw = worker(async () => html(""));
    for (const path of ["/api/jobs", "/admin/status", "/sw.js", "/@vite/client"]) expect((await sw.ask(path)).answered, path).toBe(false);
    expect((await sw.ask("/api/x", { mode: "navigate" })).answered).toBe(false);
    expect((await sw.ask("/pair", { method: "POST" })).answered).toBe(false);
    expect(sw.fetched).toEqual([]);
  });
});
