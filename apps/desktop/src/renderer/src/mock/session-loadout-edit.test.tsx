// A skill's SKILL.md opened from its Loadout row sits beside the Loadout and is the one a person reaches: nothing is
// drawn over it, so a press on its Close lands on that Close. Its markdown Preview has no room beside both, so the
// Loadout steps aside while the Preview is open and comes back when it closes. Played to Edit by the walk
// `session-loadout`.

import { expect, onTestFinished, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { walkThrough } from "./walk";
import { WALKS } from "./walks";

unmountAfterEach();

/** Whether a press at the middle of `element` reaches it, rather than a scrim laid over it. */
function reachable(element: Element): string {
  const box = element.getBoundingClientRect();
  const reached = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
  return element.contains(reached) ? "reached" : `a press there reaches ${reached?.className} of ${reached?.querySelector(".armada-sheet")?.getAttribute("data-kind")}`;
}

/** The session-loadout scenario, played to the step that opens deep-research's SKILL.md from its row. */
async function editedFromTheLoadout(): Promise<void> {
  const script = WALKS.get("session-loadout")!;
  await page.viewport(1512, 949);
  onTestFinished(() => page.viewport(1440, 900));
  mount(script.scenario);
  await onScreen();
  await walkThrough(script.steps.slice(0, script.steps.findIndex((one) => one.say.startsWith("Its SKILL.md opens")) + 1));
}

test("the SKILL.md Edit opens beside the Loadout is on top, and shows the skill's file", async () => {
  await editedFromTheLoadout();
  const file = page.getByRole("dialog", { name: "SKILL.md" });
  expect(reachable(file.getByRole("button", { name: "Close" }).element())).toBe("reached");
  // The skill's own text, in the editor (drawn in a shadow root), rather than a file the pretend disk made up from its name.
  await expect.poll(() => file.getByRole("group", { name: "Editor SKILL.md" }).element().shadowRoot?.textContent ?? "").toContain("Write the question down in one sentence");
  await expect.element(page.getByRole("dialog", { name: "Loadout" })).toBeInTheDocument();
}, 60_000);

test("the Loadout steps aside while the SKILL.md's Preview is open, and comes back when it closes", async () => {
  await editedFromTheLoadout();
  const file = page.getByRole("dialog", { name: "SKILL.md" });
  await userEvent.click(file.getByRole("button", { name: "Preview" }));

  const preview = page.getByRole("dialog", { name: "Preview" });
  await expect.element(preview).toBeVisible();
  await expect.poll(() => page.getByRole("dialog", { name: "Loadout" }).elements().length).toBe(0);
  expect(reachable(file.getByRole("button", { name: "Close" }).element())).toBe("reached");
  expect(reachable(preview.getByRole("button", { name: "Close" }).element())).toBe("reached");
  // The file at the trailing edge and the Preview to its left, as they sit with no Loadout open.
  expect(preview.element().getBoundingClientRect().right).toBeLessThanOrEqual(file.element().getBoundingClientRect().left);

  await userEvent.click(preview.getByRole("button", { name: "Close" }));
  await expect.element(page.getByRole("dialog", { name: "Loadout" })).toBeVisible();
  await expect.element(file).toBeVisible();
  expect(reachable(file.getByRole("button", { name: "Close" }).element())).toBe("reached");
}, 60_000);
