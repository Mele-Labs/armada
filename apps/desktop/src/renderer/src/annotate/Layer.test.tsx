// The layer in a real window, on the case that broke it: an element as tall as
// the window. The card is `position: fixed`, so the window is the frame, and
// what this asserts is that every edge of the card is inside it — measured,
// because the defect was purely geometric and nothing about the markup showed it.

import { StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { SendSessionNote } from "@armada/protocol";
import type { RingedNotes } from "../../../shared/api/sessions";

import type { Annotation, AnnotationStatus } from "../../../shared/annotations";
import "../styles/index.css";
import { Layer, type LayerFleet } from "./Layer";
import type { Sink } from "./sink";

/** The owner's window in the screenshot, and a short one. */
const OWNER = { width: 1324, height: 922 };
const SHORT = { width: 1324, height: 700 };

/** Notes the layer reads, saves back to and deletes from, held in memory. */
function sinkOf(notes: Annotation[] = [], shots = new Map<string, ArrayBuffer>()): Sink {
  const held = new Map(notes.map((note) => [note.id, note]));
  return {
    via: "dev server",
    list: async () => [...held.values()],
    save: async (note) => {
      held.set(note.id, note);
    },
    remove: async (id) => {
      held.delete(id);
      shots.delete(id);
    },
    root: async () => null,
    capture: async () => null,
    saveShot: async (id, png) => {
      shots.set(id, png);
    },
    shot: async (id) => shots.get(id) ?? null,
  };
}

const sink = sinkOf();

let mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(async () => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  document.querySelectorAll("[data-tall], [data-noted]").forEach((one) => one.remove());
  sessionStorage.removeItem("armada.annotate.on");
  await page.viewport(1440, 900);
});

/** The layer already on, as it comes up after ⌥⌘A and a reload. */
async function annotating(sink: Sink, fleet?: LayerFleet, openSession?: (id: string) => void, tell?: (sentence: string) => void): Promise<void> {
  sessionStorage.setItem("armada.annotate.on", "1");
  const host = document.createElement("div");
  host.setAttribute("data-armada-annotate", "");
  document.body.append(host);
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <Layer sink={sink} {...(fleet === undefined ? {} : { fleet })} {...(openSession === undefined ? {} : { openSession })} {...(tell === undefined ? {} : { tell })} />
    </StrictMode>,
  );
  mounted.push({ root, host });
  await expect.element(page.getByRole("status")).toBeVisible();
}

/**
 * A Job settings sheet, near enough: fixed, and as tall as the window less a
 * hair. This is the element the owner picked, and the one no side has room beside.
 */
function tallSheet(): HTMLElement {
  const sheet = document.createElement("div");
  sheet.setAttribute("data-tall", "");
  sheet.setAttribute("role", "dialog");
  sheet.setAttribute("aria-label", "Job settings");
  sheet.style.cssText = "position:fixed;top:2%;bottom:2%;left:30%;width:30%";
  document.body.append(sheet);
  return sheet;
}

const rectOf = (locator: ReturnType<typeof page.getByRole>): DOMRect =>
  (locator.element() as HTMLElement).getBoundingClientRect();

/** Inside the window on every edge, to the pixel. */
function insideTheWindow(what: string, rect: DOMRect): void {
  expect(rect.top, `${what}: top edge`).toBeGreaterThanOrEqual(0);
  expect(rect.left, `${what}: leading edge`).toBeGreaterThanOrEqual(0);
  expect(rect.bottom, `${what}: bottom edge`).toBeLessThanOrEqual(window.innerHeight);
  expect(rect.right, `${what}: trailing edge`).toBeLessThanOrEqual(window.innerWidth);
  expect(rect.height, `${what}: drawn at all`).toBeGreaterThan(0);
}

async function cardOnTheTallSheet(size: { width: number; height: number }) {
  await page.viewport(size.width, size.height);
  await annotating(sink);
  tallSheet().click();
  const card = page.getByRole("dialog", { name: "New note" });
  await expect.element(card).toBeVisible();
  // The layer re-places the card off a measurement it takes each frame.
  await new Promise((settled) => requestAnimationFrame(() => requestAnimationFrame(settled)));
  return card;
}

for (const size of [OWNER, SHORT]) {
  test(`a note on an element as tall as a ${size.width}×${size.height} window is drawn whole inside it`, async () => {
    const card = await cardOnTheTallSheet(size);
    const rect = rectOf(card);
    insideTheWindow("card", rect);

    // What the owner could not see: the text box he was typing into, and the
    // two buttons, which were clipped at the top edge of the window.
    for (const name of ["Cancel", "Save note"]) {
      const button = rectOf(card.getByRole("button", { name }));
      insideTheWindow(name, button);
      expect(button.top).toBeGreaterThanOrEqual(rect.top);
      expect(button.bottom).toBeLessThanOrEqual(rect.bottom);
    }
    const box = rectOf(card.getByRole("textbox", { name: "Note" }));
    insideTheWindow("the text box", box);
    expect(box.top).toBeGreaterThanOrEqual(rect.top);
    expect(box.bottom).toBeLessThanOrEqual(rect.bottom);

    // The layer's own bar is pinned over everything; a card under it is on
    // screen and still unreadable.
    expect(rect.bottom).toBeLessThanOrEqual(rectOf(page.getByRole("status")).top);
  });
}

test("a window too short for the card gives it its own scroll rather than an edge to fall off", async () => {
  const card = await cardOnTheTallSheet({ width: 1324, height: 260 });
  const rect = rectOf(card);
  insideTheWindow("card", rect);

  const element = card.element() as HTMLElement;
  expect(element.scrollHeight).toBeGreaterThan(element.clientHeight);
  element.scrollTop = element.scrollHeight;
  expect(rectOf(card.getByRole("button", { name: "Save note" })).bottom).toBeLessThanOrEqual(rect.bottom);

  // The placement settles: a card capped by the window must not flip between
  // two sides frame after frame as it is measured, capped, and measured again.
  await new Promise((settled) => requestAnimationFrame(() => requestAnimationFrame(settled)));
  const again = rectOf(card);
  expect([again.x, again.y, again.width, again.height]).toEqual([rect.x, rect.y, rect.width, rect.height]);
});

