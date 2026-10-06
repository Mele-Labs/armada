// ⌘V on an open Studio, through `App` — the owner's note of 1 Oct 2026. Each paste is one shape a
// real ⌘V handed Chromium on macOS, as the decision `2026-10-01-a-paste-lands-at-once.md` lists
// them, sent to whatever a real press on the board left focused — so focus is the browser's.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp, type Mounted, studying } from "@armada/desktop/mock";

const SLICES = { slices: ["core", "studios"] } as const;
import { onDisk } from "./fake";

let mounted: { app: Mounted; host: HTMLElement } | null = null;

afterEach(() => {
  mounted?.app.unmount();
  mounted?.host.remove();
  mounted = null;
});

const ADDRESS = "https://example.invalid/armada/pull/1721";
const BRIEFING = "/Users/user/Development/armada/crates/fleet/src/briefing.rs";
const COVER = "/Users/user/Development/armada/packages/brand/covers/armada-cover-hero.png";

/** One shape, as the paste event carries it. */
type Shape = { strings?: Record<string, string>; files?: File[] };

const fleet = studying();
const kept = () => fleet.studios().find((one) => one.name === "The Board's legend")!.nodes;
const node = (name: RegExp) => page.getByRole("group", { name });
const board = () => document.querySelector<HTMLElement>(".armada-studio-whiteboard")!;

/** The Studio open and Continued, so it is the person's to add to. */
async function openEditable(): Promise<void> {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  mounted = { app: mountApp(fleet.scenario, host, undefined, SLICES), host };
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("cell", { name: "The Board's legend", exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await expect.element(node(/^Note: The legend under the step bar/)).toBeVisible();
}

/** A real press on empty board, the `nth` free point, and where it was in the window. */
async function pressBoard(nth: number): Promise<{ x: number; y: number }> {
  const pane = document.querySelector<HTMLElement>(".armada-studio-whiteboard .react-flow__pane")!;
  const box = pane.getBoundingClientRect();
  const free: { x: number; y: number }[] = [];
  for (let y = box.top + 40; y < box.bottom - 240; y += 60) {
    for (let x = box.left + 40; x < box.right - 280; x += 60) {
      if (document.elementFromPoint(x, y) === pane) free.push({ x, y });
    }
  }
  const at = free[nth * 7]!;
  await userEvent.click(page.elementLocator(pane), { position: { x: at.x - box.left, y: at.y - box.top } });
  return at;
}

/** ⌘V's event, at what has focus. Answers whether Bridge took it from the browser. */
function paste({ strings = {}, files = [] }: Shape): boolean {
  const clipboard = new DataTransfer();
  for (const [type, value] of Object.entries(strings)) clipboard.setData(type, value);
  for (const file of files) clipboard.items.add(file);
  const event = new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true });
  (document.activeElement ?? document.body).dispatchEvent(event);
  return event.defaultPrevented;
}

/** What the last node added holds, without what Fleet stamps on it. */
async function landed(before: number): Promise<Record<string, unknown>> {
  await expect.poll(() => kept().length).toBe(before + 1);
  const { id: _id, position: _at, created_at: _when, added_by: _by, ...content } = kept().at(-1)!;
  return content;
}

/** The node just landed sits with its corner where the board was pressed. */
function atPress(name: RegExp, at: { x: number; y: number }): void {
  const box = node(name).element().getBoundingClientRect();
  expect(Math.abs(box.left - at.x)).toBeLessThan(3);
  expect(Math.abs(box.top - at.y)).toBeLessThan(3);
}

test("a press on empty board gives it focus, so ⌘V reaches it", async () => {
  await openEditable();
  await pressBoard(0);
  expect(board().contains(document.activeElement)).toBe(true);
});

test("an address from the address bar, or Copy Link, lands at once as a Link under the pointer", async () => {
  await openEditable();
  const before = kept().length;
  const at = await pressBoard(0);
  expect(paste({ strings: { "text/plain": ADDRESS } })).toBe(true);
  expect(await landed(before)).toEqual({ kind: "link", address: ADDRESS });
  await expect.element(node(/^Link: https:\/\/example\.invalid\/armada\/pull\/1721/)).toBeVisible();
  atPress(/^Link: https:\/\/example/, at);
  // At once: no draft, and no field opened for a line.
  expect(board().querySelector("textarea, input")).toBeNull();
});

test("text copied off a page lands as a Note, and its HTML is not what it says", async () => {
  await openEditable();
  const before = kept().length;
  await pressBoard(1);
  const said = "Plain sentence to copy from the page.";
  expect(paste({ strings: { "text/plain": said, "text/html": `<meta charset='utf-8'><span>${said}</span>` } })).toBe(true);
  expect(await landed(before)).toEqual({ kind: "note", said });
  await expect.element(node(/^Note: Plain sentence to copy from the page\./)).toBeVisible();
});

test("a path copied in a terminal lands as a File, trimmed, beside its HTML and RTF", async () => {
  await openEditable();
  const before = kept().length;
  await pressBoard(2);
  const strings = {
    "text/plain": "crates/fleet/src/briefing.rs\n  ",
    "text/html": "<html><body><pre>crates/fleet/src/briefing.rs</pre></body></html>",
    "text/rtf": "{\\rtf1\\ansi crates/fleet/src/briefing.rs}",
  };
  expect(paste({ strings })).toBe(true);
  expect(await landed(before)).toEqual({ kind: "file", path: "crates/fleet/src/briefing.rs" });
  await expect.element(node(/^File: crates\/fleet\/src\/briefing\.rs/)).toBeVisible();
});

