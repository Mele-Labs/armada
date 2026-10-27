// Blueprint with no plan: the agent's own tasks in the order made, each its mark and subject, one tab stop moved by the arrows, Home and End.
// Browser: a rendering and a focus are what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import type { AgentTask, Session, SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { AgentTasks } from "./AgentTasks";
import { BlueprintPanel } from "./BlueprintPanel";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
beforeEach(() => localStorage.clear());
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  localStorage.clear();
});

const TASKS: AgentTask[] = [
  { id: "a", subject: "Read the runner", state: "done" },
  { id: "b", subject: "Rename the files", state: "working" },
  { id: "c", subject: "Update the tests", state: "open" },
];

function show(ui: React.ReactElement): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(ui));
  roots.push({ root, host });
  return host;
}

const rows = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>(".sv-agent-tasks__row")];
const press = (key: string) => act(() => void document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true })));

test("the tasks are listed in the order made, each its mark and its subject", () => {
  const host = show(<AgentTasks tasks={TASKS} />);
  expect(host.querySelector('ul[aria-label="Tasks"]')).not.toBeNull();
  expect(rows(host).map((one) => one.querySelector(".sv-agent-tasks__subject")?.textContent)).toEqual(["Read the runner", "Rename the files", "Update the tests"]);
  expect(rows(host).map((one) => one.querySelector(".armada-task-mark")?.getAttribute("data-state"))).toEqual(["done", "working", "open"]);
  expect(rows(host)[1]!.querySelector('[role="img"][aria-label="Working"]')).not.toBeNull();
});

test("the list is one tab stop, and the arrows, Home and End move the focus through it", () => {
  const host = show(<AgentTasks tasks={TASKS} />);
  expect(rows(host).map((one) => one.tabIndex)).toEqual([0, -1, -1]);
  const at = () => rows(host).findIndex((one) => one === document.activeElement);
  act(() => rows(host)[0]!.focus());
  press("ArrowDown");
  expect(at()).toBe(1);
  press("End");
  expect(at()).toBe(2);
  press("ArrowDown");
  expect(at()).toBe(2);
  press("ArrowUp");
  expect(at()).toBe(1);
  press("Home");
  expect(at()).toBe(0);
  press("ArrowUp");
  expect(at()).toBe(0);
  // Tab leaves from where the arrows left it.
  press("End");
  expect(rows(host).map((one) => one.tabIndex)).toEqual([-1, -1, 0]);
});

test("the list keeps its arrows from the deck, and a key it does not move is left alone", () => {
  const host = show(<AgentTasks tasks={TASKS} />);
  expect(host.querySelector("ul")!.hasAttribute("data-own-arrows")).toBe(true);
  act(() => rows(host)[1]!.focus());
  const other = new KeyboardEvent("keydown", { key: "a", bubbles: true, cancelable: true });
  act(() => void document.activeElement?.dispatchEvent(other));
  expect(other.defaultPrevented).toBe(false);
  expect(rows(host).findIndex((one) => one === document.activeElement)).toBe(1);
});

const session = { id: "s1", attachments: [], rows: [], turn: { state: "idle" } } as unknown as Session;

function panel(work: SessionWork, acts?: SessionWorkActs): HTMLElement {
  return show(<BlueprintPanel work={work} place="side" id="s1" session={session} acts={acts} ed={undefined} onOpenSketch={() => undefined} onOpenSession={() => undefined} />);
}

test("Blueprint lists the tasks where there is no plan, and a Session with none stays open with nothing listed, since a panel is folded only by a press", () => {
  const acts = { workflows: [], choose: () => undefined } as unknown as SessionWorkActs;
  const bare = panel({ proposal: { workflow: null } }, acts);
  expect(bare.querySelector<HTMLElement>('section[aria-label="Blueprint"]')!.dataset["collapsed"]).toBeUndefined();
  expect(bare.querySelector('ul[aria-label="Tasks"]')).toBeNull();

  const busy = panel({ proposal: { workflow: null }, tasks: TASKS }, acts);
  expect(busy.querySelector<HTMLElement>('section[aria-label="Blueprint"]')!.dataset["collapsed"]).toBeUndefined();
  expect(rows(busy)).toHaveLength(3);
});