// A batch of notes has been fixed and the layer is turned on again. What the
// owner saw was every fixed note still pinned to its screen, numbered, so a new
// note came up as 9 while one was open. A done note is now out of the drawing
// and out of the numbering, and the bar is where it is said how many there are.

/** An element a note points at, found again by the selector the note carries. */
function noted(name: string, top: number): string {
  const element = document.createElement("button");
  element.setAttribute("data-noted", name);
  element.textContent = name;
  // A share of the window, as the tall sheet above is: the pins only have to
  // land clear of one another, and a length literal is off-contract here too.
  element.style.cssText = `position:fixed;left:20%;top:${top}%;width:15%;height:4%`;
  document.body.append(element);
  return `[data-noted="${name}"]`;
}

/** A saved note on that element, written at a time the numbering sorts by. */
function note(name: string, status: AnnotationStatus, at: string, top: number): Annotation {
  return {
    id: `2026091${at}`,
    status,
    text: `${name}: what the owner said`,
    component: "JobRowStacked",
    owners: ["ActiveJobsList", "Board"],
    ownersFrom: "parent",
    selector: noted(name, top),
    element: { tag: "button", text: name, label: null },
    screen: "Job Board",
    layer: null,
    location: "/",
    scenario: null,
    box: { x: 0, y: top, width: 0, height: 0 },
    window: { width: 1440, height: 900 },
    createdAt: `2026-09-1${at}T10:00:00.000Z`,
    updatedAt: `2026-09-1${at}T10:00:00.000Z`,
  };
}

/** Every pin drawn, as a person reads it: its number, or nothing where it has none. */
function pinned(): { name: string; reads: string }[] {
  return page
    .getByRole("button", { name: /^(Note \d+, open|Done note)/ })
    .elements()
    .map((one) => ({ name: one.getAttribute("aria-label") ?? "", reads: one.textContent ?? "" }));
}

/** Three notes, the middle one already fixed by an agent, on three live elements. */
const batch = (): Annotation[] => [
  note("first", "open", "5", 15),
  note("second", "done", "6", 30),
  note("third", "open", "7", 45),
];

/** ⌥⌘A, which is how the layer is put away and brought back. */
async function pressToggle(): Promise<void> {
  window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyA", metaKey: true, altKey: true, bubbles: true }));
  await new Promise((settled) => setTimeout(settled, 0));
}

test("a note marked done draws no pin, and the open ones are numbered 1 to n with no gap", async () => {
  await annotating(sinkOf(batch()));

  await expect.element(page.getByRole("button", { name: /^Note 1, open/ })).toBeVisible();
  expect(pinned()).toEqual([
    { name: "Note 1, open: first: what the owner said", reads: "1" },
    { name: "Note 2, open: third: what the owner said", reads: "2" },
  ]);

  // The count is still said, because the file is still there.
  await expect.element(page.getByRole("status")).toHaveTextContent("2 open, 1 done");
});

test("the bar shows the done notes again, unnumbered, and their cards still work", async () => {
  await annotating(sinkOf(batch()));
  const bar = page.getByRole("status");

  await bar.getByRole("button", { name: "Show done notes" }).click();
  const back = page.getByRole("button", { name: /^Done note/ });
  await expect.element(back).toBeVisible();
  expect(pinned()).toEqual([
    { name: "Note 1, open: first: what the owner said", reads: "1" },
    { name: "Done note: second: what the owner said", reads: "" },
    { name: "Note 2, open: third: what the owner said", reads: "2" },
  ]);

  // The card opens off the pin, and reopening from it puts the note back into
  // the numbering — at 2, where it was written, not at the end.
  await back.click();
  const card = page.getByRole("dialog", { name: "Note" });
  await expect.element(card).toHaveTextContent("second: what the owner said");
  await expect.element(card.getByRole("button", { name: "Delete" })).toBeVisible();
  await card.getByRole("button", { name: "Reopen" }).click();
  await expect.element(page.getByRole("button", { name: /^Note 2, open: second/ })).toBeVisible();
  await expect.element(bar).toHaveTextContent("3 open, 0 done");
});

test("putting the layer away puts the done notes away with it, and looking wrote nothing", async () => {
  const held = sinkOf(batch());
  await annotating(held);
  const bar = page.getByRole("status");

  await bar.getByRole("button", { name: "Show done notes" }).click();
  await expect.element(page.getByRole("button", { name: /^Done note/ })).toBeVisible();
  await bar.getByRole("button", { name: "Hide done notes" }).click();
  expect(page.getByRole("button", { name: /^Done note/ }).elements()).toHaveLength(0);

  // ⌥⌘A off and on again, which is how the notes are read from the files a
  // second time: the layer comes up on the open notes, as it did the first time.
  await pressToggle();
  await expect.element(bar).not.toBeInTheDocument();
  await pressToggle();
  await expect.element(page.getByRole("status")).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Show done notes" })).toBeVisible();
  expect(page.getByRole("button", { name: /^Done note/ }).elements()).toHaveLength(0);

  // Nothing about a note was written by looking: the files say what they said.
  expect((await held.list()).map((one) => one.status)).toEqual(["open", "done", "open"]);
});

// What the owner saw on 1 Oct 2026: one note saved twice at once, and the bar
// reporting the second save's failure though the note was written. The draft
// stays up until the save answers, so a second press of Save — or ⌘↩ held long
// enough to repeat — saved it again.
test("a note pressed to save twice while its save is out is saved once", async () => {
  const saved: Annotation[] = [];
  let answer = (): void => {};
  const slow: Sink = {
    ...sinkOf(),
    save: (one) => {
      saved.push(one);
      return new Promise<void>((resolve) => {
        answer = resolve;
      });
    },
  };
  await annotating(slow);
  document.querySelector<HTMLElement>(noted("target", 20))?.click();
  const card = page.getByRole("dialog", { name: "New note" });
  const text = card.getByRole("textbox", { name: "Note" });
  await text.fill("Save me once");

  const save = card.getByRole("button", { name: "Save note" });
  await save.click();
  await save.click();
  (text.element() as HTMLElement).dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", metaKey: true, bubbles: true }));
  expect(saved.map((one) => one.text)).toEqual(["Save me once"]);

  answer();
  await expect.element(card).not.toBeInTheDocument();
  expect(saved).toHaveLength(1);
});