test("a file copied in Finder lands as a File at its path on disk, though only its name reaches the page", async () => {
  await openEditable();
  const before = kept().length;
  await pressBoard(3);
  expect(paste({ files: [onDisk(new File(["fn main() {}"], "briefing.rs"), BRIEFING)] })).toBe(true);
  expect(await landed(before)).toEqual({ kind: "file", path: BRIEFING });
});

test("an image file copied in Finder lands as a File, since the picture beside it is its icon", async () => {
  await openEditable();
  const before = kept().length;
  await pressBoard(4);
  const png = new File([new Uint8Array([137, 80, 78, 71])], "armada-cover-hero.png", { type: "image/png" });
  expect(paste({ files: [onDisk(png, COVER)] })).toBe(true);
  expect(await landed(before)).toEqual({ kind: "file", path: COVER });
});

/** A screenshot as the clipboard hands it over: a PNG called `image.png`, on no disk. */
async function screenshot(): Promise<File> {
  const canvas = new OffscreenCanvas(320, 200);
  const ink = canvas.getContext("2d")!;
  ink.fillStyle = "darkslategray";
  ink.fillRect(0, 0, 320, 200);
  return new File([await canvas.convertToBlob({ type: "image/png" })], "image.png", { type: "image/png" });
}

test("a screenshot lands at once as a Picture under the pointer, drawn and opened full size", async () => {
  await openEditable();
  const before = kept().length;
  const at = await pressBoard(5);
  expect(paste({ files: [await screenshot()] })).toBe(true);
  await expect.poll(() => kept().length).toBe(before + 1);
  expect(kept().at(-1)).toMatchObject({ kind: "picture", frame: { width: 320, height: 200 } });
  await expect.element(node(/^Picture$/)).toBeVisible();
  atPress(/^Picture$/, at);
  await expect.element(node(/^Picture$/).getByRole("img", { name: "The picture pasted onto this Studio" })).toBeVisible();

  // Opened full size, as a Note's frame is.
  node(/^Picture$/).element().focus();
  await userEvent.keyboard("{Enter}");
  await page.getByRole("group", { name: "What is picked" }).getByRole("button", { name: "Open frame", exact: true }).click();
  await expect.element(page.getByRole("dialog").getByRole("img", { name: "The picture pasted onto this Studio" })).toBeVisible();
});

test("a picture over Fleet's 4 MiB is refused, and the board says why", async () => {
  await openEditable();
  const before = kept().length;
  await pressBoard(6);
  const heavy = new File([new Uint8Array(4 * 1024 * 1024 + 1)], "image.png", { type: "image/png" });
  expect(paste({ files: [heavy] })).toBe(true);
  await expect.element(page.getByText("Fleet did not take that", { exact: true })).toBeVisible();
  await expect.element(page.getByText(/^a frame weighs at most 4194304 bytes and this one weighs 4194305$/)).toBeVisible();
  expect(kept()).toHaveLength(before);
});

test("with the pointer off the board, a paste lands in the middle of the view", async () => {
  await openEditable();
  const before = kept().length;
  const at = await pressBoard(0);
  await userEvent.hover(page.getByRole("button", { name: "Studios", exact: true }).first());
  expect(paste({ strings: { "text/plain": "Off the board" } })).toBe(true);
  await landed(before);
  await expect.element(node(/^Note: Off the board/)).toBeVisible();
  // `useStudioPlacement`'s middle, stepped clear of whatever is already there — not the last press.
  const box = node(/^Note: Off the board/).element().getBoundingClientRect();
  const view = board().getBoundingClientRect();
  expect(Math.abs(box.left - at.x) + Math.abs(box.top - at.y)).toBeGreaterThan(100);
  expect(box.left).toBeGreaterThan(view.left);
  expect(box.right).toBeLessThan(view.right);
  expect(box.top).toBeGreaterThan(view.top);
  expect(box.bottom).toBeLessThan(view.bottom);
});

test("a paste into a draft's field or the Studio's name is that field's, and lands no node", async () => {
  await openEditable();
  const before = kept().length;
  await page.getByRole("button", { name: "Add a Note", exact: true }).click();
  await pressBoard(1);
  const field = page.getByRole("group", { name: "New Note", exact: true }).getByLabelText("Note", { exact: true });
  await expect.element(field).toHaveFocus();
  expect(paste({ strings: { "text/plain": ADDRESS } })).toBe(false);
  await userEvent.keyboard("{Escape}");

  await page.getByRole("button", { name: "Rename The Board's legend" }).click();
  expect(document.activeElement?.tagName).toBe("INPUT");
  expect(paste({ strings: { "text/plain": ADDRESS } })).toBe(false);
  await new Promise((resolve) => setTimeout(resolve, 200));
  expect(kept()).toHaveLength(before);
});

test("a Studio reopened read-only takes no paste", async () => {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  mounted = { app: mountApp(fleet.scenario, host, undefined, SLICES), host };
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("cell", { name: "The Board's legend", exact: true }).click();
  await expect.element(page.getByText("Read-only", { exact: true })).toBeVisible();
  const before = kept().length;
  await pressBoard(0);
  expect(paste({ strings: { "text/plain": ADDRESS } })).toBe(false);
  await new Promise((resolve) => setTimeout(resolve, 200));
  expect(kept()).toHaveLength(before);
});
