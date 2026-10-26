// Create mod through `App` on the mock Fleet's `mod-job` scenario: ⌘K starts an empty Mod Job that lists what it can make, an ordinary Job lists
// nothing, the aim he picks arrives with his first message, the mod the Job makes is a card in its thread where only the switch toggles it, and
// the rest of the row opens the mod in the side panel, whose Change it starts another empty Mod Job aimed at it. The walks `createMod` and
// `modCard` play it; this holds what they decided.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const SCENARIO = "mod-job";
const aims = () => page.getByRole("region", { name: "What a Mod Job makes" });
const message = () => page.getByRole("textbox", { name: "Message" });
const thread = () => page.getByRole("region", { name: "Thread", exact: true });
const jobs = () => page.getByRole("region", { name: "Jobs", exact: true });
/** A press on the switch where a person sees it, the track. Its input sits under the track's words, which take the pointer. */
const press = (control: ReturnType<typeof page.getByRole>) => (control.element() as HTMLInputElement).closest("label")!.click();
const palette = () => page.getByRole("dialog", { name: "Command palette" });

/** Create mod from the palette, as a person does: ⌘K, the words, ↵. */
async function createMod() {
  await userEvent.keyboard("{Meta>}k{/Meta}");
  await expect.element(palette()).toBeVisible();
  await userEvent.keyboard("Create mod{Enter}");
  await expect.element(aims()).toBeVisible();
}

test("an ordinary Job lists no aims, and Create mod opens an empty Mod Job that does, the caret in its box", async () => {
  mount(SCENARIO);
  await onScreen();
  await jobs().getByRole("button", { name: "Fix the flaky store test", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Job: Fix the flaky store test" })).toBeVisible();
  expect(aims().query()).toBeNull();

  await createMod();
  await expect.element(message()).toHaveFocus();
  expect(page.getByRole("radio", { checked: true }).query()).toBeNull();
  // An empty Mod Job draws no Armada configuration card.
  expect(page.getByText("Armada configuration").query()).toBeNull();
});

test("the aim he picks goes with his first message", async () => {
  mount(SCENARIO);
  await onScreen();
  await createMod();
  await userEvent.click(page.getByRole("radio", { name: "Colours" }));
  await userEvent.click(message());
  await userEvent.keyboard("Make a dusk theme{Enter}");
  // The mock agent says it is a theme only when the message arrived aimed at colours.
  await expect.element(thread().getByText("Dusk is a theme, saved and on.")).toBeVisible();
  expect(aims().query()).toBeNull();
});

test("the mod the Job made is a card in its thread: the switch toggles it, the row and its name open it, Change it starts the aimed Job", async () => {
  mount(SCENARIO);
  await onScreen();
  await createMod();
  await userEvent.click(message());
  await userEvent.keyboard("Make a dusk theme{Enter}");

  const card = thread().getByRole("group", { name: "Dusk" });
  await expect.element(card).toBeVisible();
  const toggle = card.getByRole("switch", { name: "Dusk" });
  await expect.element(toggle).toBeChecked();

  // The switch alone toggles, and opens nothing.
  press(toggle);
  await expect.element(toggle).not.toBeChecked();
  expect(page.getByRole("dialog", { name: "Mod Dusk" }).query()).toBeNull();
  press(toggle);
  await expect.element(toggle).toBeChecked();

  // The name opens the mod in the side panel, and leaves the switch where it was.
  await card.getByRole("button", { name: "Open Dusk" }).click();
  const panel = page.getByRole("dialog", { name: "Mod Dusk" });
  await expect.element(panel).toBeVisible();
  await expect.element(toggle).toBeChecked();

  // Change it starts an empty Mod Job aimed at colours, the box open on the change.
  await panel.getByRole("button", { name: "Change it" }).click();
  await expect.element(aims()).toBeVisible();
  await expect.element(page.getByRole("radio", { name: "Colours" })).toHaveAttribute("aria-checked", "true");
  await expect.element(message()).toHaveTextContent("Change the Dusk theme:");
  expect(page.getByRole("dialog", { name: "Mod Dusk" }).query()).toBeNull();
});