// The picture a note keeps, taken once as it is saved, of the app under the layer.

/** The eight bytes a PNG starts with, and one more: what a capture answers here. */
const PNG = Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10, 0]).buffer;

/** A sink in Bridge, which captures, keeping its pictures in `shots`. */
const capturing = (shots: Map<string, ArrayBuffer>, capture: Sink["capture"] = async () => PNG): Sink => ({ ...sinkOf([], shots), via: "main", capture });

async function writeOn(target: string, text: string): Promise<void> {
  document.querySelector<HTMLElement>(target)?.click();
  const card = page.getByRole("dialog", { name: "New note" });
  await card.getByRole("textbox", { name: "Note" }).fill(text);
  await card.getByRole("button", { name: "Save note" }).click();
  await expect.element(card).not.toBeInTheDocument();
}

/** A PNG's signature and header, `width` by `height`, which is all the layer reads of a picture. */
function headerOf(width: number, height: number): ArrayBuffer {
  const bytes = new Uint8Array(24);
  bytes.set([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52]);
  new DataView(bytes.buffer).setUint32(16, width);
  new DataView(bytes.buffer).setUint32(20, height);
  return bytes.buffer;
}

test("a note's first save captures the whole window and keeps it beside the note, with the element's box on it in the picture's pixels", async () => {
  const shots = new Map<string, ArrayBuffer>();
  // A Retina window's picture: twice as wide as the window in CSS pixels.
  const picture = headerOf(window.innerWidth * 2, window.innerHeight * 2);
  const capture = vi.fn(async () => picture);
  const held = capturing(shots, capture);
  await annotating(held);
  const target = noted("target", 20);
  await writeOn(target, "Keep a picture of me");

  const [saved] = await held.list();
  expect(saved?.screenshot).toBe(`${saved?.id}.png`);
  expect(shots.get(saved!.id)).toBe(picture);
  expect(capture).toHaveBeenCalledTimes(1);
  expect(capture).toHaveBeenCalledWith({ x: 0, y: 0, width: window.innerWidth, height: window.innerHeight });
  const rect = document.querySelector(target)!.getBoundingClientRect();
  expect(saved?.ring).toEqual({ x: Math.round(rect.x * 2), y: Math.round(rect.y * 2), width: Math.round(rect.width * 2), height: Math.round(rect.height * 2), scale: 2 });

  // Marking it done rewrites the file and keeps the picture it named, taking no other.
  await countOf().click();
  await rowOf("Keep a picture of me").getByRole("button", { name: "Mark done" }).click();
  await vi.waitFor(async () => expect((await held.list())[0]?.status).toBe("done"));
  expect((await held.list())[0]?.screenshot).toBe(`${saved?.id}.png`);
  expect(capture).toHaveBeenCalledTimes(1);
});

test("a note whose picture is not kept is told, and saved without one", async () => {
  const told = vi.fn();
  const held: Sink = { ...capturing(new Map()), saveShot: async () => Promise.reject(new Error("disk full")) };
  await annotating(held, undefined, undefined, told);
  await writeOn(noted("target", 20), "Save me anyway");
  expect(told).toHaveBeenCalledWith("Saving the screenshot failed through main: disk full");
  const [saved] = await held.list();
  expect(saved?.text).toBe("Save me anyway");
  expect(saved).not.toHaveProperty("screenshot");
});

test("in a browser, where nothing captures, a note is saved with no picture and none is asked for", async () => {
  const capture = vi.fn(async () => null);
  const held: Sink = { ...sinkOf(), capture };
  await annotating(held);
  await writeOn(noted("target", 20), "No picture here");
  expect(capture).not.toHaveBeenCalled();
  expect((await held.list())[0]).not.toHaveProperty("screenshot");
});

/** Pixels of a PNG that are not the colour the app under the layer is painted. */
async function strayPixels(png: Blob): Promise<number> {
  const bitmap = await createImageBitmap(png);
  const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
  const context = canvas.getContext("2d")!;
  context.drawImage(bitmap, 0, 0);
  const { data } = context.getImageData(0, 0, bitmap.width, bitmap.height);
  let stray = 0;
  for (let at = 0; at < data.length; at += 4) {
    if (Math.abs(data[at]! - 0) > 8 || Math.abs(data[at + 1]! - 128) > 8 || Math.abs(data[at + 2]! - 0) > 8) stray += 1;
  }
  return stray;
}

const pngOf = (base64: string): Blob => new Blob([Uint8Array.from(atob(base64), (c) => c.charCodeAt(0))], { type: "image/png" });

// A real screenshot of the page stands in for main's capturePage, which shoots the window as
// painted, the layer with it. The app is one flat colour under the card and the bar, so a
// picture that holds any other pixel holds the layer.
test("the picture kept at save is of the app under the note, with none of the layer drawn over it", async () => {
  await page.viewport(800, 600);
  const app = document.createElement("div");
  app.setAttribute("data-noted", "app");
  app.style.cssText = "position:fixed;inset:0;background:rgb(0,128,0)";
  document.body.append(app);
  const shot = async (): Promise<ArrayBuffer> => (await pngOf(await page.screenshot({ element: app, save: false })).arrayBuffer());
  const shots = new Map<string, ArrayBuffer>();
  await annotating(capturing(shots, shot));

  app.click();
  const card = page.getByRole("dialog", { name: "New note" });
  await card.getByRole("textbox", { name: "Note" }).fill("The whole window is wrong");
  // The same shot with the layer up holds the card, the outline and the bar: the check can see them.
  expect(await strayPixels(new Blob([await shot()]))).toBeGreaterThan(1000);

  await card.getByRole("button", { name: "Save note" }).click();
  await expect.element(card).not.toBeInTheDocument();
  const [kept] = [...shots.values()];
  expect(kept).toBeDefined();
  expect(await strayPixels(new Blob([kept!]))).toBe(0);
  // And the layer is back once it is taken.
  await expect.element(page.getByRole("status")).toBeVisible();
  expect(getComputedStyle(document.querySelector(".armada-annotate")!).opacity).toBe("1");
});

