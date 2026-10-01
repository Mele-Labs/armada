// A link in a model's text, through `App`: the Record draws a failed task's
// reason as markdown, and a link the OS will not open is said on screen.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { scenarioNamed, type Scenario } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const RUN = "https://forge.invalid/armada/actions/runs/7";

/** `arc/group-failed`, with T6's reason carrying a link, on a machine with no browser. */
function refusingALink(): Scenario {
  const base = scenarioNamed("arc/group-failed")!;
  const groups = base.draft!.groups!.map((group) => ({
    ...group,
    tasks: group.tasks.map((task) =>
      task.id === "T6"
        ? { ...task, failed_reason: `The row's press opened the **Board**. See [the run](${RUN}).` }
        : task,
    ),
  }));
  return {
    ...base,
    draft: { ...base.draft!, groups },
    behaves: () => ({
      openLink: async (address) => ({ ok: false, why: "refused", address, detail: "no browser" }),
    }),
  };
}

test("a failed task's reason is markdown on the Record, and a link the OS refuses is said", async () => {
  mount(refusingALink());
  await page.getByRole("tab", { name: /^Record/ }).click();
  const link = page.getByRole("link", { name: "the run" }).first();
  await expect.element(link).toBeVisible();
  await link.click();
  await expect.element(page.getByText(`This machine did not open ${RUN}: no browser`)).toBeVisible();
});
