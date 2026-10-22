// @vitest-environment happy-dom
// Pocket v2's claim, with two fake Macs: one phone app, paired once per Mac, a combined view labelled
// by Mac; a Mac that forgets the phone is that Mac's trouble alone; a Job opens to its thread and is
// answered on the Mac that holds it; a push opens the Job it is about.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { answer, type Posted } from "./fixture/gateway";
import { fixtureMacs, type FixtureMac } from "./fixture/wire";
import { signedMessage } from "./signing";

const STUDIO = "https://studio.tail0.ts.net";
const LAPTOP = "https://laptop.tail0.ts.net";
const NOW = Date.now();

type Key = { deviceId: string; pair: CryptoKeyPair };
type Sent = { origin: string; method: string; target: string; body: string; device: string; time: string; signature: string };

let macs: Record<string, FixtureMac>;
let keys: Record<string, Key>;
let posted: Record<string, Posted[]>;
let sent: Sent[];
let forgotten: Set<string>;
let root: Root | undefined;
let host: HTMLElement;

const NOT_PAIRED = "This phone is not paired with this Mac. Pair it again from Bridge, Settings, Phone.";

async function world() {
  vi.resetModules();
  macs = fixtureMacs(NOW, { studio: STUDIO, laptop: LAPTOP });
  posted = { [STUDIO]: [], [LAPTOP]: [] };
  sent = [];
  forgotten = new Set();
  const make = () => crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, false, ["sign", "verify"]) as Promise<CryptoKeyPair>;
  keys = { [STUDIO]: { deviceId: "phone-on-studio", pair: await make() }, [LAPTOP]: { deviceId: "phone-on-laptop", pair: await make() } };
  const held = [STUDIO, LAPTOP].map((origin) => ({ origin, name: macs[origin]!.name, deviceId: keys[origin]!.deviceId, key: keys[origin]!.pair.privateKey }));
  vi.doMock("./machines", () => ({
    machines: async () => held,
    keepMachine: async () => undefined,
    forgetMachine: async () => undefined,
  }));
  vi.stubGlobal("fetch", async (url: string, init: { method?: string; body?: string; headers?: Record<string, string> } = {}) => {
    const at = new URL(url);
    const method = init.method ?? "GET";
    const headers = init.headers ?? {};
    const one: Sent = {
      origin: at.origin, method, target: at.pathname + at.search, body: init.body ?? "",
      device: headers["X-Pocket-Device"] ?? "", time: headers["X-Pocket-Time"] ?? "", signature: headers["X-Pocket-Signature"] ?? "",
    };
    sent.push(one);
    if (forgotten.has(at.origin)) return new Response(NOT_PAIRED, { status: 401 });
    if (at.pathname === "/api/live") return new Response(new ReadableStream<Uint8Array>({ start: () => undefined }), { headers: { "Content-Type": "text/event-stream" } });
    const mac = macs[at.origin];
    if (mac === undefined) throw new TypeError("Failed to fetch");
    const said = answer(mac, posted[at.origin]!, method, one.target, one.body, /^[0-9a-f]{128}$/.test(one.signature));
    return new Response(said.status === 204 ? null : said.body, { status: said.status, headers: said.type === undefined ? {} : { "Content-Type": said.type } });
  });
}

async function open(path: string) {
  window.history.replaceState(null, "", path);
  const { Root: App } = await import("./App");
  host = document.createElement("div");
  document.body.replaceChildren(host);
  await act(async () => {
    root = createRoot(host);
    root.render(<App />);
  });
}

const text = () => host.textContent ?? "";
const rows = (section: string) => Array.from(host.querySelectorAll(`[aria-label="${section}"] li`)).map((one) => one.textContent ?? "");
const press = async (element: Element | null | undefined) => {
  expect(element).toBeTruthy();
  await act(async () => (element as HTMLElement).click());
};
const button = (name: string) => Array.from(host.querySelectorAll("button")).find((one) => one.textContent?.trim() === name || one.getAttribute("aria-label") === name);

async function verifies(one: Sent, origin: string): Promise<boolean> {
  const message = new TextEncoder().encode(await signedMessage(one.method, one.target, Number(one.time), one.body));
  const raw = Uint8Array.from(one.signature.match(/../g)!.map((byte) => parseInt(byte, 16)));
  return crypto.subtle.verify({ name: "ECDSA", hash: "SHA-256" }, keys[origin]!.pair.publicKey, raw, message);
}