// The bar's send: the notes as a message to a Session.

const sendable = (notes: Annotation[], shots?: Map<string, ArrayBuffer>): Sink => ({ ...sinkOf(notes, shots), via: "main", root: async () => "/Users/user/armada" });

const record = (id: string, title: string | undefined, change: object = {}) =>
  ({ id, title, state: "live", origin: "bridge", ...change }) as never;

/** A Fleet serving three Sessions, one from a terminal and one ended, and answering every send. `sent` is every note, each with its turn's Session; `rings` each note's ring. */
function served(refuse = false) {
  const sent: SendSessionNote[] = [];
  const rings: (RingedNotes["notes"][number]["ring"])[] = [];
  const fleet = {
    startSession: vi.fn(async (title?: string) => ({ ok: true, value: record("01NEWSESSION0000", title) }) as never),
    sendSessionMessage: vi.fn(async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }) as never),
    sendSessionNotes: vi.fn(async (send: RingedNotes) => {
      for (const one of send.notes) {
        sent.push({ session_id: send.session_id, ...one.note });
        rings.push(one.ring);
      }
      return refuse ? ({ ok: false, outcome: { ok: false, why: "not_connected" } } as never) : ({ ok: true, value: { session: record(send.session_id, "x"), candidates: send.notes.map(() => []) } } as never);
    }),
    state: vi.fn(async () => ({
      sessions: {
        state: "read",
        sessions: [record("01HOSTEDAAAAAAAA", "Fix the flaky store test", { hosted: {} }), record("01TERMINALBBBBBB", "Release notes script", { origin: "terminal", terminal: { listening: true } }), record("01ENDEDCCCCCCCC", "Old one", { state: "ended" })],
      },
    }) as never),
    subscribe: () => () => undefined,
  } satisfies LayerFleet;
  return { fleet, sent, rings };
}

test("the bar offers Start job in place of Send to Fleet", async () => {
  await annotating(sendable(batch()), served().fleet);
  const bar = page.getByRole("status");
  await expect.element(bar.getByRole("button", { name: "Start job" })).toBeVisible();
  expect(bar.element().textContent).not.toContain("Send");
});

test("Start job starts a Job, sends its notes to it as one turn, marks them sent and opens it", async () => {
  const { fleet, sent } = served();
  const held = sendable(batch());
  const opened = vi.fn();
  await annotating(held, fleet, opened);
  await page.getByRole("status").getByRole("button", { name: "Start job" }).click();
  await vi.waitFor(() => expect(opened).toHaveBeenCalledWith("01NEWSESSION0000"));
  expect(fleet.startSession).toHaveBeenCalledTimes(1);
  expect(fleet.sendSessionNotes).toHaveBeenCalledTimes(1);
  expect(sent.map((one) => [one.session_id, one.said])).toEqual([
    ["01NEWSESSION0000", "first: what the owner said"],
    ["01NEWSESSION0000", "third: what the owner said"],
  ]);
  expect(sent.every((one) => one.note_id !== undefined && one.address === window.location.href)).toBe(true);
  const saved = (await held.list()).filter((one) => one.sent !== undefined);
  expect(saved.map((one) => one.sent)).toEqual([expect.objectContaining({ sessionId: "01NEWSESSION0000" }), expect.objectContaining({ sessionId: "01NEWSESSION0000" })]);
});

test("the dropdown lists every live Session by title, and a pick sends to it", async () => {
  const { fleet, sent } = served();
  const held = sendable(batch());
  const opened = vi.fn();
  await annotating(held, fleet, opened);
  await page.getByRole("button", { name: "Send to a Job" }).click();
  const menu = page.getByRole("menu", { name: "Send to a Job" });
  await expect.element(menu.getByRole("menuitem", { name: "Fix the flaky store test" })).toBeVisible();
  await expect.element(menu.getByRole("menuitem", { name: "Release notes script" })).toBeVisible();
  await expect.element(menu.getByRole("menuitem", { name: "Old one" })).not.toBeInTheDocument();
  await menu.getByRole("menuitem", { name: "Release notes script" }).click();
  await vi.waitFor(() => expect(opened).toHaveBeenCalledWith("01TERMINALBBBBBB"));
  expect(fleet.startSession).not.toHaveBeenCalled();
  expect(sent[0]!.session_id).toBe("01TERMINALBBBBBB");
  const saved = (await held.list()).filter((one) => one.sent !== undefined);
  expect(saved[0]!.sent).toEqual(expect.objectContaining({ sessionId: "01TERMINALBBBBBB", title: "Release notes script" }));
});

test("the dropdown leaves out a terminal Session that is not listening, as the Jobs list does", async () => {
  const { fleet } = served();
  const quiet = record("01QUIETDDDDDDDDD", "Task: explain ONE changed file", { origin: "terminal", terminal: { listening: false } });
  const before = fleet.state;
  fleet.state = vi.fn(async () => {
    const state = (await before()) as unknown as { sessions: { sessions: unknown[] } };
    return { sessions: { ...state.sessions, sessions: [...state.sessions.sessions, quiet] } } as never;
  });
  await annotating(sendable(batch()), fleet);
  await page.getByRole("button", { name: "Send to a Job" }).click();
  const menu = page.getByRole("menu", { name: "Send to a Job" });
  await expect.element(menu.getByRole("menuitem", { name: "Fix the flaky store test" })).toBeVisible();
  await expect.element(menu.getByRole("menuitem", { name: "Task: explain ONE changed file" })).not.toBeInTheDocument();
});

