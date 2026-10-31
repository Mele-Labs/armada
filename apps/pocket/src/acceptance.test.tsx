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
/** Every `/api/live` body the stub opened, to feed bytes into or break. */
type Live = { origin: string; aborted: boolean; push: (text: string) => void; end: () => void; fail: () => void };
let lives: Live[];
let unreachable: Set<string>;
const realSleep = (ms: number) => new Promise<void>((resolve) => realTimeout(resolve, ms));

const realTimeout = globalThis.setTimeout;
const NOT_PAIRED = "This phone is not paired with this Mac. Pair it again from Bridge, Settings, Phone.";

async function world() {
  vi.resetModules();
  macs = fixtureMacs(NOW, { studio: STUDIO, laptop: LAPTOP });
  posted = { [STUDIO]: [], [LAPTOP]: [] };
  sent = [];
  forgotten = new Set();
  lives = [];
  unreachable = new Set();
  const make = () => crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, false, ["sign", "verify"]) as Promise<CryptoKeyPair>;
  keys = { [STUDIO]: { deviceId: "phone-on-studio", pair: await make() }, [LAPTOP]: { deviceId: "phone-on-laptop", pair: await make() } };
  const held = [STUDIO, LAPTOP].map((origin) => ({ origin, name: macs[origin]!.name, deviceId: keys[origin]!.deviceId, key: keys[origin]!.pair.privateKey }));
  vi.doMock("./machines", () => ({
    machines: async () => held,
    keepMachine: async () => undefined,
    forgetMachine: async () => undefined,
  }));
  vi.stubGlobal("fetch", async (url: string, init: { method?: string; body?: string; headers?: Record<string, string>; signal?: AbortSignal } = {}) => {
    const at = new URL(url);
    const method = init.method ?? "GET";
    const headers = init.headers ?? {};
    const one: Sent = {
      origin: at.origin, method, target: at.pathname + at.search, body: init.body ?? "",
      device: headers["X-Pocket-Device"] ?? "", time: headers["X-Pocket-Time"] ?? "", signature: headers["X-Pocket-Signature"] ?? "",
    };
    sent.push(one);
    if (forgotten.has(at.origin)) return new Response(NOT_PAIRED, { status: 401 });
    if (at.pathname === "/api/live") {
      if (unreachable.has(at.origin)) throw new TypeError("Failed to fetch");
      const body = new ReadableStream<Uint8Array>({
        start(controller) {
          const live: Live = {
            origin: at.origin,
            aborted: false,
            push: (chunk) => controller.enqueue(new TextEncoder().encode(chunk)),
            end: () => controller.close(),
            fail: () => controller.error(new TypeError("network error")),
          };
          lives.push(live);
          init.signal?.addEventListener("abort", () => {
            live.aborted = true;
            controller.error(new DOMException("The operation was aborted.", "AbortError"));
          });
        },
      });
      return new Response(body, { headers: { "Content-Type": "text/event-stream" } });
    }
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
  // Each Mac now answers 401 and every open stream breaks, so no stream of this world reconnects into the next.
  forgotten = new Set([STUDIO, LAPTOP]);
  lives.forEach((live) => live.fail());
  await realSleep(40);
  // A stream that broke moments after opening waits a second before it reconnects into the 401.
  if (vi.isFakeTimers()) await vi.advanceTimersByTimeAsync(1_000);
  await realSleep(vi.isFakeTimers() ? 40 : 1_050);
  vi.useRealTimers();
  Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
  vi.unstubAllGlobals();
  vi.doUnmock("./machines");
});

