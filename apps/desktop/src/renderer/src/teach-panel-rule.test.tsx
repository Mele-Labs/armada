// The Rule section of the Teach panel draws a lifecycle rule as the Workflows page does, with When, If and Do to change before it is
// kept, and one line for what Fleet cannot test. `TeachPanel` in `@armada/components`, in a browser.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { TeachPanel } from "@armada/components";
import type { TeachReading } from "@armada/components";
import type { LifecycleRule } from "@armada/protocol";
import { mount, unmount } from "@armada/screens/src/mounted";

afterEach(unmount);

const RULE: LifecycleRule = {
  id: "move_its_tickets_to_done",
  name: "Move its tickets to Done",
  scope: { kind: "within", id: "armada" },
  when: { kind: "enters", lifecycle: "pull_request", state: "approved" },
  conditions: [],
  does: [{ what: { act: "tracker_move", to: "done", target: { among: { kind: "linked_from", lifecycle: "ticket" } } }, kind: "tracker_write", autonomy: "asks" }],
  autonomy: "asks",
  on_failure: { retries: 2, then: "ask_owner" },
};

const LIFECYCLES = [
  { id: "ticket", name: "Ticket", kind: "tracker" as const, states: [{ id: "todo", label: "To do" }, { id: "done", label: "Done" }] },
  { id: "pull_request", name: "Pull request", kind: "forge" as const, states: [{ id: "ready", label: "Ready" }, { id: "approved", label: "Approved" }] },
];

const shown = (rule: Partial<NonNullable<TeachReading["rule"]>> = {}, onRule: (rule: LifecycleRule) => void = () => undefined) =>
  mount(
    <TeachPanel
      source={{ kind: "pull_request", text: "#2712 Pin the clock" }}
      say="When this is approved, move its tickets to Done"
      onSay={() => undefined}
      keep="teach-panel-rule"
      repository="armada"
      reach="repository"
      onReach={() => undefined}
      stage="shown"
      reading={{ rule: { rule: RULE, lifecycles: LIFECYCLES, steps: [], workflows: [], ...rule } }}
      onTeach={() => undefined}
      onRule={onRule}
      onChange={() => undefined}
      onSave={() => undefined}
    />,
  );

test("a rule draws its name, when it fires, what it does and how far it goes alone", async () => {
  shown();
  const section = page.getByRole("region", { name: "Rule" });

  await expect.element(section.getByRole("combobox", { name: "When" })).toHaveDisplayValue("Pull request enters Approved");
  await expect.element(section.getByText("Name", { exact: true })).toBeVisible();
  await expect.element(section.getByRole("combobox", { name: "Move a ticket 1 to" })).toHaveDisplayValue("Done");
  await expect.element(section.getByRole("radio", { name: "Asks" })).toBeChecked();
  await expect.element(section.getByText(/Fleet cannot test/)).not.toBeInTheDocument();
});

test("changing how far it goes alone hands the rule back with the new word on the rule and on each act", async () => {
  const onRule = vi.fn();
  shown({}, onRule);

  // The radio is drawn as a ring over its input, so its word is what is pressed.
  await page.getByRole("region", { name: "Rule" }).getByText("Acts", { exact: true }).click();

  expect(onRule).toHaveBeenCalledTimes(1);
  expect(onRule.mock.calls[0]![0]).toMatchObject({ id: "move_its_tickets_to_done", autonomy: "acts", does: [{ autonomy: "acts" }] });
});

test("what Fleet cannot test is one line under the rule", async () => {
  shown({ untestable: "Fleet cannot test: the build is green" });

  await expect.element(page.getByRole("region", { name: "Rule" }).getByText("Fleet cannot test: the build is green")).toBeVisible();
});

test("a reading with no rule has no Rule section", async () => {
  mount(
    <TeachPanel
      source={{ kind: "pull_request", text: "#2712 Pin the clock" }}
      say="Use pnpm"
      onSay={() => undefined}
      keep="teach-panel-rule"
      repository="armada"
      reach="repository"
      onReach={() => undefined}
      stage="shown"
      reading={{ agents: "Use pnpm." }}
      onTeach={() => undefined}
      onRule={() => undefined}
      onChange={() => undefined}
      onSave={() => undefined}
    />,
  );

  await expect.element(page.getByRole("region", { name: "Agents" })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Rule" })).not.toBeInTheDocument();
});