test("a Session Fleet refuses leaves the notes unsent and tells why in a toast, not on the bar", async () => {
  const { fleet } = served(true);
  const held = sendable(batch());
  const opened = vi.fn();
  const told = vi.fn();
  await annotating(held, fleet, opened, told);
  await page.getByRole("status").getByRole("button", { name: "Start job" }).click();
  await vi.waitFor(() => expect(told).toHaveBeenCalledWith('Notes not sent to "first: what the owner said": Fleet is not connected. Nothing was sent.'));
  expect(page.getByRole("status").element().textContent).not.toContain("Not sent");
  expect(opened).not.toHaveBeenCalled();
  expect((await held.list()).some((one) => one.sent !== undefined)).toBe(false);
});

test("a refusal with no copy of its own still tells what failed", async () => {
  const { fleet } = served();
  fleet.startSession.mockResolvedValueOnce({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.x", message: "" } } } as never);
  const held = sendable(batch());
  const told = vi.fn();
  await annotating(held, fleet, undefined, told);
  await page.getByRole("status").getByRole("button", { name: "Start job" }).click();
  await vi.waitFor(() => expect(told).toHaveBeenCalledWith("Job not started: Fleet refused it"));
  expect((await held.list()).some((one) => one.sent !== undefined)).toBe(false);
});

test("a note sent to a Session says which, on its card", async () => {
  const sent = note("sent", "open", "5", 15);
  await annotating(sendable([{ ...sent, sent: { sessionId: "01HOSTEDAAAAAAAA", title: "Fix the flaky store test", at: sent.createdAt } }]), served().fleet);
  await page.getByRole("button", { name: /sent as Fix the flaky store test/ }).click();
  await expect.element(page.getByRole("dialog", { name: "Note" })).toHaveTextContent("Sent to Job Fix the flaky store test");
});

test("a saved note stays up in its card, with the acts its panel row has", async () => {
  await annotating(sendable([]), served().fleet);
  (document.querySelector(noted("fresh", 15)) as HTMLElement).click();
  const draft = page.getByRole("dialog", { name: "New note" });
  await draft.getByRole("textbox", { name: "Note" }).fill("The row should say which step it is on");
  await draft.getByRole("button", { name: "Save note" }).click();

  const card = page.getByRole("dialog", { name: "Note", exact: true });
  await expect.element(card).toHaveTextContent("The row should say which step it is on");
  for (const name of ["Start job", "Send to a Job", "Mark done", "Delete"]) {
    await expect.element(card.getByRole("button", { name, exact: true })).toBeVisible();
  }
});

test("a card's Start job sends its note alone and opens the Session", async () => {
  const { fleet, sent } = served();
  const held = sendable(batch());
  const opened = vi.fn();
  await annotating(held, fleet, opened);
  await page.getByRole("button", { name: /^Note 2, open/ }).click();
  await page.getByRole("dialog", { name: "Note" }).getByRole("button", { name: "Start job" }).click();

  await vi.waitFor(() => expect(opened).toHaveBeenCalledWith("01NEWSESSION0000"));
  expect(sent.map((one) => one.said)).toEqual(["third: what the owner said"]);
});

// The all-notes panel, opened from the bar's count. What it does, held as the owner approved it.

/** A note whose element is not on this screen: its selector finds nothing. */
function away(name: string, at: string, change: Partial<Annotation> = {}): Annotation {
  const one = note(name, "open", at, 60);
  document.querySelector(one.selector)?.remove();
  return { ...one, selector: `[data-noted="${name}-is-elsewhere"]`, ...change };
}

const panel = () => page.getByRole("complementary", { name: "All notes" });
const countOf = () => page.getByRole("status").getByRole("button", { name: /open, \d+ done/ });
const rowOf = (text: string) => panel().getByRole("listitem").filter({ hasText: text });
const headings = () => panel().getByRole("heading").elements().map((one) => one.textContent);
const screened = (one: Annotation, screen: string | null): Annotation => ({ ...one, screen });

test("the status text opens the panel and closes it again", async () => {
  await annotating(sinkOf(batch()));
  expect(panel().elements()).toHaveLength(0);
  const count = countOf();
  await expect.element(count).toHaveAttribute("aria-expanded", "false");

  await count.click();
  await expect.element(panel()).toBeVisible();
  await expect.element(count).toHaveAttribute("aria-expanded", "true");

  await count.click();
  expect(panel().elements()).toHaveLength(0);
  await expect.element(count).toHaveAttribute("aria-expanded", "false");
});

test("the panel groups notes by screen, in name order, with No screen last", async () => {
  await annotating(
    sinkOf([
      screened(note("first", "open", "5", 15), null),
      screened(note("second", "open", "6", 30), "Overview"),
      screened(note("third", "open", "7", 45), "Job Board"),
      screened(note("fourth", "open", "8", 60), "Overview"),
    ]),
  );
  await countOf().click();
  await expect.element(panel()).toBeVisible();
  expect(headings()).toEqual(["Job Board", "Overview", "No screen"]);

  expect(panel().getByRole("region").elements()).toHaveLength(3);
  const overview = panel().getByRole("region", { name: "Overview" });
  await expect.element(overview).toHaveTextContent("second: what the owner said");
  await expect.element(overview).toHaveTextContent("fourth: what the owner said");
  await expect.element(overview).not.toHaveTextContent("third: what the owner said");
  await expect.element(panel().getByRole("region", { name: "Job Board" })).toHaveTextContent("third: what the owner said");
  await expect.element(panel().getByRole("region", { name: "No screen" })).toHaveTextContent("first: what the owner said");
});

test("a done note is listed only once the done notes are shown, and then after the open ones", async () => {
  await annotating(sinkOf(batch()));
  await countOf().click();
  await expect.element(rowOf("first:")).toBeVisible();
  expect(rowOf("second:").elements()).toHaveLength(0);

  await page.getByRole("status").getByRole("button", { name: "Show done notes" }).click();
  await expect.element(rowOf("second:")).toBeVisible();
  expect(panel().getByRole("listitem").elements().map((one) => one.getAttribute("data-status"))).toEqual(["open", "open", "done"]);

  await page.getByRole("status").getByRole("button", { name: "Hide done notes" }).click();
  expect(rowOf("second:").elements()).toHaveLength(0);
});

test("a screen holding only done notes has no heading until they are shown", async () => {
  await annotating(sinkOf([screened(note("first", "open", "5", 15), "Overview"), screened(note("second", "done", "6", 30), "Job Board")]));
  await countOf().click();
  await expect.element(panel()).toBeVisible();
  expect(headings()).toEqual(["Overview"]);
  await page.getByRole("status").getByRole("button", { name: "Show done notes" }).click();
  await expect.element(panel().getByRole("heading", { name: "Job Board" })).toBeVisible();
});

test("a row's Start job sends that note alone to a new Session, and marks only it sent", async () => {
  const { fleet, sent } = served();
  const held = sendable(batch());
  const opened = vi.fn();
  await annotating(held, fleet, opened);
  await countOf().click();
  await rowOf("third:").getByRole("button", { name: "Start job" }).click();

  await vi.waitFor(() => expect(opened).toHaveBeenCalledWith("01NEWSESSION0000"));
  expect(fleet.startSession).toHaveBeenCalledTimes(1);
  expect(sent.map((one) => one.said)).toEqual(["third: what the owner said"]);
  expect((await held.list()).filter((one) => one.sent !== undefined).map((one) => one.text)).toEqual(["third: what the owner said"]);
});

test("a sent note and a done note offer no send, and an open unsent one does", async () => {
  const unsent = note("first", "open", "5", 15);
  const forwarded = note("second", "open", "6", 30);
  const finished = note("third", "done", "7", 45);
  await annotating(
    sendable([unsent, { ...forwarded, sent: { sessionId: "01S", title: "Release notes", at: forwarded.createdAt } }, finished]),
    served().fleet,
  );
  await countOf().click();
  await page.getByRole("status").getByRole("button", { name: "Show done notes" }).click();
  await expect.element(rowOf("third:")).toBeVisible();

  await expect.element(rowOf("first:").getByRole("button", { name: "Start job" })).toBeVisible();
  expect(rowOf("second:").getByRole("button", { name: "Start job" }).elements()).toHaveLength(0);
  expect(rowOf("third:").getByRole("button", { name: "Start job" }).elements()).toHaveLength(0);
  // What stays on every row is what a person does to a note, sent or not.
  await expect.element(rowOf("second:").getByRole("button", { name: "Mark done" })).toBeVisible();
  await expect.element(rowOf("third:").getByRole("button", { name: "Reopen" })).toBeVisible();
});

test("a row opens its card when the element is on screen, and does nothing when it is not", async () => {
  await annotating(sinkOf([note("here", "open", "5", 15), away("gone", "6")]));
  await countOf().click();
  await expect.element(rowOf("gone:")).toBeVisible();
  const there = rowOf("gone:").element() as HTMLElement;
  expect(there.hasAttribute("data-here")).toBe(false);
  expect(rowOf("here:").element().hasAttribute("data-here")).toBe(true);

  // A DOM click: the row is aria-disabled, which the pointer-level click waits on forever.
  (there.querySelector(".armada-annotate__row-main") as HTMLElement).click();
  await new Promise((settled) => requestAnimationFrame(() => requestAnimationFrame(settled)));
  expect(page.getByRole("dialog", { name: "Note" }).elements()).toHaveLength(0);

  await rowOf("here:").getByRole("button", { name: /here: what the owner said/ }).click();
  await expect.element(page.getByRole("dialog", { name: "Note" })).toHaveTextContent("here: what the owner said");
});

test("a note off this screen sent to a Session goes without a screenshot", async () => {
  const { fleet, sent } = served();
  const capture = vi.fn(async () => new Uint8Array([137, 80, 78, 71]).buffer);
  const held = { ...sendable([away("gone", "6")]), capture };
  await annotating(held, fleet);
  await countOf().click();

  await rowOf("gone:").getByRole("button", { name: "Start job" }).click();
  await vi.waitFor(() => expect(sent).toHaveLength(1));
  expect(capture).not.toHaveBeenCalled();
  expect(sent[0]).not.toHaveProperty("crop");
});

test("a row off this screen says on Start job that it goes without a screenshot", async () => {
  await annotating(sendable([note("here", "open", "5", 15), away("gone", "6")]), served().fleet);
  await countOf().click();
  const fact = "No screenshot: on another screen";

  await expect.element(rowOf("gone:").getByRole("button", { name: "Start job" })).toHaveAccessibleDescription(fact);
  await expect.element(rowOf("here:").getByRole("button", { name: "Start job" })).not.toHaveAccessibleDescription(expect.stringContaining("screenshot"));
});

/** A note off this screen that kept its picture as it was saved, and the store holding it. */
function keptAway(name: string, at: string): { note: Annotation; shots: Map<string, ArrayBuffer> } {
  const one = away(name, at);
  return { note: { ...one, screenshot: `${one.id}.png` }, shots: new Map([[one.id, PNG]]) };
}

test("a note that kept its picture is sent with it from another screen, and its row says nothing is missing", async () => {
  const { fleet, sent } = served();
  const capture = vi.fn(async () => new Uint8Array([137, 80, 78, 71]).buffer);
  const kept = keptAway("kept", "6");
  const held = { ...sendable([kept.note], kept.shots), capture };
  await annotating(held, fleet);
  await countOf().click();

  await expect.element(rowOf("kept:").getByRole("button", { name: "Start job" })).not.toHaveAccessibleDescription(expect.stringContaining("screenshot"));
  await rowOf("kept:").getByRole("button", { name: "Start job" }).click();
  await vi.waitFor(() => expect(sent).toHaveLength(1));
  expect(sent[0]!.note_id).toBe(kept.note.id);
  expect(sent[0]!.crop?.media_type).toBe("image/png");
  expect(capture).not.toHaveBeenCalled();
});

test("a screenshot taken at send, of an older note on this screen, is taken with the layer out of it", async () => {
  const { fleet, sent } = served();
  const seen: boolean[] = [];
  const capture = vi.fn(async () => {
    seen.push(document.querySelector(".armada-annotate")!.hasAttribute("data-shooting"));
    return PNG;
  });
  await annotating({ ...sendable([note("here", "open", "5", 15)]), capture }, fleet);
  await countOf().click();
  await rowOf("here:").getByRole("button", { name: "Start job" }).click();
  await vi.waitFor(() => expect(sent).toHaveLength(1));
  expect(seen).toEqual([true]);
  expect(document.querySelector(".armada-annotate")!.hasAttribute("data-shooting")).toBe(false);
});

// Several notes ticked in the panel and acted on at once. Every act here takes
// the ticked notes and no others, which the bar's own sends cannot: they take
// every open note on this screen.

/** Four open notes, each on a live element, in the order they were written. */
const four = (): Annotation[] => [
  note("first", "open", "5", 15),
  note("second", "open", "6", 30),
  note("third", "open", "7", 45),
  note("fourth", "open", "8", 60),
];

const selection = () => panel().getByRole("group", { name: "Selected notes" });
const tickOf = (text: string) => panel().getByRole("checkbox", { name: `Select ${text}` });
const ticked = () =>
  panel()
    .getByRole("checkbox", { name: /^Select (?!all)/ })
    .elements()
    .filter((one) => (one as HTMLInputElement).checked)
    .map((one) => (one.getAttribute("aria-label") ?? "").replace(/^Select /, "").replace(/: what the owner said$/, ""));
const textsOf = async (sink: Sink, which: (one: Annotation) => boolean) => (await sink.list()).filter(which).map((one) => one.text.replace(/: what the owner said$/, ""));

async function panelOn(sink: Sink, fleet?: LayerFleet, openSession?: (id: string) => void, tell?: (sentence: string) => void): Promise<void> {
  await annotating(sink, fleet, openSession, tell);
  await countOf().click();
  await expect.element(panel()).toBeVisible();
}

test("ticking rows selects them, and the head says how many with the acts on them", async () => {
  await panelOn(sendable(four()), served().fleet);
  // Nothing is ticked, so the head offers Select all and no act.
  await expect.element(selection().getByRole("checkbox", { name: "Select all" })).not.toBeChecked();
  expect(selection().getByRole("button").elements()).toHaveLength(0);

  await tickOf("first:").click();
  await tickOf("third:").click();
  expect(ticked()).toEqual(["first", "third"]);
  await expect.element(selection()).toHaveTextContent("2 selected");
  for (const name of ["Start job", "Mark done", "Delete"]) {
    await expect.element(selection().getByRole("button", { name, exact: true })).toBeVisible();
  }
  expect(rowOf("first:").element().hasAttribute("data-selected")).toBe(true);
  expect(rowOf("second:").element().hasAttribute("data-selected")).toBe(false);

  await tickOf("first:").click();
  expect(ticked()).toEqual(["third"]);
  await expect.element(selection()).toHaveTextContent("1 selected");
});

test("Select all ticks every listed note, and pressed again clears them", async () => {
  await panelOn(sendable(four()), served().fleet);
  const all = selection().getByRole("checkbox", { name: "Select all" });
  await all.click();
  expect(ticked()).toEqual(["first", "second", "third", "fourth"]);
  await expect.element(all).toBeChecked();
  await all.click();
  expect(ticked()).toEqual([]);
  expect(selection().getByRole("button").elements()).toHaveLength(0);
});

test("Select all takes only what is listed: a done note stays out until the done notes are shown", async () => {
  await panelOn(sendable(batch()), served().fleet);
  await selection().getByRole("checkbox", { name: "Select all" }).click();
  expect(ticked()).toEqual(["first", "third"]);
  await expect.element(selection()).toHaveTextContent("2 selected");
});

test("a shift-press ticks every row between it and the last one ticked", async () => {
  await panelOn(sendable(four()), served().fleet);
  await tickOf("first:").click();
  await tickOf("third:").click({ modifiers: ["Shift"] });
  expect(ticked()).toEqual(["first", "second", "third"]);
});

test("Start job on the selection sends the ticked notes alone, each on its own, and clears the selection", async () => {
  const { fleet, sent } = served();
  const held = sendable(four());
  const opened = vi.fn();
  await panelOn(held, fleet, opened);
  await tickOf("first:").click();
  await tickOf("third:").click();
  await selection().getByRole("button", { name: "Start job" }).click();

  await vi.waitFor(() => expect(opened).toHaveBeenCalledWith("01NEWSESSION0000"));
  expect(sent.map((one) => one.said)).toEqual(["first: what the owner said", "third: what the owner said"]);
  expect(await textsOf(held, (one) => one.sent !== undefined)).toEqual(["first", "third"]);
  await vi.waitFor(() => expect(ticked()).toEqual([]));
});

test("the selection's caret sends the ticked notes to a live Session", async () => {
  const { fleet, sent } = served();
  const held = sendable(four());
  await panelOn(held, fleet, vi.fn());
  await tickOf("fourth:").click();
  await selection().getByRole("button", { name: "Send to a Job" }).click();
  await page.getByRole("menu", { name: "Send to a Job" }).getByRole("menuitem", { name: "Fix the flaky store test" }).click();

  await vi.waitFor(() => expect(sent).toHaveLength(1));
  expect(fleet.startSession).not.toHaveBeenCalled();
  expect(sent[0]!.session_id).toBe("01HOSTEDAAAAAAAA");
  expect(sent.map((one) => one.said)).toEqual(["fourth: what the owner said"]);
  await vi.waitFor(async () => expect(await textsOf(held, (one) => one.sent !== undefined)).toEqual(["fourth"]));
});

test("Mark done and Reopen on the selection change the ticked notes alone", async () => {
  const held = sinkOf(four());
  await panelOn(held);
  await tickOf("second:").click();
  await tickOf("third:").click();
  await selection().getByRole("button", { name: "Mark done" }).click();

  await vi.waitFor(async () => expect(await textsOf(held, (one) => one.status === "done")).toEqual(["second", "third"]));
  await expect.element(page.getByRole("status")).toHaveTextContent("2 open, 2 done");
  expect(ticked()).toEqual([]);

  await page.getByRole("status").getByRole("button", { name: "Show done notes" }).click();
  await tickOf("first:").click();
  await tickOf("third:").click();
  // One open and one done: both acts are offered, each for the ticked notes it applies to.
  await selection().getByRole("button", { name: "Reopen" }).click();
  await vi.waitFor(async () => expect(await textsOf(held, (one) => one.status === "done")).toEqual(["second"]));
  expect(ticked()).toEqual([]);
});

test("Delete on the selection asks once for the batch, then deletes the ticked notes alone", async () => {
  const held = sinkOf(four());
  await panelOn(held);
  await tickOf("first:").click();
  await tickOf("fourth:").click();
  const remove = selection().getByRole("button", { name: "Delete" });
  await remove.click();
  await expect.element(remove).toHaveTextContent("Press again to delete");
  expect(await textsOf(held, () => true)).toEqual(["first", "second", "third", "fourth"]);

  await remove.click();
  await vi.waitFor(async () => expect(await textsOf(held, () => true)).toEqual(["second", "third"]));
  expect(rowOf("first:").elements()).toHaveLength(0);
  expect(ticked()).toEqual([]);
  expect(selection().getByRole("button").elements()).toHaveLength(0);
});

test("a change to the selection takes back an armed Delete", async () => {
  await panelOn(sinkOf(four()));
  await tickOf("first:").click();
  await selection().getByRole("button", { name: "Delete" }).click();
  await expect.element(selection().getByRole("button", { name: "Press again to delete" })).toBeVisible();
  await tickOf("second:").click();
  await expect.element(selection().getByRole("button", { name: "Delete", exact: true })).toBeVisible();
});

test("a ticked note deleted from its own row leaves the selection", async () => {
  await panelOn(sinkOf(four()));
  await tickOf("first:").click();
  await tickOf("second:").click();
  const remove = rowOf("first:").getByRole("button", { name: "Delete" });
  await remove.click();
  await rowOf("first:").getByRole("button", { name: "Press again to delete" }).click();
  await expect.element(rowOf("first:")).not.toBeInTheDocument();
  expect(ticked()).toEqual(["second"]);
  await expect.element(selection()).toHaveTextContent("1 selected");
});

test("a selection holding sent and done notes sends only the open unsent ones, and says how many it leaves out", async () => {
  const { fleet, sent } = served();
  const forwarded = note("second", "open", "6", 30);
  const held = sendable([note("first", "open", "5", 15), { ...forwarded, sent: { sessionId: "01OLD", title: "Old one", at: forwarded.createdAt } }, note("third", "done", "7", 45)]);
  await panelOn(held, fleet);
  await page.getByRole("status").getByRole("button", { name: "Show done notes" }).click();
  await selection().getByRole("checkbox", { name: "Select all" }).click();
  await expect.element(selection()).toHaveTextContent("3 selected. Sends leave out 2: 1 already sent, 1 done");

  await selection().getByRole("button", { name: "Start job" }).click();
  await vi.waitFor(() => expect(sent).toHaveLength(1));
  expect(fleet.startSession).toHaveBeenCalledTimes(1);
  expect(sent.map((one) => one.said)).toEqual(["first: what the owner said"]);
  await vi.waitFor(() => expect(ticked()).toEqual([]));
});

test("a selection of only sent and done notes offers no send, and still offers the rest", async () => {
  const forwarded = note("second", "open", "6", 30);
  await panelOn(sendable([{ ...forwarded, sent: { sessionId: "01OLD", title: "Old one", at: forwarded.createdAt } }, note("third", "done", "7", 45)]), served().fleet);
  await page.getByRole("status").getByRole("button", { name: "Show done notes" }).click();
  await selection().getByRole("checkbox", { name: "Select all" }).click();
  await expect.element(selection()).toHaveTextContent("2 selected");
  expect(selection().element().textContent).not.toContain("Sends leave out");
  expect(selection().getByRole("button", { name: "Start job" }).elements()).toHaveLength(0);
  for (const name of ["Mark done", "Reopen", "Delete"]) await expect.element(selection().getByRole("button", { name, exact: true })).toBeVisible();
});

test("where sending is not possible, the selection offers no Start job", async () => {
  await panelOn(sinkOf(four()));
  await tickOf("first:").click();
  expect(selection().getByRole("button", { name: "Start job" }).elements()).toHaveLength(0);
});

test("a selection reaching off this screen is sent, and its acts say how many go without a screenshot", async () => {
  const { fleet, sent } = served();
  const capture = vi.fn(async () => new Uint8Array([137, 80, 78, 71]).buffer);
  const held = { ...sendable([note("here", "open", "5", 15), away("gone", "6")]), capture };
  await panelOn(held, fleet, vi.fn());
  await tickOf("here:").click();
  await tickOf("gone:").click();
  await expect.element(selection().getByRole("button", { name: "Start job" })).toHaveAccessibleDescription("No screenshot for 1 of 2: on another screen");

  await selection().getByRole("button", { name: "Start job" }).click();
  await vi.waitFor(() => expect(sent).toHaveLength(2));
  // One screenshot, of the note on this screen.
  expect(capture).toHaveBeenCalledTimes(1);
  expect(sent.map((one) => [one.said, one.crop !== undefined])).toEqual([["here: what the owner said", true], ["gone: what the owner said", false]]);
  await vi.waitFor(async () => expect(await textsOf(held, (one) => one.sent !== undefined)).toEqual(["here", "gone"]));
});

test("a selection counts as going without a screenshot only the notes off this screen that kept none", async () => {
  const kept = keptAway("kept", "7");
  await panelOn(sendable([note("here", "open", "5", 15), away("gone", "6"), kept.note], kept.shots), served().fleet);
  await selection().getByRole("checkbox", { name: "Select all" }).click();
  await expect.element(selection().getByRole("button", { name: "Start job" })).toHaveAccessibleDescription("No screenshot for 1 of 3: on another screen");
});

test("turning the layer off and on again comes back with nothing selected", async () => {
  await panelOn(sinkOf(four()));
  await tickOf("first:").click();
  await pressToggle();
  await pressToggle();
  await countOf().click();
  await expect.element(panel()).toBeVisible();
  expect(ticked()).toEqual([]);
});
