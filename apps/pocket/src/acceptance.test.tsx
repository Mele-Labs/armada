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
import { everyState, fixtureMacs, type FixtureMac } from "./fixture/wire";
import { signedMessage } from "./signing";

const STUDIO = "https://studio.tail0.ts.net";
const LAPTOP = "https://laptop.tail0.ts.net";
const NOW = Date.now();

type Key = {
  deviceId: string;
  pair: CryptoKeyPair;
};
type Sent = {
  origin: string;
  method: string;
  target: string;
  body: string;
  device: string;
  time: string;
  signature: string;
};

let macs: Record<string, FixtureMac>;
let keys: Record<string, Key>;
let posted: Record<string, Posted[]>;
let sent: Sent[];
let forgotten: Set<string>;
let root: Root | undefined;
let host: HTMLElement;
/** Every `/api/live` body the stub opened, to feed bytes into or break. */
type Live = {
  origin: string;
  aborted: boolean;
  push: (text: string) => void;
  end: () => void;
  fail: () => void;
};
let lives: Live[];
let unreachable: Set<string>;
const realSleep = (ms: number) => new Promise<void>((resolve) => realTimeout(resolve, ms));

const realTimeout = globalThis.setTimeout;
const NOT_PAIRED =
  "This phone is not paired with this Mac. Pair it again from Bridge, Settings, Phone.";

async function world() {
  vi.resetModules();
  macs = fixtureMacs(NOW, {
    studio: STUDIO,
    laptop: LAPTOP,
  });
  posted = {
    [STUDIO]: [],
    [LAPTOP]: [],
  };
  sent = [];
  forgotten = new Set();
  lives = [];
  unreachable = new Set();
  const make = () =>
    crypto.subtle.generateKey(
      {
        name: "ECDSA",
        namedCurve: "P-256",
      },
      false,
      [
        "sign",
        "verify",
      ],
    ) as Promise<CryptoKeyPair>;
  keys = {
    [STUDIO]: {
      deviceId: "phone-on-studio",
      pair: await make(),
    },
    [LAPTOP]: {
      deviceId: "phone-on-laptop",
      pair: await make(),
    },
  };
  const held = [
    STUDIO,
    LAPTOP,
  ].map((origin) => ({
    origin,
    name: macs[origin]!.name,
    deviceId: keys[origin]!.deviceId,
    key: keys[origin]!.pair.privateKey,
  }));
  vi.doMock("./machines", () => ({
    machines: async () => held,
    keepMachine: async () => undefined,
    forgetMachine: async () => undefined,
  }));
  vi.stubGlobal(
    "fetch",
    async (
      url: string,
      init: {
        method?: string;
        body?: string;
        headers?: Record<string, string>;
        signal?: AbortSignal;
      } = {},
    ) => {
      const at = new URL(url);
      const method = init.method ?? "GET";
      const headers = init.headers ?? {};
      const one: Sent = {
        origin: at.origin,
        method,
        target: at.pathname + at.search,
        body: init.body ?? "",
        device: headers["X-Pocket-Device"] ?? "",
        time: headers["X-Pocket-Time"] ?? "",
        signature: headers["X-Pocket-Signature"] ?? "",
      };
      sent.push(one);
      if (forgotten.has(at.origin))
        return new Response(NOT_PAIRED, {
          status: 401,
        });
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
        return new Response(body, {
          headers: {
            "Content-Type": "text/event-stream",
          },
        });
      }
      const mac = macs[at.origin];
      if (mac === undefined) throw new TypeError("Failed to fetch");
      const said = answer(
        mac,
        posted[at.origin]!,
        method,
        one.target,
        one.body,
        /^[0-9a-f]{128}$/.test(one.signature),
      );
      // What the Mac's stream says a moment after this answer, as a Fleet event reaches the phone.
      if (said.then !== undefined) {
        const { after, change } = said.then;
        realTimeout(
          () =>
            lives
              .filter((live) => live.origin === at.origin && !live.aborted)
              .forEach((live) => {
                try {
                  live.push(`data: ${JSON.stringify(change)}\n\n`);
                } catch {
                  // That stream was closed in the meantime.
                }
              }),
          after,
        );
      }
      return new Response(said.status === 204 ? null : said.body, {
        status: said.status,
        headers:
          said.type === undefined
            ? {}
            : {
                "Content-Type": said.type,
              },
      });
    },
  );
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
const rows = (section: string) =>
  Array.from(host.querySelectorAll(`[aria-label="${section}"] li`)).map(
    (one) => one.textContent ?? "",
  );
/** Home's Job rows as drawn, all of them or those under one repository's heading. */
const jobs = (heading?: string) =>
  Array.from(
    host.querySelectorAll<HTMLElement>(
      `.pk-home ${heading === undefined ? "" : `section[aria-label="${heading}"] `}button.deck-row`,
    ),
  );
const titles = (heading?: string) => jobs(heading).map((one) => one.getAttribute("aria-label"));
const job = (title: string) =>
  host.querySelector<HTMLElement>(`.pk-home button.deck-row[aria-label="${title}"]`);
const press = async (element: Element | null | undefined) => {
  expect(element).toBeTruthy();
  await act(async () => (element as HTMLElement).click());
};
const button = (name: string) =>
  Array.from(host.querySelectorAll("button")).find(
    (one) => one.textContent?.trim() === name || one.getAttribute("aria-label") === name,
  );

async function verifies(one: Sent, origin: string): Promise<boolean> {
  const message = new TextEncoder().encode(
    await signedMessage(one.method, one.target, Number(one.time), one.body),
  );
  const raw = Uint8Array.from(one.signature.match(/../g)!.map((byte) => parseInt(byte, 16)));
  return crypto.subtle.verify(
    {
      name: "ECDSA",
      hash: "SHA-256",
    },
    keys[origin]!.pair.publicKey,
    raw,
    message,
  );
}

beforeEach(async () => {
  (
    globalThis as {
      IS_REACT_ACT_ENVIRONMENT?: boolean;
    }
  ).IS_REACT_ACT_ENVIRONMENT = true;
  await world();
});

afterEach(async () => {
  await act(async () => root?.unmount());
  root = undefined;
  // Each Mac now answers 401 and every open stream breaks, so no stream of this world reconnects into the next.
  forgotten = new Set([
    STUDIO,
    LAPTOP,
  ]);
  lives.forEach((live) => live.fail());
  await realSleep(40);
  // A stream that broke moments after opening waits a second before it reconnects into the 401.
  if (vi.isFakeTimers()) await vi.advanceTimersByTimeAsync(1_000);
  await realSleep(vi.isFakeTimers() ? 40 : 1_050);
  vi.useRealTimers();
  Object.defineProperty(document, "visibilityState", {
    value: "visible",
    configurable: true,
  });
  vi.unstubAllGlobals();
  vi.doUnmock("./machines");
});

