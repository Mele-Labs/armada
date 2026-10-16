// The Plan panel's Ask for an update: it stands in the band, and pressing it does not fold the panel.
// Browser: a rendering is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import type { WorkPlan } from "@armada/screens/src/draft/sessions";

import { PlanPanel } from "./PlanPanel";
import type { PlanEditing } from "./PlanEdit";

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

const plan = { scope: [], approved: false, groups: [{ id: "g1", title: "Retry", agent: "rust-engineer", paths: [], tasks: [{ id: "t1", text: "Retry the lease", state: "open" }] }] } as unknown as WorkPlan;
const ed: PlanEditing = { id: "p1", plan, marks: [], set: () => undefined, send: () => undefined };

function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<PlanPanel id="s1" plan={plan} place="side" ed={ed} onOpenSketch={() => undefined} />));
  roots.push({ root, host });
  return host;
}

test("Ask for an update is a button in the band, and pressing it leaves the panel open", () => {
  const host = mount();
  const section = host.querySelector<HTMLElement>('section[aria-label="Plan"]')!;
  const ask = section.querySelector<HTMLButtonElement>('header button[aria-label="Ask for an update"]');
  expect(ask).not.toBeNull();
  act(() => ask!.click());
  expect(section.dataset["collapsed"]).toBeUndefined();
});
