// The steps a window plays are read off what Fleet kept, and re-held to Fleet's shape: anything that is
// not a list of steps as `show_window` takes them reads as none, so the player is handed no more than
// Fleet would have accepted. `docs/practices/capture-window.md`, *Steps Bridge plays*.

import type { SessionRecord } from "@armada/protocol";
import { expect, it } from "vitest";

import { readSteps, stepsOfWindow } from "./session-file";

const GOOD = [
  { say: "The table", look: { role: "table", name: "Stock" } },
  { say: "Open the tab", press: { role: "tab", name: "Receiving" } },
  { say: "Name it", type: "Bolts\n", into: { role: "textbox", name: "Item" } },
  { say: "A key", key: "Meta+n", on: { role: "textbox" } },
  { say: "Words", select: "bolts", in: { role: "text", name: "Twelve washers" } },
  { say: "Carry", drag: { role: "button", name: "Add", exact: true }, by: { x: 20, y: -5 } },
  { say: "Inside", hover: { role: "button", name: "Close", within: { role: "dialog", name: "Settings" } } },
  { say: "Paste", paste: { role: "textbox" } },
  { say: "Later", later: { role: "status" } },
];

it("reads good steps whole and in order", () => {
  expect(readSteps(GOOD)).toEqual(GOOD);
});

it("reads anything but a list as none", () => {
  for (const bad of [undefined, null, "steps", 3, {}, [null], ["press"]]) expect(readSteps(bad)).toEqual([]);
});

it("reads a list with one bad step as none: a window plays what Fleet would have accepted, or nothing", () => {
  const bad: unknown[] = [
    { say: "", look: { role: "table" } },
    { say: "No verb" },
    { say: "Two", press: { role: "button" }, look: { role: "button" } },
    { say: "No target", press: {} },
    { say: "Text with no words", look: { role: "text" } },
    { say: "Type with no into", type: "x" },
    { say: "Key with no on", key: "Tab" },
    { say: "Drag with no by", drag: { role: "button" } },
    { say: "Drag far", drag: { role: "button" }, by: { x: "1", y: 2 } },
    { say: "Name is a number", look: { role: "button", name: 4 } },
    { say: "Exact is a word", look: { role: "button", exact: "yes" } },
  ];
  for (const one of bad) expect(readSteps([GOOD[0], one])).toEqual([]);
});

it("holds the nesting and the count Fleet holds", () => {
  let deep: Record<string, unknown> = { role: "form" };
  for (let level = 0; level < 5; level += 1) deep = { role: "form", within: deep };
  expect(readSteps([{ say: "Deep", look: deep }])).toEqual([]);
  expect(readSteps(Array.from({ length: 201 }, () => GOOD[0]))).toEqual([]);
  expect(readSteps(Array.from({ length: 200 }, () => GOOD[0]))).toHaveLength(200);
});

it("drops a key the shape does not have rather than passing it to the player", () => {
  const read = readSteps([{ say: "s", look: { role: "table", onclick: "x" }, script: "x" }]);
  expect(JSON.stringify(read)).not.toContain("onclick");
  expect(JSON.stringify(read)).not.toContain("script");
});

const AT = "2026-10-15T09:00:00Z";
const URL_ = "http://localhost:5173/depot";
const record = (detail: Record<string, string>): SessionRecord => ({
  id: "a",
  harness: "h",
  origin: "bridge",
  cwd: "/repo",
  state: "live",
  started_at: AT,
  last_seen_at: AT,
  usage: {},
  attachments: [{ kind: "artifact", target: URL_, state: "standing", detail, since: AT, changed_at: AT }],
  waiting_for: [],
});

it("finds the steps a Session showed in the window's artifact on the ledger, for that address", () => {
  const kept = record({ form: "window", steps: JSON.stringify(GOOD) });
  expect(stepsOfWindow(kept, URL_)).toEqual(GOOD);
  expect(stepsOfWindow(kept, "http://localhost:5173/other")).toEqual([]);
  expect(stepsOfWindow(undefined, URL_)).toEqual([]);
});

it("reads a window with no steps, an artifact that is not a window, or text that is not JSON, as none", () => {
  expect(stepsOfWindow(record({ form: "window" }), URL_)).toEqual([]);
  expect(stepsOfWindow(record({ form: "sketch", steps: JSON.stringify(GOOD) }), URL_)).toEqual([]);
  expect(stepsOfWindow(record({ form: "window", steps: "{not json" }), URL_)).toEqual([]);
});