// The first test pays for transforming the components and the screens, which takes seconds.
describe("Pocket v2 with two Macs", {
  timeout: 30_000,
}, () => {
  it("lists every Job from both Macs in Fleet's order under a heading for each repository, each row labelled with its Mac", async () => {
    await open("/");
    await vi.waitFor(() => expect(jobs()).toHaveLength(20));
    expect(Array.from(host.querySelectorAll("h2")).map((one) => one.textContent)).toEqual([
      "armada",
      "store",
    ]);
    const listed: [
      string,
      string,
    ][] = [
      [
        "Review what a Job waits on",
        "studio",
      ],
      [
        "Split the pool crate",
        "studio",
      ],
      [
        "Trim the Drone pool",
        "studio",
      ],
      [
        "Move Fleet to the preview",
        "studio",
      ],
      [
        "Fix the lost lease",
        "studio",
      ],
      [
        "Move the walk engine",
        "studio",
      ],
      [
        "Close out the tracker mod",
        "studio",
      ],
      [
        "Plan the phone screens",
        "studio",
      ],
      [
        "Build the Draft screen",
        "studio",
      ],
      [
        "Measure the Draft screen on a phone",
        "studio",
      ],
      [
        "Write the Draft screen copy",
        "studio",
      ],
      [
        "Redraw the Needs you band",
        "studio",
      ],
      [
        "Draw the slot pool on the phone",
        "studio",
      ],
      [
        "Trim the Drone pool tests",
        "studio",
      ],
      [
        "Check the restart notes",
        "studio",
      ],
      [
        "Rename the slot pool",
        "laptop",
      ],
      [
        "Open the push payload PR",
        "laptop",
      ],
      [
        "Read the push payload",
        "laptop",
      ],
      [
        "Trace every slot lease",
        "laptop",
      ],
      [
        "Move the slot docs",
        "laptop",
      ],
    ];
    expect(titles()).toEqual(listed.map(([title]) => title));
    expect(titles("armada")).toEqual(listed.slice(0, 15).map(([title]) => title));
    expect(titles("store")).toEqual(listed.slice(15).map(([title]) => title));
    for (const [title, mac] of listed)
      expect(job(title)!.querySelector(".pk-row__mac")!.textContent).toBe(mac);
    const said = job("Trim the Drone pool tests")!.querySelector(".deck-row__says")!.textContent;
    expect(said).toContain("The pool tests now cover shrinking to four slots");
    expect(text()).not.toContain("Fix the merge line");
  });

  it("marks only the Mac that answered 401 as not paired, and the other still lists", async () => {
    forgotten.add(LAPTOP);
    await open("/");
    await vi.waitFor(() => expect(text()).toContain("Not paired"));
    expect(window.location.pathname).toBe("/");
    await vi.waitFor(() => expect(jobs()).toHaveLength(15));
    expect(titles()).toEqual([
      "Review what a Job waits on",
      "Split the pool crate",
      "Trim the Drone pool",
      "Move Fleet to the preview",
      "Fix the lost lease",
      "Move the walk engine",
      "Close out the tracker mod",
      "Plan the phone screens",
      "Build the Draft screen",
      "Measure the Draft screen on a phone",
      "Write the Draft screen copy",
      "Redraw the Needs you band",
      "Draw the slot pool on the phone",
      "Trim the Drone pool tests",
      "Check the restart notes",
    ]);
    // One repository is left, so no heading stands over it.
    expect(host.querySelectorAll("h2")).toHaveLength(0);
    expect(
      jobs()
        .map((one) => one.textContent)
        .join(),
    ).not.toContain("laptop");
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
      body: {
        session_id: "a1",
        call: "call-a1",
        answer: "allow_once",
        answers: [
          {
            question: "Which slots go when the pool shrinks?",
            chosen: [
              "The idle ones",
            ],
          },
        ],
      },
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
      addEventListener: (name: string, listen: (event: unknown) => void) =>
        (listeners[name] = listen),
      clients: {
        matchAll: async () => [],
        openWindow: async (path: string) => opened.push(path),
        claim: async () => undefined,
      },
      registration: {
        showNotification: async () => undefined,
      },
      skipWaiting: () => undefined,
    };
    new Function("self", readFileSync(join(import.meta.dirname, "../public/sw.js"), "utf8"))(self);
    const tap = async (data: unknown) => {
      let waited: Promise<unknown> = Promise.resolve();
      listeners["notificationclick"]!({
        notification: {
          data,
          close: () => undefined,
        },
        waitUntil: (done: Promise<unknown>) => (waited = done),
      });
      await waited;
    };
    await tap({
      session_id: "a1",
      title: "Trim the Drone pool",
      reason: "Asks",
      origin: STUDIO,
    });
    await tap({
      session_id: "b1",
      title: "Rename the slot pool",
      reason: "Plan",
    });
    expect(opened).toEqual([
      `/jobs/${encodeURIComponent(STUDIO)}/a1`,
      "/jobs/_/b1",
    ]);

    await open(opened[1]!);
    await vi.waitFor(() => expect(text()).toContain("Call the pool slots everywhere."));
    await press(button("Approve"));
    await vi.waitFor(() =>
      expect(posted[LAPTOP]).toEqual([
        {
          path: "/api/sessions/plan/approve",
          body: {
            session_id: "b1",
            version: 1,
          },
        },
      ]),
    );
  });
});

