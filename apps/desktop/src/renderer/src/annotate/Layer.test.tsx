// The layer in a real window, on the case that broke it: an element as tall as
// the window. The card is `position: fixed`, so the window is the frame, and
// what this asserts is that every edge of the card is inside it — measured,
// because the defect was purely geometric and nothing about the markup showed it.

import { StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { Annotation, AnnotationStatus } from "../../../shared/annotations";
import "../styles/index.css";
import { Layer, type LayerFleet } from "./Layer";
import type { Sink } from "./sink";

/** The owner's window in the screenshot, and a short one. */
const OWNER = { width: 1324, height: 922 };
const SHORT = { width: 1324, height: 700 };

/** Notes the layer reads, saves back to and deletes from, held in memory. */
function sinkOf(notes: Annotation[] = []): Sink {
  const held = new Map(notes.map((note) => [note.id, note]));
  return {
    via: "dev server",
    list: async () => [...held.values()],
    save: async (note) => {
      held.set(note.id, note);
    },
    remove: async (id) => {
      held.delete(id);
    },
    root: async () => null,
    capture: async () => null,
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
test("a note saved with a Job's detail open carries that Job's id for its retro, and one saved without names none", async () => {
  const saved: Annotation[] = [];
  const keeping: Sink = { ...sinkOf(), save: async (one) => void saved.push(one) };
  await annotating(keeping);
  const detail = document.createElement("div");
  detail.setAttribute("data-noted", "");
  detail.setAttribute("data-armada-open-job", "01M22TYSAE0023MADDP5ZQEYGW");
  document.body.append(detail);

  async function saveOn(text: string, top: number): Promise<void> {
    document.querySelector<HTMLElement>(noted(`target-${top}`, top))?.click();
    const card = page.getByRole("dialog", { name: "New note" });
    await card.getByRole("textbox", { name: "Note" }).fill(text);
    await card.getByRole("button", { name: "Save note" }).click();
    await expect.element(card).not.toBeInTheDocument();
  }

  await saveOn("With the Job open", 20);
  detail.remove();
  await saveOn("With nothing open", 50);
  expect(saved.map((one) => [one.text, one.openJobId])).toEqual([
    ["With the Job open", "01M22TYSAE0023MADDP5ZQEYGW"],
    ["With nothing open", undefined],
  ]);
  expect("openJobId" in (saved[1] ?? {})).toBe(false);
});

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

// The bar's two sends: a Job at the gate, and the notes as a message to a Session.

const sendable = (notes: Annotation[]): Sink => ({ ...sinkOf(notes), via: "main", root: async () => "/Users/user/armada" });

const record = (id: string, title: string | undefined, change: object = {}) =>
  ({ id, title, state: "live", origin: "bridge", ...change }) as never;

/** A Fleet serving three Sessions, one from a terminal and one ended, and answering every send. */
function served(refuse = false) {
  const sent: { session_id: string; text: string }[] = [];
  const fleet = {
    stageAttachment: vi.fn(async () => ({ path: "/tmp/x.png" })),
    proposeFromRequest: vi.fn(async () => ({ ok: true, jobs: [{ id: "01JOB", handle: "9-the-note" }] }) as never),
    startSession: vi.fn(async (title?: string) => ({ ok: true, value: record("01NEWSESSION0000", title) }) as never),
    sendSessionMessage: vi.fn(async (send: { session_id: string; text: string }) => {
      sent.push(send);
      return refuse ? ({ ok: false, outcome: { ok: false, why: "not_connected" } } as never) : ({ ok: true, value: record(send.session_id, "x") } as never);
    }),
    state: vi.fn(async () => ({
      sessions: {
        state: "read",
        sessions: [record("01HOSTEDAAAAAAAA", "Fix the flaky store test"), record("01TERMINALBBBBBB", "Release notes script", { origin: "terminal" }), record("01ENDEDCCCCCCCC", "Old one", { state: "ended" })],
      },
    }) as never),
    subscribe: () => () => undefined,
  } satisfies LayerFleet;
  return { fleet, sent };
}

test("the bar offers Dispatch job and Start session in place of Send to Fleet", async () => {
  await annotating(sendable(batch()), served().fleet);
  const bar = page.getByRole("status");
  await expect.element(bar.getByRole("button", { name: "Dispatch job" })).toBeVisible();
  await expect.element(bar.getByRole("button", { name: "Start session" })).toBeVisible();
  expect(bar.element().textContent).not.toContain("Send");
});

test("Dispatch job proposes the notes as a Job, as Send to Fleet did", async () => {
  const { fleet } = served();
  await annotating(sendable(batch()), fleet);
  await page.getByRole("status").getByRole("button", { name: "Dispatch job" }).click();
  await vi.waitFor(() => expect(fleet.proposeFromRequest).toHaveBeenCalledTimes(2));
  expect(fleet.startSession).not.toHaveBeenCalled();
});

test("Start session starts a Session with the notes as its first message, marks them sent and opens it", async () => {
  const { fleet, sent } = served();
  const held = sendable(batch());
  const opened = vi.fn();
  await annotating(held, fleet, opened);
  await page.getByRole("status").getByRole("button", { name: "Start session" }).click();
  await vi.waitFor(() => expect(opened).toHaveBeenCalledWith("01NEWSESSION0000"));
  expect(fleet.startSession).toHaveBeenCalledTimes(1);
  expect(sent).toHaveLength(1);
  expect(sent[0]!.session_id).toBe("01NEWSESSION0000");
  expect(sent[0]!.text).toContain("first: what the owner said");
  expect(sent[0]!.text).toContain("third: what the owner said");
  const saved = (await held.list()).filter((one) => one.sent !== undefined);
  expect(saved.map((one) => one.sent)).toEqual([expect.objectContaining({ sessionId: "01NEWSESSION0000" }), expect.objectContaining({ sessionId: "01NEWSESSION0000" })]);
});

test("the dropdown lists every live Session by title, and a pick sends to it", async () => {
  const { fleet, sent } = served();
  const held = sendable(batch());
  const opened = vi.fn();
  await annotating(held, fleet, opened);
  await page.getByRole("button", { name: "Send to a Session" }).click();
  const menu = page.getByRole("menu", { name: "Send to a Session" });
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

test("a Session Fleet refuses leaves the notes unsent and tells why in a toast, not on the bar", async () => {
  const { fleet } = served(true);
  const held = sendable(batch());
  const opened = vi.fn();
  const told = vi.fn();
  await annotating(held, fleet, opened, told);
  await page.getByRole("status").getByRole("button", { name: "Start session" }).click();
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
  await page.getByRole("status").getByRole("button", { name: "Start session" }).click();
  await vi.waitFor(() => expect(told).toHaveBeenCalledWith("Session not started: Fleet refused it"));
  expect((await held.list()).some((one) => one.sent !== undefined)).toBe(false);
});

test("a note sent to a Session says which, on its card", async () => {
  const sent = note("sent", "open", "5", 15);
  await annotating(sendable([{ ...sent, sent: { sessionId: "01HOSTEDAAAAAAAA", title: "Fix the flaky store test", at: sent.createdAt } }]), served().fleet);
  await page.getByRole("button", { name: /sent as Fix the flaky store test/ }).click();
  await expect.element(page.getByRole("dialog", { name: "Note" })).toHaveTextContent("Sent to Session Fix the flaky store test");
});
