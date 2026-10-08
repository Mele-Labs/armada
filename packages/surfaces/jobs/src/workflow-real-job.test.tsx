// The Workflow tab on a Job shaped the way a real Fleet serves it.
//
// **The first real Job drew an empty canvas** (owner, 1 Oct 2026): Job 1, a
// `refactor` held at its plan, three steps on the wire and the tab saying so,
// and not one card on the board. Every screen since 29 Sep had been reviewed
// on the mock, which renders a Job once. A real one renders again on every
// read, every event and every guide card that opens, and React Flow takes a
// node's size off the node it is handed: the run, rebuilt fresh on every
// render, handed it nodes with none, so each render hid every card until it
// was measured again — and a render landing on the first measurement hid them
// for good. `GraphCanvas.tsx` keeps the sizes now.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { refactorAtItsPlan } from "./fake";
import { mount, onJob, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Jobs only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "jobs"] } as const;

/** The three steps `GET /jobs/1` served, by the labels the workflow file declares. */
const STEPS = ["Scope the refactor", "Restructure", "Review the change"];

const card = (label: string) => page.getByRole("button", { name: new RegExp(`^${label}, `) }).last();

/** Every step's card that is not drawn, by its label. */
const hidden = (): string[] =>
  STEPS.filter((label) => {
    const one = card(label).query();
    return one === null || !one.checkVisibility({ visibilityProperty: true });
  });

test("the canvas draws one card per step of a real Job, and a render after it hides none of them", async () => {
  const fixture = refactorAtItsPlan();
  const app = mount(onJob(fixture), SLICES);
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
  for (const label of STEPS) await expect.element(card(label)).toBeVisible();
  // The corner counts from 1, as the cards do; Fleet's `ordinal` counts from 0.
  await expect.element(page.getByText("step 1", { exact: true }).last()).toBeVisible();

  // **What a live Job does all the time**: Fleet answers the Job's read
  // again, and the screen renders again with the same run. Anything hidden
  // between that render and the next frame is a card a person loses.
  const seen = new Set<string>();
  const watching = new MutationObserver(() => {
    for (const label of hidden()) seen.add(label);
  });
  watching.observe(document.body, { subtree: true, attributes: true, attributeFilter: ["style"] });
  await app.api.watchJob(fixture.job.id);
  await new Promise((frame) => requestAnimationFrame(() => requestAnimationFrame(frame)));
  watching.disconnect();
  expect([...seen]).toEqual([]);
  expect(hidden()).toEqual([]);
});