describe("the phone draws as Bridge does", {
  timeout: 30_000,
}, () => {
  it("says what holds a waiting Job on its second line, pulses only the Jobs that need him, and gives a Job's header the row's words", async () => {
    const states = everyState(NOW);
    macs[STUDIO]!.live.push(...states.live);
    Object.assign(macs[STUDIO]!.threads, states.threads);
    await open("/");
    await vi.waitFor(() => expect(job("Land the slot fixes")).not.toBeNull());
    expect(job("Land the slot fixes")!.querySelector(".deck-row__says")!.textContent).toBe(
      "#3209 in the merge queuestudio",
    );
    // What the Cockpit files as waiting, or running, holds nothing of his: none of it pulses.
    for (const title of [
      "Land the slot fixes",
      "Fix the push retry",
      "Deploy the store to staging",
      "Check the docs pass",
      "Rewrite the import map",
      "Tidy the doc index",
      "Sort the settings groups",
      "Sort the palette rows",
    ])
      expect(job(title)!.dataset["needs"]).toBeUndefined();
    expect(job("Trace the slow restart")!.dataset["needs"]).toBeDefined();
    // Under its repository's heading a row does not name the repository again.
    expect(job("Sort the palette rows")!.querySelector(".deck-row__says")!.textContent).toBe(
      "studio",
    );

    job("Land the slot fixes")!.click();
    await vi.waitFor(() =>
      expect(
        host.querySelector('.pk-pane [aria-label^="#3209 in the merge queue"]'),
      ).not.toBeNull(),
    );
  });

  it("lists Home as the Cockpit's rows in Fleet's order, a Job that needs him in the ask tone, each with its age", async () => {
    await open("/");
    await vi.waitFor(() => expect(jobs()).toHaveLength(20));
    const asking = jobs().filter((row) => row.dataset["needs"] !== undefined);
    expect(asking.map((row) => row.getAttribute("aria-label"))).toEqual([
      "Review what a Job waits on",
      "Trim the Drone pool",
      "Move Fleet to the preview",
      "Fix the lost lease",
      "Move the walk engine",
      "Close out the tracker mod",
      "Write the Draft screen copy",
      "Rename the slot pool",
      "Open the push payload PR",
    ]);
    expect(
      jobs().every(
        (row) => (row.dataset["tone"] === "ask") === (row.dataset["needs"] !== undefined),
      ),
    ).toBe(true);
    expect(
      asking.every((row) => /ago$/.test(row.querySelector(".deck-row__said")?.textContent ?? "")),
    ).toBe(true);
    const headings = Array.from(host.querySelectorAll("h2")).map((one) => one.textContent);
    expect(headings).toEqual([
      "armada",
      "store",
    ]);
  });

  it("steps a Job another started in under its starter, one step for each starter above it, and links only a Job whose starter is not listed", async () => {
    // The Job that started this one has ended, so it is no longer listed.
    macs[STUDIO]!.live.find((one) => one.id === "a4")!.from = {
      id: "ended",
      title: "Plan the restart notes",
    };
    await open("/");
    await vi.waitFor(() => expect(jobs()).toHaveLength(20));
    const depth = (title: string) => {
      const item = job(title)!.closest("li")!;
      return {
        nested: item.dataset["nested"] !== undefined,
        step: item.style.getPropertyValue("--pk-depth"),
      };
    };
    expect(depth("Plan the phone screens")).toEqual({
      nested: false,
      step: "",
    });
    expect(depth("Build the Draft screen")).toEqual({
      nested: true,
      step: "1",
    });
    expect(depth("Measure the Draft screen on a phone")).toEqual({
      nested: true,
      step: "2",
    });
    expect(depth("Write the Draft screen copy")).toEqual({
      nested: true,
      step: "1",
    });
    const order = titles("armada");
    const at = (title: string) => order.indexOf(title);
    expect(at("Build the Draft screen")).toBe(at("Plan the phone screens") + 1);
    expect(at("Measure the Draft screen on a phone")).toBe(at("Build the Draft screen") + 1);
    expect(at("Write the Draft screen copy")).toBe(at("Measure the Draft screen on a phone") + 1);
    // Each row turns its starter's line in at an elbow; the line runs on down only past rows its starter has more under.
    const guides = (title: string) =>
      Array.from(job(title)!.closest("li")!.querySelectorAll<HTMLElement>(".pk-guide")).map(
        (one) =>
          `${one.classList.contains("pk-guide--elbow") ? "elbow" : "line"}@${one.style.getPropertyValue("--pk-column")}`,
      );
    expect(guides("Plan the phone screens")).toEqual([]);
    expect(guides("Build the Draft screen")).toEqual([
      "line@0",
      "elbow@0",
    ]);
    expect(guides("Measure the Draft screen on a phone")).toEqual([
      "line@0",
      "elbow@1",
    ]);
    expect(guides("Write the Draft screen copy")).toEqual([
      "elbow@0",
    ]);
    // The link mark names the starter, and a row stepped in under it has no need of one.
    const linked = (title: string) => job(title)!.querySelector(".deck-row__link [role='img']");
    expect(linked("Build the Draft screen")).toBeNull();
    expect(linked("Measure the Draft screen on a phone")).toBeNull();
    expect(linked("Write the Draft screen copy")).toBeNull();
    expect(linked("Plan the phone screens")).toBeNull();
    expect(linked("Check the restart notes")!.getAttribute("aria-label")).toBe(
      "Plan the restart notes",
    );
    expect(depth("Check the restart notes").nested).toBe(false);
  });

  it("ends a Job that said it is done from its row, without opening it", async () => {
    await open("/");
    await vi.waitFor(() => expect(job("Trim the Drone pool tests")).not.toBeNull());
    await press(
      job("Trim the Drone pool tests")!
        .closest("li")!
        .querySelector('button[aria-label="End job"]'),
    );
    await vi.waitFor(() =>
      expect(posted[STUDIO]).toEqual([
        {
          path: "/api/sessions/close",
          body: {
            session_id: "e1",
          },
        },
      ]),
    );
    await vi.waitFor(() => expect(job("Trim the Drone pool tests")).toBeNull());
    expect(window.location.pathname).toBe("/");
  });

  it("opens a Job with Bridge's composer under its thread", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a1`);
    await vi.waitFor(() => expect(text()).toContain("The pool holds eight slots"));
    expect(host.querySelector("form.armada-session-composer")).not.toBeNull();
    expect(button("Home")).toBeTruthy();
  });
});

describe("a draft Job, and the acts on a Job", {
  timeout: 30_000,
}, () => {
  const DRAFT = `/drafts/${encodeURIComponent(STUDIO)}/d1`;
  const BRIEF =
    "Split crates/pool into two crates: lease, which hands out slots, and sweep, which gives back the idle ones.\n\nKeep the public names. Move the tests with the code they cover.";
  const field = (label: string) =>
    host.querySelector<HTMLTextAreaElement>(`textarea[aria-label="${label}"]`);
  const typeInto = async (element: HTMLTextAreaElement | null, value: string) => {
    expect(element).toBeTruthy();
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(
        element,
        value,
      );
      element!.dispatchEvent(
        new Event("input", {
          bubbles: true,
        }),
      );
    });
  };
  const radio = (value: string) =>
    host.querySelector<HTMLInputElement>(`input[type="radio"][value="${value}"]`);
  const openMenu = () => press(button("More"));
  const menuItem = (name: string) =>
    Array.from(document.body.querySelectorAll('[role="menuitem"]')).find(
      (one) => one.textContent?.trim() === name,
    );
  const items = () =>
    Array.from(document.body.querySelectorAll('[role="menuitem"]')).map((one) =>
      one.textContent?.trim(),
    );
  const choose = async (name: string) => {
    await openMenu();
    await press(menuItem(name));
  };

  beforeEach(() => localStorage.clear());

  it("lists a held Job as a row of its own, not in the ask tone, and opens it to its draft", async () => {
    await open("/");
    await vi.waitFor(() => expect(job("Split the pool crate")).not.toBeNull());
    expect(titles().filter((one) => one === "Split the pool crate")).toHaveLength(1);
    expect(job("Split the pool crate")!.dataset["needs"]).toBeUndefined();
    expect(job("Split the pool crate")!.dataset["tone"]).toBe("calm");
    await press(job("Split the pool crate"));
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
    await vi.waitFor(() =>
      expect(posted[STUDIO]!.filter((one) => one.path === "/api/sessions/begin")).toHaveLength(1),
    );
    expect(posted[STUDIO]!.find((one) => one.path === "/api/sessions/begin")).toEqual({
      path: "/api/sessions/begin",
      body: {
        session_id: "d1",
        message: "Split crates/pool in two. Keep the names.",
        delivery: "keep_local",
      },
    });
    expect(posted[LAPTOP]).toEqual([]);
    await vi.waitFor(() =>
      expect(window.location.pathname).toBe(`/jobs/${encodeURIComponent(STUDIO)}/d1`),
    );
    await vi.waitFor(() => expect(text()).toContain("Split crates/pool in two. Keep the names."));
  });

  it("sends the landing Fleet picked when none was chosen, and the brief untouched", async () => {
    await open(DRAFT);
    await vi.waitFor(() => expect(field("Brief")).toBeTruthy());
    await press(button("Send"));
    await vi.waitFor(() =>
      expect(posted[STUDIO]!.some((one) => one.path === "/api/sessions/begin")).toBe(true),
    );
    expect(posted[STUDIO]!.find((one) => one.path === "/api/sessions/begin")!.body).toEqual({
      session_id: "d1",
      message: BRIEF,
      delivery: "open_pr",
    });
  });

  it("discards a draft by closing it, and goes home", async () => {
    await open(DRAFT);
    await vi.waitFor(() => expect(field("Brief")).toBeTruthy());
    await press(button("Discard"));
    await vi.waitFor(() =>
      expect(posted[STUDIO]).toEqual([
        {
          path: "/api/sessions/close",
          body: {
            session_id: "d1",
          },
        },
      ]),
    );
    await vi.waitFor(() => expect(window.location.pathname).toBe("/"));
    await vi.waitFor(() => expect(jobs().length).toBeGreaterThan(0));
    await vi.waitFor(() => expect(job("Split the pool crate")).toBeNull());
  });

  it("forgets a draft's brief once it is sent, so nothing of it stays kept", async () => {
    const kept = `armada:kept:pocket:${STUDIO}:d1:brief`;
    await open(DRAFT);
    await vi.waitFor(() => expect(field("Brief")).toBeTruthy());
    await typeInto(field("Brief"), "Split crates/pool in two.");
    expect(localStorage.getItem(kept)).not.toBeNull();
    await press(button("Send"));
    await vi.waitFor(() =>
      expect(window.location.pathname).toBe(`/jobs/${encodeURIComponent(STUDIO)}/d1`),
    );
    expect(localStorage.getItem(kept)).toBeNull();
  });

  it("opens the next Dispatch empty once one is sent, and its key does not say send", async () => {
    await open("/dispatch");
    await vi.waitFor(() => expect(host.querySelector('[role="radio"]')).toBeTruthy());
    expect(field("Job")!.getAttribute("enterkeyhint")).toBeNull();
    await typeInto(field("Job"), "Old message");
    await press(button("Dispatch"));
    await vi.waitFor(() => expect(window.location.pathname).toBe("/"));
    expect(localStorage.getItem("armada:kept:pocket:dispatch")).toBeNull();
    await act(async () => root?.unmount());
    await open("/dispatch");
    await vi.waitFor(() => expect(field("Job")).toBeTruthy());
    expect(field("Job")!.value).toBe("");
  });

  it("breaks the line on Enter in a Job's box, and only Send sends", async () => {
    localStorage.setItem(
      `armada:kept:session:pocket:${STUDIO}:a2:message`,
      JSON.stringify({
        text: "first",
        tags: [],
      }),
    );
    const edited = vi.fn(() => true);
    document.execCommand = edited;
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a2`);
    await vi.waitFor(() => expect(button("Send")?.hasAttribute("disabled")).toBe(false));
    await act(async () => {
      host.querySelector('.pk-dock [role="textbox"]')!.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "Enter",
          bubbles: true,
          cancelable: true,
        }),
      );
    });
    expect(edited).toHaveBeenCalledWith("insertLineBreak");
    const messages = () => posted[STUDIO]!.filter((one) => one.path === "/api/sessions/message");
    expect(messages()).toEqual([]);
    await press(button("Send"));
    await vi.waitFor(() => expect(messages()).toHaveLength(1));
    expect(messages()[0]!.body).toMatchObject({
      session_id: "a2",
      text: "first",
    });
  });

  describe("what Fleet itself asks", () => {
    const ask = (name: string) => host.querySelector(`[role="group"][aria-label="${name}"]`);
    const option = (name: string, label: string) =>
      Array.from(ask(name)?.querySelectorAll('[aria-label="Options"] button') ?? []).find((one) =>
        one.textContent?.startsWith(label),
      );
    const answers = () =>
      posted[STUDIO]!.filter((one) => one.path === "/api/sessions/waiting/answer").map(
        (one) => one.body,
      );

    it.each([
      [
        "l1",
        "How should this work be delivered?",
        "delivery",
        "Keep local",
        2,
      ],
      [
        "s1",
        "Every slot is in use",
        "slot-choice",
        "Add a slot",
        1,
      ],
      [
        "o1",
        "Mark the tracker tasks done?",
        "task_done:tracker",
        "Keep open",
        1,
      ],
      [
        "o1",
        "Move ARM-41 to In progress",
        "rule_act:41",
        "Skip",
        1,
      ],
      [
        "o1",
        "QA deploy: smoke failed",
        "watch_not_ours:w1",
        "Fix it",
        1,
      ],
      [
        "o1",
        "Where does tickets open in armada?",
        "words:tickets:armada",
        "https://tracker.example/{match}",
        0,
      ],
    ])("answers %s's %s with its choice", async (job, name, item, label, choice) => {
      await open(`/jobs/${encodeURIComponent(STUDIO)}/${job}`);
      await vi.waitFor(() => expect(ask(name)).toBeTruthy());
      await press(option(name, label));
      await vi.waitFor(() =>
        expect(answers()).toEqual([
          {
            session_id: job,
            item_id: item,
            choice,
          },
        ]),
      );
      await vi.waitFor(() => expect(ask(name)).toBeNull());
    });

    it("draws each landing's description, since a phone shows no title", async () => {
      await open(`/jobs/${encodeURIComponent(STUDIO)}/l1`);
      const name = "How should this work be delivered?";
      await vi.waitFor(() => expect(ask(name)).toBeTruthy());
      expect(option(name, "Auto land")!.textContent).toContain(
        "Open a pull request, watch ci, merge it.",
      );
      expect(option(name, "Auto land")!.getAttribute("title")).toBeNull();
    });

    it("offers a words box only where Fleet takes words", async () => {
      await open(`/jobs/${encodeURIComponent(STUDIO)}/o1`);
      await vi.waitFor(() => expect(ask("Mark the tracker tasks done?")).toBeTruthy());
      const box = (name: string) => ask(name)!.querySelector("textarea");
      expect(box("Mark the tracker tasks done?")).toBeNull();
      expect(box("Move ARM-41 to In progress")).toBeNull();
      expect(box("QA deploy: smoke failed")).not.toBeNull();
      expect(box("Where does tickets open in armada?")).not.toBeNull();
    });

    it("takes an address in words, with no Other to press", async () => {
      const name = "Where does tickets open in armada?";
      await open(`/jobs/${encodeURIComponent(STUDIO)}/o1`);
      await vi.waitFor(() => expect(ask(name)).toBeTruthy());
      expect(option(name, "Other")).toBeUndefined();
      await typeInto(ask(name)!.querySelector("textarea"), "https://other.example/{match}");
      await press(
        Array.from(ask(name)!.querySelectorAll("button")).find(
          (one) => one.textContent === "Answer",
        ),
      );
      await vi.waitFor(() =>
        expect(answers()).toEqual([
          {
            session_id: "o1",
            item_id: "words:tickets:armada",
            text: "https://other.example/{match}",
          },
        ]),
      );
    });
  });

  it("runs a command the Job asked for on its Mac, and then shows it ran with its exit code", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/r1`);
    await vi.waitFor(() => expect(button("Run")).toBeTruthy());
    expect(text()).toContain("scripts/preview --restart");
    expect(text()).not.toContain("exit 0");
    await press(button("Run"));
    await vi.waitFor(() =>
      expect(posted[STUDIO]).toEqual([
        {
          path: "/api/sessions/run/start",
          body: {
            session_id: "r1",
            command: "scripts/preview --restart",
          },
        },
      ]),
    );
    await vi.waitFor(() => expect(text()).toContain("exit 0"));
    expect(button("Run")).toBeUndefined();
    expect(host.querySelector('[role="img"][aria-label="Ran"]')).not.toBeNull();
    expect(posted[LAPTOP]).toEqual([]);
  });

  it("approves a proposed draft by answering its item `draft:<row id>` with the first choice", async () => {
    await open(`/jobs/${encodeURIComponent(LAPTOP)}/b3`);
    await vi.waitFor(() => expect(text()).toContain("Push payload carries the Job id"));
    await press(button("Approve"));
    await vi.waitFor(() =>
      expect(posted[LAPTOP]).toEqual([
        {
          path: "/api/sessions/waiting/answer",
          body: {
            session_id: "b3",
            item_id: "draft:b3-3",
            choice: 0,
          },
        },
      ]),
    );
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
    await vi.waitFor(() =>
      expect(posted[LAPTOP]).toEqual([
        {
          path: "/api/sessions/waiting/answer",
          body: {
            session_id: "b3",
            item_id: "draft:b3-3",
            text: "Say which crate.",
          },
        },
      ]),
    );
    await vi.waitFor(() => expect(button("Approve")).toBeUndefined());
  });

  it("stops a running turn from the menu", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a2`);
    await vi.waitFor(() => expect(button("More")).toBeTruthy());
    await openMenu();
    expect(items()).toEqual(
      expect.arrayContaining([
        "Stop",
        "Pause",
        "Fork",
        "End job",
      ]),
    );
    await press(menuItem("Stop"));
    await vi.waitFor(() =>
      expect(posted[STUDIO]).toEqual([
        {
          path: "/api/sessions/stop",
          body: {
            session_id: "a2",
          },
        },
      ]),
    );
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
    await vi.waitFor(() =>
      expect(posted[STUDIO]).toEqual([
        {
          path: "/api/sessions/pause",
          body: {
            session_id: "a2",
          },
        },
      ]),
    );
    await vi.waitFor(async () => {
      if (items().length === 0) await openMenu();
      expect(items()).toContain("Resume");
    });
    expect(items()).not.toContain("Pause");
    await press(menuItem("Resume"));
    await vi.waitFor(() =>
      expect(posted[STUDIO]!.map((one) => one.path)).toEqual([
        "/api/sessions/pause",
        "/api/sessions/resume",
      ]),
    );
    expect(posted[STUDIO]![1]!.body).toEqual({
      session_id: "a2",
    });
  });

  it("ends a Job from the menu by closing it", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a2`);
    await vi.waitFor(() => expect(button("More")).toBeTruthy());
    await choose("End job");
    await vi.waitFor(() =>
      expect(posted[STUDIO]).toEqual([
        {
          path: "/api/sessions/close",
          body: {
            session_id: "a2",
          },
        },
      ]),
    );
  });

  it("forks a Job from the menu and opens the new Job", async () => {
    await open(`/jobs/${encodeURIComponent(STUDIO)}/a2`);
    await vi.waitFor(() => expect(button("More")).toBeTruthy());
    await choose("Fork");
    await vi.waitFor(() =>
      expect(posted[STUDIO]).toEqual([
        {
          path: "/api/sessions/fork",
          body: {
            session_id: "a2",
          },
        },
      ]),
    );
    await vi.waitFor(() =>
      expect(window.location.pathname).toMatch(
        new RegExp(`^/jobs/${encodeURIComponent(STUDIO)}/a2-fork-`),
      ),
    );
  });

  describe("what a Job waits on", () => {
    const F1 = `/jobs/${encodeURIComponent(STUDIO)}/f1`;
    const SLOT_POOL = "/Users/user/armada/.armada/documents/slot-pool.html";
    const PHONE_REACH = "/Users/user/armada/.armada/documents/phone-reach.html";
    const POOL_CARD = "Document: Slot pool, Revision 1";
    const REACH_CARD = "Document: Phone reach, Revision 2";
    const f1 = () => macs[STUDIO]!.threads["f1"]!;
    const held = (target: string) =>
      f1().session.attachments!.find((one) => one.target === target)!;
    const card = (name: string) =>
      host.querySelector<HTMLElement>(`[role="article"][aria-label="${name}"]`);
    const inCard = (name: string, label: string) =>
      Array.from(card(name)?.querySelectorAll("button") ?? []).find(
        (one) => one.textContent?.trim() === label,
      );
    const group = (name: string) =>
      Array.from(host.querySelectorAll('[role="group"]')).find(
        (one) => one.getAttribute("aria-label") === name,
      );
    const inGroup = (name: string, label: string) =>
      Array.from(group(name)?.querySelectorAll("button") ?? []).find(
        (one) => one.textContent?.trim() === label,
      );
    const to = (path: string) =>
      posted[STUDIO]!.filter((one) => one.path === path).map((one) => one.body);
    const opened = async () => {
      await open(F1);
      await vi.waitFor(() => expect(card(POOL_CARD)).toBeTruthy());
    };
    const note = async (text: string) => {
      await typeInto(field("Note"), text);
    };

    describe("a document", () => {
      it("is a card in the thread for each page, at its newest revision", async () => {
        await opened();
        expect(card(REACH_CARD)).toBeTruthy();
        expect(card("Document: Phone reach, Revision 1")).toBeNull();
      });

      it("approves from its card with the revision the ledger holds", async () => {
        await opened();
        await press(inCard(POOL_CARD, "Approve"));
        await vi.waitFor(() =>
          expect(to("/api/sessions/document/answer")).toEqual([
            {
              session_id: "f1",
              document: `document:${SLOT_POOL}`,
              revision: "1",
              how: "approved",
            },
          ]),
        );
      });

      it("sends back from its card with a note, on the newest revision", async () => {
        await opened();
        await press(inCard(REACH_CARD, "Send back"));
        await note("Cut the table to four columns.");
        await press(inCard(REACH_CARD, "Send back"));
        await vi.waitFor(() =>
          expect(to("/api/sessions/document/answer")).toEqual([
            {
              session_id: "f1",
              document: `document:${PHONE_REACH}`,
              revision: "2",
              how: "sent_back",
              words: "Cut the table to four columns.",
            },
          ]),
        );
      });

      it("opens its page from the card", async () => {
        await opened();
        await press(host.querySelector(`[aria-label="Open Phone reach, Revision 2"]`));
        await vi.waitFor(() =>
          expect(window.location.pathname + window.location.search).toBe(
            `${F1}/document?page=${encodeURIComponent(PHONE_REACH)}`,
          ),
        );
      });

      describe("its page", () => {
        const page = () => `${F1}/document?page=${encodeURIComponent(PHONE_REACH)}`;
        const read = async () => {
          await open(page());
          await vi.waitFor(() => expect(host.querySelector(".pk-doc__scroll")).toBeTruthy());
        };

        it("reads the newest kept revision, and not an older one", async () => {
          await read();
          const asked = sent
            .filter((one) => one.target.startsWith("/api/sessions/shown"))
            .map((one) => decodeURIComponent(one.target));
          expect(asked.some((one) => one.includes("phone-reach-r2.html"))).toBe(true);
          expect(asked.some((one) => one.includes("phone-reach-r1.html"))).toBe(false);
          expect(host.querySelectorAll(".pk-doc__scroll tbody tr")).toHaveLength(6);
          expect(text()).toContain("The phone reaches each Mac on its own origin");
        });

        it("scrolls a wide table inside its own box", async () => {
          await read();
          const box = host.querySelector<HTMLElement>(".pk-doc__scroll")!;
          expect(box.getAttribute("aria-label")).toBe("What each Mac answers");
          expect(box.tabIndex).toBe(0);
          expect(box.querySelector("table")).not.toBeNull();
        });

        it("draws its sketch live, not as a preview", async () => {
          await read();
          await vi.waitFor(() =>
            expect(host.querySelector(".pk-doc__page figure.armada-sketch")).toBeTruthy(),
          );
          const figure = host.querySelector(".pk-doc__page figure.armada-sketch")!;
          expect(figure.hasAttribute("inert")).toBe(false);
          expect(figure.getAttribute("data-preview")).toBeNull();
        });

        it("approves with the revision it read", async () => {
          await read();
          await press(
            Array.from(host.querySelectorAll(".pk-doc__answers button")).find(
              (one) => one.textContent === "Approve",
            ),
          );
          await vi.waitFor(() =>
            expect(to("/api/sessions/document/answer")).toEqual([
              {
                session_id: "f1",
                document: `document:${PHONE_REACH}`,
                revision: "2",
                how: "approved",
              },
            ]),
          );
        });

        it("says the Job revised the page where Fleet refuses by `fleet.document_revised`", async () => {
          await read();
          held(PHONE_REACH).detail!["revision"] = "3";
          await press(
            Array.from(host.querySelectorAll(".pk-doc__answers button")).find(
              (one) => one.textContent === "Approve",
            ),
          );
          await vi.waitFor(() =>
            expect(text()).toContain("The Job revised this page. The new revision is shown."),
          );
        });
      });

      it("reads the Job again and says it was revised, on `fleet.document_revised`", async () => {
        await opened();
        held(SLOT_POOL).detail!["revision"] = "3";
        await press(inCard(POOL_CARD, "Approve"));
        await vi.waitFor(() =>
          expect(text()).toContain("The Job revised this page. The new revision is shown."),
        );
        await vi.waitFor(() => expect(card("Document: Slot pool, Revision 3")).toBeTruthy());
      });

      it("shows Fleet's own message on a 422, and does not say it changed", async () => {
        await opened();
        const detail = held(SLOT_POOL).detail!;
        detail["settled"] = "approved";
        detail["settled_at"] = new Date().toISOString();
        await press(inCard(POOL_CARD, "Approve"));
        await vi.waitFor(() => expect(text()).toContain("that page is not waiting on an answer"));
        expect(text()).not.toContain("The Job revised this page.");
      });
    });

    describe("a sketch", () => {
      it("approves from its card with the drawing the ledger holds", async () => {
        await opened();
        await press(inCard("Sketch: Phone home", "Approve"));
        await vi.waitFor(() =>
          expect(to("/api/sessions/sketch/answer")).toEqual([
            {
              session_id: "f1",
              sketch: "sketch:Phone home",
              drawn: "phone-home-1",
              how: "approved",
            },
          ]),
        );
      });

      it("sends back from its card with a note", async () => {
        await opened();
        await press(inCard("Sketch: Phone home", "Send back"));
        await note("Put the card first.");
        await press(inCard("Sketch: Phone home", "Send back"));
        await vi.waitFor(() =>
          expect(to("/api/sessions/sketch/answer")).toEqual([
            {
              session_id: "f1",
              sketch: "sketch:Phone home",
              drawn: "phone-home-1",
              how: "sent_back",
              words: "Put the card first.",
            },
          ]),
        );
      });

      it("reads the Job again and says it drew it again, then answers the new drawing", async () => {
        await opened();
        await press(inCard("Sketch: Slot layout", "Approve"));
        await vi.waitFor(() =>
          expect(text()).toContain("The Job drew this again. The new drawing is shown."),
        );
        await vi.waitFor(() =>
          expect(card("Sketch: Slot layout")?.textContent).toContain("Idle given back"),
        );
        await press(inCard("Sketch: Slot layout", "Approve"));
        await vi.waitFor(() => expect(to("/api/sessions/sketch/answer")).toHaveLength(2));
        expect(
          to("/api/sessions/sketch/answer").map(
            (one) =>
              (
                one as {
                  drawn: string;
                }
              ).drawn,
          ),
        ).toEqual([
          "slot-layout-1",
          "slot-layout-2",
        ]);
      });

      it("shows Fleet's own message on a 422, and does not say it changed", async () => {
        await opened();
        const detail = held("sketch:Phone home").detail!;
        detail["settled"] = "approved";
        detail["settled_at"] = new Date().toISOString();
        await press(inCard("Sketch: Phone home", "Approve"));
        await vi.waitFor(() => expect(text()).toContain("that page is not waiting on an answer"));
        expect(text()).not.toContain("The Job drew this again.");
      });
    });

    describe("a proposed workflow change", () => {
      const NAME = "Check before every walk";
      const item = "draft:f1-10";
      const answers = () => to("/api/sessions/waiting/answer");

      it("is dropped with Dismiss, which answers its item with the second choice", async () => {
        await opened();
        await press(inGroup(NAME, "Dismiss"));
        await vi.waitFor(() =>
          expect(answers()).toEqual([
            {
              session_id: "f1",
              item_id: item,
              choice: 1,
            },
          ]),
        );
        await vi.waitFor(() => expect(group(NAME)).toBeUndefined());
      });

      it("is approved with the first choice", async () => {
        await opened();
        await press(inGroup(NAME, "Approve"));
        await vi.waitFor(() =>
          expect(answers()).toEqual([
            {
              session_id: "f1",
              item_id: item,
              choice: 0,
            },
          ]),
        );
      });

      it("is sent back with the owner's note", async () => {
        await opened();
        await press(inGroup(NAME, "Send back"));
        await typeInto(field("Note"), "Run it after Check, not before.");
        await press(inGroup(NAME, "Send back"));
        await vi.waitFor(() =>
          expect(answers()).toEqual([
            {
              session_id: "f1",
              item_id: item,
              text: "Run it after Check, not before.",
            },
          ]),
        );
      });

      it("is not drawn off its row alone, where Fleet no longer lists the item", async () => {
        f1().session.waiting_for = f1().session.waiting_for!.filter((one) => one.id !== item);
        await open(F1);
        await vi.waitFor(() => expect(text()).toContain("One change to the build workflow"));
        expect(group(NAME)).toBeUndefined();
      });

      it("is the only draft with a Dismiss: an ordinary draft has none", async () => {
        await open(`/jobs/${encodeURIComponent(LAPTOP)}/b3`);
        await vi.waitFor(() => expect(text()).toContain("Push payload carries the Job id"));
        expect(button("Approve")).toBeTruthy();
        expect(button("Send back")).toBeTruthy();
        expect(button("Dismiss")).toBeUndefined();
      });
    });

    describe("Press Start, and a Fleet call", () => {
      const START = 'Press Start on "Split the pool crate"';
      const CALL = "main went red after this Job's last push";

      it("opens the Job it names, and offers no Dismiss", async () => {
        await opened();
        expect(inGroup(START, "Dismiss")).toBeUndefined();
        await press(inGroup(START, "Open"));
        await vi.waitFor(() =>
          expect(window.location.pathname).toBe(`/drafts/${encodeURIComponent(STUDIO)}/d1`),
        );
        expect(posted[STUDIO]).toEqual([]);
      });

      it("dismisses a Fleet call through Fleet's dismiss", async () => {
        await opened();
        await press(inGroup(CALL, "Dismiss"));
        await vi.waitFor(() =>
          expect(to("/api/sessions/waiting/dismiss")).toEqual([
            {
              session_id: "f1",
              item_id: "call:1",
            },
          ]),
        );
        await vi.waitFor(() => expect(group(CALL)).toBeUndefined());
      });
    });

    describe("a walk, and a question in one", () => {
      const onMac = () => Array.from(host.querySelectorAll<HTMLElement>(".pk-onmac"));

      it("is a line each, saying to open it on the Mac", async () => {
        await opened();
        expect(
          onMac().map((one) => [
            one.querySelector(".pk-onmac__says")?.textContent,
            one.querySelector(".pk-onmac__where")?.textContent,
          ]),
        ).toEqual([
          [
            "Pool band",
            "Open on your Mac",
          ],
          [
            "Does the band read as one line or two?",
            "Open on your Mac",
          ],
        ]);
      });

      it("ends the thread, and takes no answer", async () => {
        await opened();
        const [walk, question] = onMac();
        const last = Array.from(host.querySelectorAll(".pk-ask, .pk-card, .pk-onmac")).at(-1);
        expect(last).toBe(question);
        expect(
          walk!.compareDocumentPosition(question!) & Node.DOCUMENT_POSITION_FOLLOWING,
        ).toBeTruthy();
        expect(question!.querySelector("textarea, button")).toBeNull();
        expect(
          group("Does the band read as one line or two?")?.querySelector("textarea"),
        ).toBeNull();
        expect(posted[STUDIO]).toEqual([]);
      });
    });

    describe("a browser's question", () => {
      const NAME = "localhost wants to use your camera";
      it.each([
        [
          "Allow",
          0,
        ],
        [
          "Block",
          1,
        ],
      ])("is answered with %s, the choice at %s, and no words", async (label, choice) => {
        await opened();
        expect(group(NAME)?.querySelector("textarea")).toBeNull();
        await press(
          Array.from(group(NAME)?.querySelectorAll('[aria-label="Options"] button') ?? []).find(
            (one) => one.textContent?.startsWith(label),
          ),
        );
        await vi.waitFor(() =>
          expect(to("/api/sessions/waiting/answer")).toEqual([
            {
              session_id: "f1",
              item_id: "browser_ask:tab-1",
              choice,
            },
          ]),
        );
      });
    });
  });
});

describe("a Job that waits on another", {
  timeout: 30_000,
}, () => {
  const WAITS = `/jobs/${encodeURIComponent(STUDIO)}/w1`;
  const APPROVED = `/jobs/${encodeURIComponent(LAPTOP)}/w2`;
  const facts = () =>
    Object.fromEntries(
      Array.from(host.querySelectorAll(".pk-facts__row")).map((row) => [
        row.querySelector("dt")?.textContent,
        row.querySelector("dd")?.textContent,
      ]),
    );
  const select = (label: string) =>
    host.querySelector<HTMLSelectElement>(`select[aria-label="${label}"]`);
  const pick = async (label: string, value: string) => {
    await act(async () => {
      const element = select(label)!;
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(
        element,
        value,
      );
      element.dispatchEvent(
        new Event("change", {
          bubbles: true,
        }),
      );
    });
  };
  const changes = (origin: string) =>
    posted[origin]!.filter((one) => one.path === "/api/sessions/change_waiting").map(
      (one) => one.body,
    );

  it("draws its brief, the Job it waits on, and when it starts and from where, and no thread or box", async () => {
    await open(WAITS);
    await vi.waitFor(() => expect(button("Start now")).toBeTruthy());
    expect(facts()).toMatchObject({
      "Waits on": "Plan the phone screens",
      Needs: "The fields the phone reads for a slot",
      "Starts from": "main, after the merge",
    });
    expect(facts()["Brief"]).toContain("Draw the slot pool on the phone");
    expect(select("Starts when")!.value).toBe("lands");
    expect(button("Approve")!.getAttribute("aria-pressed")).toBe("false");
    expect(host.querySelector('[aria-label^="Starts when #3232 lands"]')).toBeTruthy();
    expect(host.querySelector(".armada-session-composer")).toBeNull();
  });

  it("opens the Job it waits on", async () => {
    await open(WAITS);
    await vi.waitFor(() => expect(button("Plan the phone screens")).toBeTruthy());
    await press(button("Plan the phone screens"));
    expect(window.location.pathname).toBe(`/jobs/${encodeURIComponent(STUDIO)}/p1`);
  });

  it("approves it, and a second press takes the approval back", async () => {
    await open(WAITS);
    await vi.waitFor(() => expect(button("Approve")).toBeTruthy());
    await press(button("Approve"));
    await vi.waitFor(() => expect(button("Approved")).toBeTruthy());
    expect(button("Approved")!.getAttribute("aria-pressed")).toBe("true");
    await press(button("Approved"));
    await vi.waitFor(() => expect(button("Approve")).toBeTruthy());
    expect(changes(STUDIO)).toEqual([
      {
        waiting_session: "w1",
        held: false,
      },
      {
        waiting_session: "w1",
        held: true,
      },
    ]);
  });

  it("starts now: one not approved opens as a draft, one approved opens its thread", async () => {
    await open(WAITS);
    await vi.waitFor(() => expect(button("Start now")).toBeTruthy());
    await press(button("Start now"));
    await vi.waitFor(() =>
      expect(window.location.pathname).toBe(`/drafts/${encodeURIComponent(STUDIO)}/w1`),
    );
    expect(posted[STUDIO]!.find((one) => one.path === "/api/sessions/start_waiting")?.body).toEqual(
      {
        session_id: "w1",
      },
    );
    await act(async () => root?.unmount());
    await open(APPROVED);
    await vi.waitFor(() => expect(button("Start now")).toBeTruthy());
    await press(button("Start now"));
    await vi.waitFor(() => expect(host.querySelector(".armada-session-composer")).toBeTruthy());
    expect(window.location.pathname).toBe(APPROVED);
  });

  it("changes when it starts, and a plan part only where the Job waited on has a plan", async () => {
    await open(WAITS);
    await vi.waitFor(() => expect(select("Starts when")).toBeTruthy());
    const plan = select("Starts when")!.querySelector<HTMLOptionElement>('option[value="plan"]')!;
    await vi.waitFor(() => expect(plan.disabled).toBe(true));
    await pick("Starts when", "ends");
    await vi.waitFor(() => expect(select("Starts when")!.value).toBe("ends"));
    expect(changes(STUDIO)).toEqual([
      {
        waiting_session: "w1",
        condition: {
          kind: "ends",
        },
      },
    ]);
    await act(async () => root?.unmount());
    await open(APPROVED);
    await vi.waitFor(() =>
      expect(Array.from(select("Plan part")?.options ?? []).map((one) => one.textContent)).toEqual([
        "Group: Rename in the crate",
        "Task: Rename Pool to Slots",
        "Task: Rename the tests",
        "Group: Rename in Bridge",
        "Task: Rename the labels and tooltips",
        "Task: Walk the Slots page",
        "Group: Rename in the docs",
        "Task: Rename the concept page",
      ]),
    );
    await pick("Plan part", "t2");
    await vi.waitFor(() => expect(select("Plan part")!.value).toBe("t2"));
    expect(changes(LAPTOP)).toEqual([
      {
        waiting_session: "w2",
        condition: {
          kind: "task_done",
          task: "t2",
        },
      },
    ]);
  });

  it("answers what Fleet asks of it on the same screen", async () => {
    await open(APPROVED);
    const asked = () =>
      host.querySelector('[role="group"][aria-label="How should this work be delivered?"]');
    await vi.waitFor(() => expect(asked()).toBeTruthy());
    await press(
      Array.from(asked()!.querySelectorAll("button")).find((one) =>
        one.textContent?.startsWith("Auto land"),
      ),
    );
    await vi.waitFor(() => expect(asked()).toBeNull());
    expect(
      posted[LAPTOP]!.find((one) => one.path === "/api/sessions/waiting/answer")?.body,
    ).toEqual({
      session_id: "w2",
      item_id: "delivery",
      choice: 0,
    });
    expect(button("Start now")).toBeTruthy();
  });

  it("offers End job alone in its menu", async () => {
    await open(WAITS);
    await vi.waitFor(() => expect(button("More")).toBeTruthy());
    await press(button("More"));
    await vi.waitFor(() =>
      expect(
        Array.from(document.body.querySelectorAll('[role="menuitem"]')).map((one) =>
          one.textContent?.trim(),
        ),
      ).toEqual([
        "End job",
      ]),
    );
  });
});

describe("a plan's card on the phone", {
  timeout: 30_000,
}, () => {
  const PLAN = `/jobs/${encodeURIComponent(LAPTOP)}/b1`;
  const OWN = "Rename in the docs";
  const TASKS = [
    "Rename Pool to Slots",
    "Rename the tests",
    "Rename the labels and tooltips",
    "Walk the Slots page",
    "Rename the concept page",
  ];
  /** The card a group that goes to a Job of its own is drawn as, not started yet. */
  const placeholders = () => host.querySelectorAll(`[role="group"][aria-label^="${OWN}"]`);
  const gives = () =>
    Array.from(host.querySelectorAll('[aria-label="What each group gives"] li')).map(
      (one) => one.textContent,
    );

  it("lists each task's words under its group once Details is pressed, and not before", async () => {
    await open(PLAN);
    await vi.waitFor(() => expect(button("Details")).toBeTruthy());
    for (const words of TASKS) expect(text()).not.toContain(words);
    await press(button("Details"));
    for (const words of TASKS) expect(text()).toContain(words);
  });

  it("draws the group that goes to a Job of its own as one placeholder card, in Details", async () => {
    await open(PLAN);
    await vi.waitFor(() => expect(button("Details")).toBeTruthy());
    expect(placeholders()).toHaveLength(0);
    await press(button("Details"));
    expect(placeholders()).toHaveLength(1);
    expect(placeholders()[0]!.getAttribute("aria-label")).toBe(`${OWN}, not started yet`);
  });

  it("lists what the own-Job group gives with the others', and under its card with its tasks", async () => {
    await open(PLAN);
    await vi.waitFor(() => expect(button("Details")).toBeTruthy());
    expect(gives()).toEqual([
      "The crate and its tests say slot",
      "Every screen says slot where it said pool",
      "The docs say slot, with the old word named once",
    ]);
    await press(button("Details"));
    const run = placeholders()[0]!.closest("section")!;
    expect(run.textContent).toContain("The docs say slot, with the old word named once");
    expect(run.textContent).toContain("Rename the concept page");
    expect(run.textContent).not.toContain("Rename Pool to Slots");
  });
});

describe("a Job longer than one read", {
  timeout: 30_000,
}, () => {
  const LONG = `/jobs/${encodeURIComponent(LAPTOP)}/b4`;
  const reads = () =>
    sent
      .filter((one) => one.origin === LAPTOP && one.target.startsWith("/api/sessions/one?"))
      .map((one) => one.target);

  it("opens on its newest rows, not its first, asking for 200 of them", async () => {
    await open(LONG);
    await vi.waitFor(() => expect(text()).toContain("Turn 448 of 450."));
    expect(text()).not.toContain("Turn 1 of 450.");
    expect(reads()[0]).toBe("/api/sessions/one?session_id=b4&last=200");
    expect(button("Show earlier")).toBeTruthy();
  });

  it("reaches its first row by pressing Show earlier, and the press goes once it has", async () => {
    await open(LONG);
    await vi.waitFor(() => expect(text()).toContain("Turn 448 of 450."));
    await vi.waitFor(
      async () => {
        const more = button("Show earlier");
        if (more !== undefined) await press(more);
        expect(button("Show earlier")).toBeUndefined();
      },
      {
        timeout: 20_000,
        interval: 50,
      },
    );
    expect(text()).toContain("Turn 1 of 450.");
    expect(text()).toContain("Turn 448 of 450.");
    // Each page asks for the rows before the first one held.
    expect(reads()).toContain("/api/sessions/one?session_id=b4&last=200&before=b4-250");
    expect(reads()).toContain("/api/sessions/one?session_id=b4&last=200&before=b4-50");
  });
});

describe("the lists stay fresh", {
  timeout: 30_000,
}, () => {
  const toFake = [
    "setTimeout",
    "clearTimeout",
    "setInterval",
    "clearInterval",
  ] as const;
  const settle = () => realSleep(40);
  const until = async (ok: () => boolean) => {
    for (let waited = 0; waited < 400 && !ok(); waited++) await realSleep(5);
    expect(ok()).toBe(true);
  };
  const connects = (origin = STUDIO) =>
    sent.filter((one) => one.origin === origin && one.target === "/api/live").length;
  const listReads = (origin = STUDIO) =>
    sent
      .filter((one) => one.origin === origin && /^\/api\/(needs|sessions(\?.*)?$)/.test(one.target))
      .map((one) => one.target);
  const begin = async () => {
    vi.useFakeTimers({
      toFake: [
        ...toFake,
      ],
    });
    const fleet = await import("./fleet");
    await fleet.start();
    await until(() => lives.some((live) => live.origin === STUDIO));
    return fleet;
  };

  it("reads each Mac's live Jobs once on open, and takes Needs you from the waiting ones", async () => {
    const fleet = await begin();
    await until(() => fleet.macOf(STUDIO)!.live.length > 0 && fleet.macOf(LAPTOP)!.live.length > 0);
    await settle();
    expect(listReads(STUDIO)).toEqual([
      "/api/sessions",
    ]);
    expect(listReads(LAPTOP)).toEqual([
      "/api/sessions",
    ]);
    // The Mac's own list holds a draft too, which is waiting on its first message; Home sets it apart.
    expect(fleet.macOf(STUDIO)!.needs.map((one) => one.id)).toEqual([
      "f1",
      "d1",
      "a1",
      "r1",
      "l1",
      "s1",
      "o1",
      "c2",
    ]);
    expect(fleet.macOf(LAPTOP)!.needs.map((one) => one.id)).toEqual([
      "b1",
      "b3",
    ]);
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
    vi.useFakeTimers({
      toFake: [
        ...toFake,
      ],
    });
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
    Object.defineProperty(document, "visibilityState", {
      value: "hidden",
      configurable: true,
    });
    document.dispatchEvent(new Event("visibilitychange"));
    await settle();
    expect(connects()).toBe(1);
    Object.defineProperty(document, "visibilityState", {
      value: "visible",
      configurable: true,
    });
    document.dispatchEvent(new Event("visibilitychange"));
    await until(() => first.aborted && connects() === 2 && connects(LAPTOP) === 2);
    await until(() => listReads().length === 2);
    window.dispatchEvent(new Event("online"));
    await until(() => connects() === 3);
    await until(() => listReads().length === 3);
  });

  describe("coming back to the app", () => {
    const seen = (state: "hidden" | "visible") => {
      Object.defineProperty(document, "visibilityState", {
        value: state,
        configurable: true,
      });
      document.dispatchEvent(new Event("visibilitychange"));
    };
    const away = async (path: string, ms: number) => {
      await begin();
      window.history.replaceState(null, "", path);
      const left = Date.now();
      // Held across both, since the app reads the clock itself: a tick after `left` would make the time away too short.
      const clock = vi.spyOn(Date, "now").mockReturnValue(left);
      seen("hidden");
      clock.mockReturnValue(left + ms);
      seen("visible");
      clock.mockRestore();
      return window.location.pathname;
    };
    const JOB = `/jobs/${encodeURIComponent(STUDIO)}/a2`;

    it("opens on the list after five minutes on a Job", async () => {
      expect(await away(JOB, 5 * 60_000)).toBe("/");
    });

    it("stays on the Job after a quick switch", async () => {
      expect(await away(JOB, 5 * 60_000 - 1)).toBe(JOB);
    });

    it("stays on Dispatch however long, since he may be typing there", async () => {
      expect(await away("/dispatch", 60 * 60_000)).toBe("/dispatch");
    });
  });
});

describe("the Operator on the phone", {
  timeout: 30_000,
}, () => {
  const field = () => host.querySelector<HTMLTextAreaElement>('textarea[aria-label="Message"]');
  const typeInto = async (value: string) => {
    expect(field()).toBeTruthy();
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(
        field(),
        value,
      );
      field()!.dispatchEvent(
        new Event("input", {
          bubbles: true,
        }),
      );
    });
  };
  const choose = async (repository: string) => {
    const select = host.querySelector<HTMLSelectElement>('select[aria-label="Repository"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(
        select,
        repository,
      );
      select.dispatchEvent(
        new Event("change", {
          bubbles: true,
        }),
      );
    });
  };
  const dispatches = (origin: string) =>
    posted[origin]!.filter((one) => one.path === "/api/sessions/dispatch").map((one) => one.body);
  const sends = () => rows("Sends");
  beforeEach(() => {
    asleep = false;
  });
  /** The laptop answers nothing, and the studio is the Mac a send to it goes through. The phone kept the laptop's list from before. */
  let asleep = false;
  const laptopAsleep = () => {
    asleep = true;
    unreachable.add(LAPTOP);
    delete macs[LAPTOP];
    macs[STUDIO]!.asleep = [
      "laptop",
    ];
  };
  const openOperator = async () => {
    localStorage.clear();
    if (asleep)
      localStorage.setItem(
        "pocket:operator:repositories",
        JSON.stringify({
          [LAPTOP]: [
            "store",
          ],
        }),
      );
    await open("/operator");
    await vi.waitFor(() =>
      expect(host.querySelector('select[aria-label="Repository"] option')).toBeTruthy(),
    );
  };

  it("sends to the Mac it is paired with without naming one, with the kind as a hint", async () => {
    await openOperator();
    await typeInto("Cut the pool to four");
    await press(button("Feature"));
    await press(button("Send"));
    await vi.waitFor(() => expect(dispatches(STUDIO)).toHaveLength(1));
    expect(dispatches(STUDIO)[0]).toEqual({
      text: "Cut the pool to four",
      repository: "armada",
      kind: "feature",
    });
    expect(dispatches(LAPTOP)).toEqual([]);
    expect(field()!.value).toBe("");
  });

  it("sends to an awake other Mac at its own origin, with no `mac`, and hears what became of it on its stream", async () => {
    await openOperator();
    await choose("store");
    await typeInto("Rename the footer links");
    await press(button("Bug"));
    await press(button("Send"));
    await vi.waitFor(() => expect(dispatches(LAPTOP)).toHaveLength(1));
    expect(dispatches(LAPTOP)[0]).toEqual({
      text: "Rename the footer links",
      repository: "store",
      kind: "bug",
    });
    expect(dispatches(STUDIO)).toEqual([]);
    // The laptop holds the send, and says what the Operator did with it a moment later.
    await vi.waitFor(() => expect(sends().join()).toContain("Started"), {
      timeout: 5_000,
    });
    expect(sends().join()).toContain("Rename the footer links");
  });

  it("shows a send to a Mac that does not answer as waiting, and Take back posts it and returns the words", async () => {
    laptopAsleep();
    await openOperator();
    await choose("store");
    await vi.waitFor(() => expect(button("laptop, asleep")).toBeTruthy());
    await typeInto("Move the pricing table above the fold");
    await press(button("Feature"));
    await press(button("Send"));
    await vi.waitFor(() => expect(sends().join()).toContain("Waiting"));
    expect(dispatches(STUDIO)[0]).toMatchObject({
      repository: "store",
      kind: "feature",
      mac: "laptop",
    });
    expect(sends().join()).toContain("This sends when laptop wakes.");
    const kept = macs[STUDIO]!.queued![0]!;
    expect(field()!.value).toBe("");
    await press(button("Take back"));
    await vi.waitFor(() => expect(sends()).toEqual([]));
    expect(posted[STUDIO]!.find((one) => one.path === "/api/queued/take_back")).toEqual({
      path: "/api/queued/take_back",
      body: {
        id: kept.id,
      },
    });
    expect(field()!.value).toBe("Move the pricing table above the fold");
    expect(macs[STUDIO]!.queued).toEqual([]);
  });

  it("says a send through another Mac is in no shared project on the composer, and leaves the words in the box", async () => {
    laptopAsleep();
    macs[STUDIO]!.unshared = true;
    await openOperator();
    await choose("store");
    await typeInto("Rename the footer links");
    await press(button("Bug"));
    await press(button("Send"));
    await vi.waitFor(() =>
      expect(host.querySelector('[role="alert"]')?.textContent).toContain(
        "not in a project shared",
      ),
    );
    expect(dispatches(STUDIO)[0]).toMatchObject({
      mac: "laptop",
    });
    expect(field()!.value).toBe("Rename the footer links");
    expect(sends()).toEqual([]);
    expect(host.querySelector('button[aria-pressed="true"]')?.textContent).toBe("Bug");
  });

  it("says why a waiting send cannot be taken back, in Fleet's words, and keeps it", async () => {
    laptopAsleep();
    await openOperator();
    await choose("store");
    await typeInto("Rename the footer links");
    await press(button("Send"));
    await vi.waitFor(() => expect(button("Take back")).toBeTruthy());
    // It was sent from the other side in the meantime.
    macs[STUDIO]!.queued = [];
    await press(button("Take back"));
    await vi.waitFor(() => expect(text()).toContain("Nothing waits under that id."));
    expect(sends().join()).toContain("Waiting");
    expect(field()!.value).toBe("");
  });

  it("draws what already waits when the page opens, a refused one with Take back and the Mac's words", async () => {
    laptopAsleep();
    macs[STUDIO]!.queued = [
      {
        id: "q9",
        mac: "laptop-id",
        act: {
          kind: "dispatch",
          body: {
            request: "Rename the footer links",
            kind: "bug",
            then: [
              {
                message: {
                  text: "Rename the footer links",
                },
              },
            ],
          },
        },
        since: NOW - 60_000,
        refused: "laptop runs an older build than this Mac, so it took nothing.",
      },
    ];
    await openOperator();
    await vi.waitFor(() => expect(sends().join()).toContain("Refused"));
    expect(sends().join()).toContain(
      "laptop runs an older build than this Mac, so it took nothing.",
    );
    await press(button("Take back"));
    await vi.waitFor(() => expect(field()!.value).toBe("Rename the footer links"));
    expect(posted[STUDIO]![0]!.body).toEqual({
      id: "q9",
    });
  });

  it("says an asked dispatch's question is on Bridge", async () => {
    await openOperator();
    await typeInto("Add the export?");
    await press(button("Send"));
    await vi.waitFor(() => expect(sends().join()).toContain("Asked"), {
      timeout: 5_000,
    });
    expect(sends().join()).toContain("The Operator asked about this. Its question is on Bridge.");
  });

  it("shows a refusal of the Gateway as a refused row, and Take back returns the words without a call", async () => {
    await openOperator();
    await typeInto("Rename the footer links");
    // The Gateway refuses a repository its Mac does not list.
    macs[STUDIO]!.repositories = [];
    await press(button("Send"));
    await vi.waitFor(() => expect(sends().join()).toContain("Refused"));
    expect(sends().join()).toContain(
      "Armada does not know that repository. Pick one from the list.",
    );
    await press(button("Take back"));
    await vi.waitFor(() => expect(field()!.value).toBe("Rename the footer links"));
    expect(sends()).toEqual([]);
    expect(posted[STUDIO]!.some((one) => one.path === "/api/queued/take_back")).toBe(false);
  });
});