// The first test pays for transforming the components and the screens, which takes seconds.
describe("Pocket v2 with two Macs", { timeout: 30_000 }, () => {
  it("lists Needs you from both Macs, oldest first, each row labelled with its Mac", async () => {
    await open("/");
    await vi.waitFor(() => expect(rows("Needs you")).toHaveLength(4));
    const needs = rows("Needs you");
    const waiting: [string, string][] = [
      ["Rename the slot pool", "laptop"],
      ["Trim the Drone pool", "studio"],
      ["Open the push payload PR", "laptop"],
      ["Move Fleet to the preview", "studio"],
    ];
    waiting.forEach(([title, mac], at) => {
      expect(needs[at]).toContain(title);
      expect(needs[at]).toContain(mac);
    });
    await vi.waitFor(() => expect(rows("Running").join()).toContain("Read the push payload"));
    expect(rows("Running").join()).toContain("Redraw the Needs you band");
    expect(rows("Done").join()).toContain("Trim the Drone pool tests");
    expect(rows("Running").join()).not.toContain("Trim the Drone pool tests");
    expect(text()).not.toContain("Fix the merge line");
  });

  it("marks only the Mac that answered 401 as not paired, and the other still lists", async () => {
    forgotten.add(LAPTOP);
    await open("/");
    await vi.waitFor(() => expect(text()).toContain("Not paired"));
    expect(window.location.pathname).toBe("/");
    const needs = rows("Needs you");
    expect(needs).toHaveLength(2);
    expect(needs[0]).toContain("Trim the Drone pool");
    expect(needs[1]).toContain("Move Fleet to the preview");
    expect(needs.join()).not.toContain("laptop");
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

describe("the phone draws as Bridge does", { timeout: 30_000 }, () => {
  it("lists Home as the Cockpit's rows, Needs you in the ask tone above Running, each with its age", async () => {
    await open("/");
    await vi.waitFor(() => expect(host.querySelectorAll('[aria-label="Running"] .deck-row')).toHaveLength(5));
    const asking = Array.from(host.querySelectorAll<HTMLElement>('[aria-label="Needs you"] .deck-row'));
    expect(asking.map((row) => row.getAttribute("aria-label"))).toEqual(["Rename the slot pool", "Trim the Drone pool", "Open the push payload PR", "Move Fleet to the preview"]);
    expect(asking.every((row) => row.dataset["tone"] === "ask" && row.dataset["needs"] !== undefined)).toBe(true);
    expect(asking.every((row) => /ago$/.test(row.querySelector(".deck-row__said")?.textContent ?? ""))).toBe(true);
    const headings = Array.from(host.querySelectorAll("h2")).map((one) => one.textContent);
    expect(headings).toEqual(["Drafts", "Needs you", "Done", "Running"]);
  });

  it("ends a Job that said it is done from its row in Done, without opening it", async () => {
    await open("/");
    await vi.waitFor(() => expect(rows("Done")).toHaveLength(1));
    await press(host.querySelector('[aria-label="Done"] button[aria-label="End job"]'));
    await vi.waitFor(() => expect(posted[STUDIO]).toEqual([{ path: "/api/sessions/close", body: { session_id: "e1" } }]));
    await vi.waitFor(() => expect(host.querySelector('[aria-label="Done"]')).toBeNull());
  });

  it("opens a Job with Bridge's composer under its thread", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a1`);
    await vi.waitFor(() => expect(text()).toContain("The pool holds eight slots"));
    expect(host.querySelector("form.armada-session-composer")).not.toBeNull();
    expect(button("Home")).toBeTruthy();
  });
});

describe("a draft Job, and the acts on a Job", { timeout: 30_000 }, () => {
  const DRAFT = `/drafts/${encodeURIComponent(STUDIO)}/d1`;
  const BRIEF = "Split crates/pool into two crates: lease, which hands out slots, and sweep, which gives back the idle ones.\n\nKeep the public names. Move the tests with the code they cover.";
  const field = (label: string) => host.querySelector<HTMLTextAreaElement>(`textarea[aria-label="${label}"]`);
  const typeInto = async (element: HTMLTextAreaElement | null, value: string) => {
    expect(element).toBeTruthy();
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(element, value);
      element!.dispatchEvent(new Event("input", { bubbles: true }));
    });
  };
  const radio = (value: string) => host.querySelector<HTMLInputElement>(`input[type="radio"][value="${value}"]`);
  const openMenu = () => press(button("More"));
  const menuItem = (name: string) => Array.from(document.body.querySelectorAll('[role="menuitem"]')).find((one) => one.textContent?.trim() === name);
  const items = () => Array.from(document.body.querySelectorAll('[role="menuitem"]')).map((one) => one.textContent?.trim());
  const choose = async (name: string) => {
    await openMenu();
    await press(menuItem(name));
  };

  beforeEach(() => localStorage.clear());

  it("lists a held Job under Drafts, and neither under Needs you nor under Running", async () => {
    await open("/");
    await vi.waitFor(() => expect(rows("Drafts")).toHaveLength(1));
    expect(rows("Drafts")[0]).toContain("Split the pool crate");
    expect(rows("Needs you").join()).not.toContain("Split the pool crate");
    expect(rows("Running").join()).not.toContain("Split the pool crate");
    await press(host.querySelector('[aria-label="Drafts"] .deck-row'));
    expect(window.location.pathname).toBe(DRAFT);
  });

  it("opens a draft to its brief and Fleet's pick of how the work lands", async () => {
    await open(DRAFT);
    await vi.waitFor(() => expect(field("Brief")).toBeTruthy());
    expect(field("Brief")!.value).toBe(BRIEF);
    expect(radio("open_pr")!.checked).toBe(true);
    expect(radio("auto_land")!.checked).toBe(false);
    expect(radio("keep_local")!.checked).toBe(false);
    expect(text()).toContain("Your usual");
  });

  it("sends a draft with the brief as edited and the landing picked, then opens it as a Job", async () => {
    await open(DRAFT);
    await vi.waitFor(() => expect(field("Brief")).toBeTruthy());
    await typeInto(field("Brief"), "Split crates/pool in two. Keep the names.");
    await press(radio("keep_local"));
    await press(button("Send"));
    await vi.waitFor(() => expect(posted[STUDIO]!.filter((one) => one.path === "/api/sessions/begin")).toHaveLength(1));
    expect(posted[STUDIO]!.find((one) => one.path === "/api/sessions/begin")).toEqual({
      path: "/api/sessions/begin",
      body: { session_id: "d1", message: "Split crates/pool in two. Keep the names.", delivery: "keep_local" },
    });
    expect(posted[LAPTOP]).toEqual([]);
    await vi.waitFor(() => expect(window.location.pathname).toBe(`/jobs/${encodeURIComponent(STUDIO)}/d1`));
    await vi.waitFor(() => expect(text()).toContain("Split crates/pool in two. Keep the names."));
  });

  it("sends the landing Fleet picked when none was chosen, and the brief untouched", async () => {
    await open(DRAFT);
    await vi.waitFor(() => expect(field("Brief")).toBeTruthy());
    await press(button("Send"));
    await vi.waitFor(() => expect(posted[STUDIO]!.some((one) => one.path === "/api/sessions/begin")).toBe(true));
    expect(posted[STUDIO]!.find((one) => one.path === "/api/sessions/begin")!.body).toEqual({ session_id: "d1", message: BRIEF, delivery: "open_pr" });
  });

  it("discards a draft by closing it, and goes home", async () => {
    await open(DRAFT);
    await vi.waitFor(() => expect(field("Brief")).toBeTruthy());
    await press(button("Discard"));
    await vi.waitFor(() => expect(posted[STUDIO]).toEqual([{ path: "/api/sessions/close", body: { session_id: "d1" } }]));
    await vi.waitFor(() => expect(window.location.pathname).toBe("/"));
    await vi.waitFor(() => expect(rows("Needs you").length).toBeGreaterThan(0));
    expect(host.querySelector('[aria-label="Drafts"]')).toBeNull();
  });

  it("runs a command the Job asked for on its Mac, and then shows it ran with its exit code", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/r1`);
    await vi.waitFor(() => expect(button("Run")).toBeTruthy());
    expect(text()).toContain("scripts/preview --restart");
    expect(text()).not.toContain("exit 0");
    await press(button("Run"));
    await vi.waitFor(() => expect(posted[STUDIO]).toEqual([{ path: "/api/sessions/run/start", body: { session_id: "r1", command: "scripts/preview --restart" } }]));
    await vi.waitFor(() => expect(text()).toContain("exit 0"));
    expect(button("Run")).toBeUndefined();
    expect(host.querySelector('[role="img"][aria-label="Ran"]')).not.toBeNull();
    expect(posted[LAPTOP]).toEqual([]);
  });

  it("approves a proposed draft by answering its item `draft:<row id>` with the first choice", async () => {
    await open(`/jobs/${encodeURIComponent(LAPTOP)}/b3`);
    await vi.waitFor(() => expect(text()).toContain("Push payload carries the Job id"));
    await press(button("Approve"));
    await vi.waitFor(() => expect(posted[LAPTOP]).toEqual([{ path: "/api/sessions/waiting/answer", body: { session_id: "b3", item_id: "draft:b3-3", choice: 0 } }]));
    await vi.waitFor(() => expect(button("Approve")).toBeUndefined());
    expect(posted[STUDIO]).toEqual([]);
  });

  it("sends a proposed draft back with the owner's note, on the same item", async () => {
    await open(`/jobs/${encodeURIComponent(LAPTOP)}/b3`);
    await vi.waitFor(() => expect(text()).toContain("Push payload carries the Job id"));
    await press(button("Send back"));
    expect(button("Send back")!.hasAttribute("disabled")).toBe(true);
    await typeInto(field("Note"), "Say which crate.");
    await press(button("Send back"));
    await vi.waitFor(() => expect(posted[LAPTOP]).toEqual([{ path: "/api/sessions/waiting/answer", body: { session_id: "b3", item_id: "draft:b3-3", text: "Say which crate." } }]));
    await vi.waitFor(() => expect(button("Approve")).toBeUndefined());
  });

  it("stops a running turn from the menu", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a2`);
    await vi.waitFor(() => expect(button("More")).toBeTruthy());
    await openMenu();
    expect(items()).toEqual(expect.arrayContaining(["Stop", "Pause", "Fork", "End job"]));
    await press(menuItem("Stop"));
    await vi.waitFor(() => expect(posted[STUDIO]).toEqual([{ path: "/api/sessions/stop", body: { session_id: "a2" } }]));
  });

  it("offers no Stop to a Job with no turn running", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a1`);
    await vi.waitFor(() => expect(button("More")).toBeTruthy());
    await openMenu();
    expect(items()).not.toContain("Stop");
    expect(items()).toContain("Pause");
  });

  it("pauses a Job from the menu, and then offers Resume in place of Pause, which resumes it", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a2`);
    await vi.waitFor(() => expect(button("More")).toBeTruthy());
    await choose("Pause");
    await vi.waitFor(() => expect(posted[STUDIO]).toEqual([{ path: "/api/sessions/pause", body: { session_id: "a2" } }]));
    await vi.waitFor(async () => {
      if (items().length === 0) await openMenu();
      expect(items()).toContain("Resume");
    });
    expect(items()).not.toContain("Pause");
    await press(menuItem("Resume"));
    await vi.waitFor(() => expect(posted[STUDIO]!.map((one) => one.path)).toEqual(["/api/sessions/pause", "/api/sessions/resume"]));
    expect(posted[STUDIO]![1]!.body).toEqual({ session_id: "a2" });
  });

  it("ends a Job from the menu by closing it", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a2`);
    await vi.waitFor(() => expect(button("More")).toBeTruthy());
    await choose("End job");
    await vi.waitFor(() => expect(posted[STUDIO]).toEqual([{ path: "/api/sessions/close", body: { session_id: "a2" } }]));
  });

  it("forks a Job from the menu and opens the new Job", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a2`);
    await vi.waitFor(() => expect(button("More")).toBeTruthy());
    await choose("Fork");
    await vi.waitFor(() => expect(posted[STUDIO]).toEqual([{ path: "/api/sessions/fork", body: { session_id: "a2" } }]));
    await vi.waitFor(() => expect(window.location.pathname).toMatch(new RegExp(`^/jobs/${encodeURIComponent(STUDIO)}/a2-fork-`)));
  });
});

describe("the lists stay fresh", { timeout: 30_000 }, () => {
  const toFake = ["setTimeout", "clearTimeout", "setInterval", "clearInterval"] as const;
  const settle = () => realSleep(40);
  const until = async (ok: () => boolean) => {
    for (let waited = 0; waited < 400 && !ok(); waited++) await realSleep(5);
    expect(ok()).toBe(true);
  };
  const connects = (origin = STUDIO) => sent.filter((one) => one.origin === origin && one.target === "/api/live").length;
  const listReads = (origin = STUDIO) => sent.filter((one) => one.origin === origin && /^\/api\/(needs|sessions(\?.*)?$)/.test(one.target)).map((one) => one.target);
  const begin = async () => {
    vi.useFakeTimers({ toFake: [...toFake] });
    const fleet = await import("./fleet");
    await fleet.start();
    await until(() => lives.some((live) => live.origin === STUDIO));
    return fleet;
  };

  it("reads each Mac's live Jobs once on open, and takes Needs you from the waiting ones", async () => {
    const fleet = await begin();
    await until(() => fleet.macOf(STUDIO)!.live.length > 0 && fleet.macOf(LAPTOP)!.live.length > 0);
    await settle();
    expect(listReads(STUDIO)).toEqual(["/api/sessions"]);
    expect(listReads(LAPTOP)).toEqual(["/api/sessions"]);
    // The Mac's own list holds a draft too, which is waiting on its first message; Home sets it apart.
    expect(fleet.macOf(STUDIO)!.needs.map((one) => one.id)).toEqual(["d1", "a1", "r1"]);
    expect(fleet.macOf(LAPTOP)!.needs.map((one) => one.id)).toEqual(["b1", "b3"]);
  });

  it("aborts a stream that has gone 40 s without a byte, and a beat counts as a byte", async () => {
    const fleet = await begin();
    const live = lives.find((one) => one.origin === STUDIO)!;
    await vi.advanceTimersByTimeAsync(30_000);
    live.push(": beat\n\n");
    await settle();
    await vi.advanceTimersByTimeAsync(39_000);
    await settle();
    expect(live.aborted).toBe(false);
    await vi.advanceTimersByTimeAsync(1_001);
    await until(() => live.aborted);
    await until(() => connects() === 2);
    expect(fleet.macOf(STUDIO)!.error).toBeUndefined();
  });

  it("reconnects without an error to a stream that opened and then ended or broke, a second later when it was brief", async () => {
    const fleet = await begin();
    lives.find((one) => one.origin === STUDIO)!.end();
    await settle();
    expect(connects()).toBe(1);
    await vi.advanceTimersByTimeAsync(1_000);
    await until(() => connects() === 2);
    lives.filter((one) => one.origin === STUDIO)[1]!.fail();
    await vi.advanceTimersByTimeAsync(1_000);
    await until(() => connects() === 3);
    expect(fleet.macOf(STUDIO)!.error).toBeUndefined();
  });

  it("tells the owner a Mac did not answer when the connect itself fails, and tries again after 3 s", async () => {
    unreachable.add(STUDIO);
    vi.useFakeTimers({ toFake: [...toFake] });
    const fleet = await import("./fleet");
    await fleet.start();
    await until(() => connects() === 1 && fleet.macOf(STUDIO)!.error !== undefined);
    await vi.advanceTimersByTimeAsync(2_999);
    await settle();
    expect(connects()).toBe(1);
    await vi.advanceTimersByTimeAsync(1);
    await until(() => connects() === 2);
  });

  it("reads again on every open stream when the app comes back on screen or the phone is online again", async () => {
    await begin();
    const first = lives.find((one) => one.origin === STUDIO)!;
    await until(() => listReads().length === 1);
    Object.defineProperty(document, "visibilityState", { value: "hidden", configurable: true });
    document.dispatchEvent(new Event("visibilitychange"));
    await settle();
    expect(connects()).toBe(1);
    Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
    document.dispatchEvent(new Event("visibilitychange"));
    await until(() => first.aborted && connects() === 2 && connects(LAPTOP) === 2);
    await until(() => listReads().length === 2);
    window.dispatchEvent(new Event("online"));
    await until(() => connects() === 3);
    await until(() => listReads().length === 3);
  });
});
