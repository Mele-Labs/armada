// A layout mod arranges the Cockpit as it is drawn today: the panels it hides leave the deck, the Jobs list stays, and an open Job's
// panels stand in the right column in the order it gives. Through `App` on `one-session-view`, with the mod put in the mock's source.

import { act } from "react";
import { expect, test } from "vitest";
import { page } from "vitest/browser";
import { parseLayout } from "@armada/shell";

import { mockLayout } from "./layout";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const panels = (name: string) => document.querySelectorAll(`section[aria-label="${name}"]`);

const install = (text: string) => act(() => mockLayout.install({ name: "tidy", title: "Tidy", enabled: true, file: parseLayout(text).file }));

test("a hidden Activity, Bays or Flight control is not drawn, and the Jobs list is", async () => {
  mount("one-session-view");
  await onScreen();
  await expect.element(list()).toBeVisible();
  expect(panels("Activity")).toHaveLength(1);
  expect(panels("Bays")).toHaveLength(1);
  expect(panels("Flight controls")).toHaveLength(1);
  install('{"version":1,"dashboard.panels":{"hidden":["activity","bays","controls","list"]}}');
  await expect.poll(() => panels("Activity").length).toBe(0);
  expect(panels("Bays")).toHaveLength(0);
  expect(panels("Flight controls")).toHaveLength(0);
  await expect.element(list()).toBeVisible();
});

test("an open Job's panels stand in the order the layout gives", async () => {
  mount("one-session-view");
  await onScreen();
  await expect.element(list()).toBeVisible();
  install('{"version":1,"job.panels":{"order":["blueprint","field"]}}');
  await list().getByRole("button", { name: "Queue session for later", exact: true }).click();
  await expect.poll(() => panels("Blueprint").length).toBe(1);
  const field = page.getByRole("region", { name: "Field", exact: true }).element();
  const blueprint = page.getByRole("region", { name: "Blueprint", exact: true }).element();
  expect(blueprint.compareDocumentPosition(field) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
});
