// The Builds panel of the Usage page: what a row says of its build, where a change shows and where its slot is kept empty,
// and the keys that move through the list. The rows are mounted over the panel's own props; `c` belongs to the page, so those
// run on the page. Keys go through the browser, since the list takes them while it holds focus.

import { UsagePage } from "@armada/components";
import { BuildsPanel } from "@armada/components/src/compositions/UsagePage/UsageBuilds";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { userEvent } from "vitest/browser";

import "@armada/components/src/look";
import "./styles/index.css";
import { BUILDS, TIME } from "./usage-time-fixtures";

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

function render(element: React.ReactElement): HTMLElement {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  flushSync(() => root.render(element));
  return host;
}

const panel = (onPick: (id: string) => void = () => {}, view: "time" | "tokens" = "time") =>
  render(<BuildsPanel builds={BUILDS} view={view} pick={undefined} onPick={onPick} />);

const keys = (...pressed: string[]) => userEvent.keyboard(pressed.join(""));
const row = (host: HTMLElement, id: string) => host.querySelector<HTMLElement>(`#build-${id}`)!;
const list = (host: HTMLElement) =>
  host.querySelector<HTMLElement>('[role="listbox"][aria-label="Builds"]')!;
const active = (host: HTMLElement) => list(host).getAttribute("aria-activedescendant");

/** What every tooltip inside the element says. */
const tips = (el: Element): string[] =>
  [
    ...el.querySelectorAll("[aria-describedby]"),
  ].map((one) =>
    (one.getAttribute("aria-describedby") ?? "")
      .split(" ")
      .map((id) => document.getElementById(id)?.textContent ?? "")
      .join(" ")
      .trim(),
  );

test("a preview build carries a glyph that says what it holds, and a main build carries none", () => {
  const host = panel();
  const preview = row(host, "7ac3d52");
  const main = row(host, "e5604ab");
  const held = "Preview of main e5604ab91c with armada/check-queue, armada/slot-pool";
  expect(tips(preview)).toContain(held);
  expect(preview.getAttribute("aria-label")).toContain(", preview,");
  // The glyph is the tooltip's anchor and draws an icon; a main row draws none.
  const glyph = preview.querySelector(`[aria-describedby]:has(svg)`);
  expect(glyph).not.toBeNull();
  expect(main.querySelector("svg")).toBeNull();
  expect(tips(main).some((one) => one.startsWith("Preview"))).toBe(false);
  expect(tips(main)).toContain("Main e5604ab91c");
  expect(main.getAttribute("aria-label")).toContain(", main,");
});

test("a build began reads as month, day and the clock in its row", () => {
  const host = panel();
  expect(row(host, "c3d4e5f").getAttribute("aria-label")).toContain("c3d4e5f, Nov 3 14:36, main");
  expect(row(host, "7ac3d52").textContent).toContain("Nov 3 09:05");
  expect(row(host, "e5604ab").textContent).toContain("Nov 2 18:00");
});

test("a figure that moved shows its change, and every other figure keeps an empty slot for one", () => {
  const host = panel();
  const chips = (id: string) => [
    ...row(host, id).querySelectorAll<HTMLElement>(".armada-builds__cell"),
  ].map((cell) => cell.querySelector(".armada-builds__chip")!);
  // 7ac3d52 against e5604ab: Checks per run 512 to 731, the only figure of the four that moved.
  const moved = chips("7ac3d52");
  expect(moved).toHaveLength(4);
  const said = (chip: Element) => chip.querySelector(".armada-usage__delta")?.textContent ?? "";
  expect(moved.map(said)).toEqual([
    "",
    "",
    "43%",
    "",
  ]);
  expect(tips(moved[2]!)).toEqual([
    "Checks per run up 43 percent on the build before",
  ]);
  // The build with too few turns, and the oldest build, have nothing to read against, and still keep the four slots.
  for (const id of [
    "c3d4e5f",
    "e5604ab",
  ]) {
    const empty = chips(id);
    expect(empty).toHaveLength(4);
    expect(empty.map(said)).toEqual([
      "",
      "",
      "",
      "",
    ]);
  }
});

test("↑↓ and j k move down the builds, ↵ and o pick the one under the cursor", async () => {
  const picked = vi.fn();
  const host = panel(picked);
  list(host).focus();
  await expect.poll(() => active(host)).toBe("build-c3d4e5f");
  await keys("j");
  await expect.poll(() => active(host)).toBe("build-7ac3d52");
  await keys("{ArrowDown}");
  await expect.poll(() => active(host)).toBe("build-e5604ab");
  await keys("j");
  await expect.poll(() => active(host)).toBe("build-e5604ab");
  await keys("k");
  await expect.poll(() => active(host)).toBe("build-7ac3d52");
  await keys("{ArrowUp}");
  await expect.poll(() => active(host)).toBe("build-c3d4e5f");
  expect(picked).not.toHaveBeenCalled();
  await keys("{ArrowDown}", "{Enter}");
  expect(picked).toHaveBeenLastCalledWith("7ac3d52");
  await keys("j", "o");
  expect(picked).toHaveBeenLastCalledWith("e5604ab");
  expect(picked).toHaveBeenCalledTimes(2);
});

test("c moves focus to the Builds list, whose keys then move its cursor and not the Jobs'", async () => {
  const host = render(<UsagePage time={TIME} builds={BUILDS} />);
  await keys("v");
  await expect.poll(() => host.querySelector('[role="listbox"][aria-label="Slowest Jobs"]')).not.toBeNull();
  const jobs = () =>
    host.querySelector('[role="listbox"][aria-label="Slowest Jobs"]')!.getAttribute("aria-activedescendant");
  await keys("c");
  await expect.poll(() => document.activeElement).toBe(list(host));
  await expect.poll(() => active(host)).toBe("build-c3d4e5f");
  await keys("j");
  await expect.poll(() => active(host)).toBe("build-7ac3d52");
  expect(jobs()).toBeNull();
  await keys("{Enter}");
  await expect.poll(() => row(host, "7ac3d52").getAttribute("aria-selected")).toBe("true");
  expect(host.querySelector('section[aria-label="Split"] .armada-usage__title')?.textContent).toBe(
    "Build 7ac3d52",
  );
});
