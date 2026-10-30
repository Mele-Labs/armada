// A rule taught from a pull request, through `App` on the mock Fleet: Fleet reads it, the panel changes it in place, and Save keeps the rule as the
// person left it, in the scope the reach gives and under the id its name makes. The walk `teach-rule` plays the same steps.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { lessonsNow } from "../teach";
import { mount, onScreen, unmountAfterEach } from "./testing";
import { walkThrough } from "./walk";
import { WALKS } from "./walks";

unmountAfterEach();

const steps = () => WALKS.get("teach-rule")!.steps;
const rule = () => page.getByRole("dialog", { name: "Teach" }).getByRole("region", { name: "Rule", exact: true });

test("a pull request taught to move its tickets once approved is shown as a rule, with its fields to change", async () => {
  mount("teach-rule");
  await onScreen();
  // Up to the reading shown: the Job, its pull request's menu, the words, ⌘↵.
  await walkThrough(steps().slice(0, 7));

  await expect.element(rule().getByRole("combobox", { name: "When" })).toHaveDisplayValue("Pull request enters Approved");
  await expect.element(rule().getByRole("combobox", { name: "Move a ticket 1 to" })).toHaveDisplayValue("Done");
  await expect.element(rule().getByRole("radio", { name: "Asks" })).toBeChecked();
  // Only the rule was understood: no note for agents.
  await expect.element(page.getByRole("region", { name: "Agents" })).not.toBeInTheDocument();
});

test("Save keeps the rule as the person changed it, as a rule piece and not a mod", async () => {
  const app = mount("teach-rule");
  await onScreen();
  const saved = vi.spyOn(app.api, "saveTeaching");
  await walkThrough(steps());

  await vi.waitFor(() => expect(saved).toHaveBeenCalledTimes(1));
  expect(saved.mock.calls[0]![0].rule).toMatchObject({ id: "move_its_tickets_to_done", autonomy: "acts", does: [{ autonomy: "acts" }] });
  const done = await saved.mock.results[0]!.value;
  expect(done).toMatchObject({ ok: true, value: { rule: { kind: "rule", id: "move_its_tickets_to_done", scope: { kind: "within", id: "armada" } } } });
  expect(done.value).not.toHaveProperty("mods");
  expect(done.value).not.toHaveProperty("entry");
  expect(lessonsNow()).toHaveLength(1);
  await expect.element(page.getByRole("dialog", { name: "Teach" })).not.toBeInTheDocument();
});