beforeEach(async () => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  await world();
});

afterEach(async () => {
  await act(async () => root?.unmount());
  root = undefined;
  vi.unstubAllGlobals();
  vi.doUnmock("./machines");
});

// The first test pays for transforming the components and the screens, which takes seconds.
describe("Pocket v2 with two Macs", { timeout: 30_000 }, () => {
  it("lists Needs you from both Macs, oldest first, each row labelled with its Mac", async () => {
    await open("/");
    await vi.waitFor(() => expect(rows("Needs you")).toHaveLength(2));
    const [first, second] = rows("Needs you");
    expect(first).toContain("Rename the slot pool");
    expect(first).toContain("laptop");
    expect(second).toContain("Trim the Drone pool");
    expect(second).toContain("studio");
    await vi.waitFor(() => expect(rows("Running").join()).toContain("Read the push payload"));
    expect(rows("Running").join()).toContain("Redraw the Needs you band");
    expect(rows("Done").join()).toContain("Fix the merge line");
  });

  it("marks only the Mac that answered 401 as not paired, and the other still lists", async () => {
    forgotten.add(LAPTOP);
    await open("/");
    await vi.waitFor(() => expect(text()).toContain("Not paired"));
    expect(window.location.pathname).toBe("/");
    const needs = rows("Needs you");
    expect(needs).toHaveLength(1);
    expect(needs[0]).toContain("Trim the Drone pool");
    const unpaired = rows("Macs");
    expect(unpaired.join()).toContain("laptop");
    expect(button("Pair again")).toBeTruthy();
  });

  it("opens a Job to its thread and answers its question on that Mac, signed with that Mac's key", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a1`);
    await vi.waitFor(() => expect(text()).toContain("The pool holds eight slots"));
    expect(text()).toContain("Which slots go when the pool shrinks?");
    await press(host.querySelector('[role="radio"][aria-label="The idle ones"]'));
    await press(button("Answer"));
    await vi.waitFor(() => expect(posted[STUDIO]).toHaveLength(1));
    expect(posted[STUDIO]![0]).toEqual({
      path: "/api/sessions/ask/answer",
      body: { session_id: "a1", call: "call-a1", answer: "allow_once", answers: [{ question: "Which slots go when the pool shrinks?", chosen: ["The idle ones"] }] },
    });
    expect(posted[LAPTOP]).toEqual([]);
    const post = sent.find((one) => one.method === "POST")!;
    expect(post.origin).toBe(STUDIO);
    expect(post.device).toBe("phone-on-studio");
    expect(await verifies(post, STUDIO)).toBe(true);
  });

  it("opens the Job a push was about when it is tapped, never home", async () => {
    const opened: string[] = [];
    const listeners: Record<string, (event: unknown) => void> = {};
    const self = {
      addEventListener: (name: string, listen: (event: unknown) => void) => (listeners[name] = listen),
      clients: { matchAll: async () => [], openWindow: async (path: string) => opened.push(path), claim: async () => undefined },
      registration: { showNotification: async () => undefined },
      skipWaiting: () => undefined,
    };
    new Function("self", readFileSync(join(import.meta.dirname, "../public/sw.js"), "utf8"))(self);
    const tap = async (data: unknown) => {
      let waited: Promise<unknown> = Promise.resolve();
      listeners["notificationclick"]!({ notification: { data, close: () => undefined }, waitUntil: (done: Promise<unknown>) => (waited = done) });
      await waited;
    };
    await tap({ session_id: "a1", title: "Trim the Drone pool", reason: "Asks", origin: STUDIO });
    await tap({ session_id: "b1", title: "Rename the slot pool", reason: "Plan" });
    expect(opened).toEqual([`/jobs/${encodeURIComponent(STUDIO)}/a1`, "/jobs/_/b1"]);

    await open(opened[1]!);
    await vi.waitFor(() => expect(text()).toContain("Call the pool slots everywhere."));
    await press(button("Approve"));
    await vi.waitFor(() => expect(posted[LAPTOP]).toEqual([{ path: "/api/sessions/plan/approve", body: { session_id: "b1", version: 1 } }]));
  });
});
