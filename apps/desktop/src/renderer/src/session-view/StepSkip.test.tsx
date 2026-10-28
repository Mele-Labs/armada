// Skipping a step from the story: the icon on a current or coming step, the Undo on a skipped one, and the band's menu. Browser: the drawing is what is read.

import { act, type ComponentProps } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { Session, SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { StoryColumn } from "./StoryColumn";
import { resetFolds } from "./story";

// The mod list is Fleet's, read through `window.armada`; no Job here made a mod.
vi.mock("../mod-list", () => ({ useModList: () => [] }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ID = "step-skip-s1";
const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  resetFolds(ID);
  localStorage.clear();
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

// Propose and Discuss are done, the Job is at Walk, Refine is to come, and Deliver is the last step.
const STEPS = ["Propose", "Discuss", "Walk", "Refine", "Deliver"] as const;
const REFINE = 3;

function mount(own: { skipped?: number[]; skip?: SessionWorkActs["skip"] } = {}): HTMLElement {
  const group = { id: "g1", title: "Alpha", agent: "rust-engineer", paths: [], tasks: [{ id: "g1-t", text: "Task of Alpha", state: "open" }] };
  const plan = { scope: [], approved: true, groups: [group] };
  const work = { plan, workflow: { name: "Product", steps: STEPS, at: 2, ...(own.skipped === undefined ? {} : { skipped: own.skipped }) } } as unknown as SessionWork;
  const session = {
    id: ID,
    title: "Rotate the audit log",
    attachments: [],
    turn: { state: "idle" },
    work,
    rows: [{ id: "m1", at: "2026-10-27T09:02:00Z", kind: "message", from: { kind: "you" }, text: "Rotate the audit log" }],
  } as unknown as Session;
  const acts = (own.skip === undefined ? undefined : { skip: own.skip, workflows: [] }) as unknown as SessionWorkActs | undefined;
  const ed = { id: ID, plan, marks: [], set: () => undefined, send: () => undefined } as unknown as ComponentProps<typeof StoryColumn>["ed"];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<StoryColumn session={session} entries={[]} work={work} id={ID} manifestId={undefined} acts={acts} ed={ed} onOpenSketch={() => undefined} onOpenSession={() => undefined} live={undefined} />));
  roots.push({ root, host });
  return host;
}

const zone = (host: HTMLElement, label: string) => host.querySelector<HTMLElement>(`section[aria-label="${label}"]`)!;
const skipIn = (host: HTMLElement, label: string) => zone(host, label).querySelector<HTMLButtonElement>('button[aria-label="Skip step"]');
const undoIn = (host: HTMLElement, label: string) => zone(host, label).querySelector<HTMLButtonElement>('button[aria-label="Undo skip"]');
const items = () => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')];

function rightPress(host: HTMLElement, label: string): void {
  const band = zone(host, label).querySelector<HTMLElement>(".sv-panel__band")!;
  act(() => {
    band.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, composed: true, clientX: 40, clientY: 40, button: 2 }));
  });
}

test("the current step and the steps to come each have a Skip button, and a finished step and the last step have none", () => {
  const host = mount({ skip: () => undefined });
  expect(skipIn(host, "Walk")).not.toBeNull();
  expect(skipIn(host, "Refine")).not.toBeNull();
  for (const name of ["Propose", "Discuss", "Deliver"]) expect(skipIn(host, name), name).toBeNull();
});

test("no step has a Skip button where nothing serves the skip", () => {
  const host = mount();
  for (const name of STEPS) expect(skipIn(host, name), name).toBeNull();
});

test("pressing Skip on a step skips that step by its place in the workflow", () => {
  const skip = vi.fn();
  const host = mount({ skip });
  act(() => skipIn(host, "Refine")!.click());
  expect(skip).toHaveBeenCalledWith(ID, REFINE, true);
  act(() => skipIn(host, "Walk")!.click());
  expect(skip).toHaveBeenLastCalledWith(ID, 2, true);
});

test("a skipped step says Skipped and offers Undo skip in place of Skip step", () => {
  const skip = vi.fn();
  const host = mount({ skipped: [REFINE], skip });
  expect(zone(host, "Refine").textContent).toContain("Skipped");
  expect(zone(host, "Refine").getAttribute("data-skipped")).toBe("true");
  expect(skipIn(host, "Refine")).toBeNull();
  act(() => undoIn(host, "Refine")!.click());
  expect(skip).toHaveBeenCalledWith(ID, REFINE, false);
});

test("the last step offers neither Skip nor Undo, were it listed as skipped", () => {
  const host = mount({ skipped: [4], skip: () => undefined });
  expect(zone(host, "Deliver").textContent).not.toContain("Skipped");
  expect(undoIn(host, "Deliver")).toBeNull();
  expect(skipIn(host, "Deliver")).toBeNull();
});

test("a right press on a step's band offers Skip step, and choosing it skips the step", async () => {
  const skip = vi.fn();
  const host = mount({ skip });
  rightPress(host, "Refine");
  await vi.waitFor(() => expect(items().map((one) => one.textContent)).toContain("Skip step"));
  act(() => items().find((one) => one.textContent === "Skip step")!.click());
  expect(skip).toHaveBeenCalledWith(ID, REFINE, true);
});

test("a right press on a skipped step's band offers Undo skip, and choosing it undoes the skip", async () => {
  const skip = vi.fn();
  const host = mount({ skipped: [REFINE], skip });
  rightPress(host, "Refine");
  await vi.waitFor(() => expect(items().map((one) => one.textContent)).toContain("Undo skip"));
  expect(items().map((one) => one.textContent)).not.toContain("Skip step");
  act(() => items().find((one) => one.textContent === "Undo skip")!.click());
  expect(skip).toHaveBeenCalledWith(ID, REFINE, false);
});

test("a right press on a finished step's band offers no Skip step", async () => {
  const host = mount({ skip: () => undefined });
  rightPress(host, "Propose");
  await new Promise((done) => setTimeout(done, 50));
  expect(items().map((one) => one.textContent)).not.toContain("Skip step");
});
