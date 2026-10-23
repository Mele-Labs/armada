// What a taught Logbook entry also changes is read from the mods Fleet lists: a line for each Words or Fleet mod the teaching made,
// and Forget on the entry has Fleet remove its mods, which empties the line and the Mods page of them. Over `teach-mods`, which starts
// with a Jira key and a batched notice taught.

import { expect, test } from "vitest";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { button, dialog, inside, place, region, role, text, walkThrough } from "./walk";

unmountAfterEach();

const entries = role("listbox", "Entries");
const also = inside(region("Entry"), role("group", "Also changes"));

const modsListed = async (): Promise<string[]> => {
  const change = await window.armada.syncState();
  const mods = "full" in change ? (change.full as { mods: { mods: { name: string }[] } | null }).mods : null;
  return (mods?.mods ?? []).map((one) => one.name);
};

test("the Jira key entry lists the Words mod it made, and Forget takes the mod with it", async () => {
  mount("teach-mods");
  await onScreen();
  await walkThrough([
    { press: place("Logbook"), say: "" },
    { press: inside(entries, text(/Tickets are Jira keys/)), say: "" },
    { look: inside(also, text("Jira keys link to each repository's site")), say: "" },
  ]);
  expect(await modsListed()).toContain("jira-keys");

  await walkThrough([
    { key: "f", on: entries, say: "" },
    { press: inside(dialog("Forget this entry"), button("Forget", { exact: true })), say: "" },
  ]);
  await expect.poll(modsListed).not.toContain("jira-keys");
  expect(await modsListed()).toContain("slot-behind-batched");
});

test("the batched notice's entry lists the Fleet mod, worded by what it does and where", async () => {
  mount("teach-mods");
  await onScreen();
  await walkThrough([
    { press: place("Logbook"), say: "" },
    { press: inside(entries, text(/Slot behind main notices arrive batched/)), say: "" },
    { look: inside(also, text(/batched for this repository/)), say: "" },
  ]);
});
