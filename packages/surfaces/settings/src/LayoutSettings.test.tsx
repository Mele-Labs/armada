// Settings → Layout as drawn: each group lists what the Cockpit and an open Job draw today, and each place once.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, unmount } from "@armada/screens/src/mounted";

import { LayoutSettings } from "./LayoutSettings";

afterEach(unmount);

const rows = async (group: string) => {
  await expect.element(page.getByRole("group", { name: group, exact: true })).toBeInTheDocument();
  return [...page.getByRole("group", { name: group, exact: true }).element().querySelectorAll<HTMLElement>(".armada-layout-row")].map((one) => one.getAttribute("aria-label"));
};

test("the Cockpit's panels and an open Job's panels are the ones they draw", async () => {
  mount(<LayoutSettings />);
  expect(await rows("Cockpit panels")).toEqual(["Jobs", "Field", "Merge queue", "Activity", "Bays", "Flight control", "Dispatch", "Autopilot"]);
  expect(await rows("Job panels")).toEqual(["Done", "Now", "Open"]);
  expect(page.getByRole("group", { name: "Job tabs", exact: true }).elements()).toHaveLength(0);
});

test("each place is listed once", async () => {
  mount(<LayoutSettings />);
  const places = await rows("Places");
  expect(places).toEqual([...new Set(places)]);
});
