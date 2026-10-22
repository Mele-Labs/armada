// A skill's SKILL.md opened from its Loadout row sits beside the Loadout and is the one a person reaches: nothing is
// drawn over it, so a press on its Close lands on that Close. Played to that step by the walk `session-loadout`.

import { expect, onTestFinished, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { walkThrough } from "./walk";
import { WALKS } from "./walks";

unmountAfterEach();

test("the SKILL.md Edit opens beside the Loadout is on top, and shows the skill's file", async () => {
  const script = WALKS.get("session-loadout")!;
  await page.viewport(1512, 949);
  onTestFinished(() => page.viewport(1440, 900));
  mount(script.scenario);
  await onScreen();
  await walkThrough(script.steps.slice(0, script.steps.findIndex((one) => one.say.startsWith("Its SKILL.md opens")) + 1));

  const file = page.getByRole("dialog", { name: "SKILL.md" });
  const close = file.getByRole("button", { name: "Close" }).element();
  const box = close.getBoundingClientRect();
  // What a press at the middle of Close reaches: Close itself, not a scrim laid over it.
  const reached = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
  expect(close.contains(reached), `a press there reaches ${reached?.className}`).toBe(true);
  // The skill's own text, in the editor (drawn in a shadow root), rather than a file the pretend disk made up from its name.
  await expect.poll(() => file.getByRole("group", { name: "Editor SKILL.md" }).element().shadowRoot?.textContent ?? "").toContain("Write the question down in one sentence");
  await expect.element(page.getByRole("dialog", { name: "Loadout" })).toBeInTheDocument();
}, 60_000);
