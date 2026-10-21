// The Kit pages of the System sidebar, through `App`: a list of what a person has and the picked item beside it.
// Servers are Kit's own and the ones connected outside Armada; the allowlist is the one list with a Remove.
// #1275, #1491.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import {
  allowlistRead,
  DRIFT_GONE,
  GH_ISSUE_VIEW,
  KIT_SERVERS,
  manifesting,
  mount,
  NOT_READ_ALLOWLIST,
  PLUGIN_KIT_INVENTORY,
  toSystemPage,
  unmountAfterEach,
} from "@armada/desktop/mock";
import type { Manifesting } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Manifest only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "manifest"] } as const;
// A Kit item's file is read through the file panel's own reads, so the one test that shows it adds that slice.
const WITH_FILES = { slices: ["core", "manifest", "files"] } as const;

/** A Kit entry of the System sidebar. */
async function kit(entry: string, options: Manifesting = {}, slices: typeof SLICES | typeof WITH_FILES = SLICES): Promise<void> {
  mount(manifesting(options), slices);
  await toSystemPage(entry);
}

const list = (name: string) => page.getByRole("region", { name, exact: true });
const detail = () => page.getByRole("region", { name: "Detail", exact: true });

/**
 * #1275's own claim, end to end and without a terminal: a person adds an MCP server, sees that it reaches no
 * Drone, allows it for this repository, and the row says a Drone dispatched here gets it.
 */
test("a server added in Bridge reaches no Drone until this repository allows it", async () => {
  await kit("Servers");

  await list("Servers").getByRole("button", { name: "Add server" }).click();
  await userEvent.fill(page.getByRole("textbox", { name: "Name" }), "nexus");
  await userEvent.fill(page.getByRole("textbox", { name: "Program" }), "npx -y @scope/server");
  await detail().getByRole("button", { name: "Add" }).click();

  await expect.element(detail().getByText("npx -y @scope/server")).toBeVisible();
  await expect.element(detail().getByText("Does not")).toBeVisible();

  await userEvent.selectOptions(page.getByRole("combobox", { name: "Here: nexus" }), "extended");
  await expect.element(detail().getByText("Gets it")).toBeVisible();

  // And taking the server out takes this repository's word with it.
  await detail().getByRole("button", { name: "Remove nexus" }).click();
  await expect.poll(() => list("Servers").getByRole("button", { name: /^nexus/ }).elements().length).toBe(0);
});

/** **Both tiers on one page**: the control that says what a Drone gets is only readable beside the two answers it is over. */
test("Kit's default and this repository's word are read side by side", async () => {
  await kit("Servers", { alwaysAllowed: [GH_ISSUE_VIEW], drift: DRIFT_GONE, kitServers: KIT_SERVERS });

  // Kit leaves `tracker` off and this repository allowed it; Kit turned `nexus` on everywhere and this repository withheld it.
  await list("Servers").getByRole("button", { name: /^tracker/ }).click();
  await expect.element(page.getByRole("combobox", { name: "Here: tracker" })).toHaveValue("extended");
  await list("Servers").getByRole("button", { name: /^nexus/ }).click();
  await expect.element(page.getByRole("combobox", { name: "In Kit: nexus" })).toHaveValue("yes");
  await expect.element(page.getByRole("combobox", { name: "Here: nexus" })).toHaveValue("restricted");
  await expect.element(detail().getByText("Does not")).toBeVisible();
});

/** On All repositories there is nothing to narrow, so the page draws Kit's own tier and no control that answers for nobody. */
test("All repositories draws no Here control", async () => {
  await kit("Servers", { kitServers: KIT_SERVERS, picked: false });
  await list("Servers").getByRole("button", { name: /^nexus/ }).click();
  await expect.element(page.getByRole("combobox", { name: "In Kit: nexus" })).toBeVisible();
  expect(page.getByRole("combobox", { name: "Here: nexus" }).elements()).toHaveLength(0);
});

/**
 * #1491's own claim: the page opens showing what this person already has, rather than asking them to type it in
 * again, and a file that would not read is named rather than quietly left out.
 */
test("Skills lists what a person already works with", async () => {
  await kit("Skills");
  await expect.element(list("Skills").getByRole("button", { name: /^humanizer/ })).toBeVisible();
  await expect.element(list("Skills").getByRole("button", { name: /^impact-analysis/ })).toBeVisible();
});

/**
 * **Seeing a server is not granting one.** A server connected outside Armada is listed, and a Drone dispatched
 * here is handed none of it: its detail draws no tier to set.
 */
test("a server Armada can see is not a server a Drone gets", async () => {
  await kit("Servers", { kitServers: KIT_SERVERS });
  await list("Servers").getByRole("button", { name: /^gitnexus/ }).click();
  await expect.element(detail().getByText("gitnexus-mcp")).toBeVisible();
  expect(page.getByRole("combobox", { name: /^(In Kit|Here): gitnexus/ }).elements()).toHaveLength(0);
});

/**
 * A skill's detail is its SKILL.md, shown whole in the editor. The two acts beside it leave Bridge for that file, so
 * what main is handed is the file's path, as a Kit file, and which of the two was pressed.
 */
test("a skill's detail is its SKILL.md, and the acts beside it name that file", async () => {
  await kit("Skills", {}, WITH_FILES);
  const left = vi.spyOn(window.armada, "openPanelFileIn");
  await list("Skills").getByRole("button", { name: /^humanizer/ }).click();
  await expect.element(detail().getByText("/Users/user/.agent/skills/humanizer")).toBeVisible();
  await expect.element(detail().getByRole("group", { name: "Editor SKILL.md", exact: true })).toBeVisible();
  expect(detail().getByRole("button", { name: "Open", exact: true }).elements()).toHaveLength(0);

  await detail().getByRole("button", { name: "Open in your editor" }).click();
  await detail().getByRole("button", { name: "Show in Finder" }).click();
  const skill = { kind: "file", path: "/Users/user/.agent/skills/humanizer/SKILL.md", from: "kit" };
  expect(left.mock.calls).toEqual([
    [skill, "editor"],
    [skill, "finder"],
  ]);
  left.mockRestore();
});

/** A plugin is a folder with no file to show, so Show in Finder names the folder and is the only way out. */
test("a plugin offers Show in Finder alone, naming its folder", async () => {
  await kit("Plugins", { kitInventory: PLUGIN_KIT_INVENTORY });
  const opened = vi.spyOn(window.armada, "openKitFile");
  await list("Plugins").getByRole("button", { name: /^armada/ }).click();
  expect(detail().getByRole("group", { name: /^Editor / }).elements()).toHaveLength(0);
  await detail().getByRole("button", { name: "Show in Finder" }).click();
  expect(opened.mock.calls).toEqual([["/Users/user/.agent/plugins/cache/armada/1.0.0", "finder"]]);
  opened.mockRestore();
});

/**
 * Fleet reads the allowlist, so the list names each command and the detail says where it came from in words, with a
 * Remove that takes one out: the row goes, and what Fleet was asked for is the line as it was spelled.
 */
test("the allowlist names where each command came from, and Remove takes one out", async () => {
  const removed: string[] = [];
  await kit("Allowlist", {
    kitInventory: allowlistRead([
      { name: "grep -n", source: "retro item 01M2LESSON3GREPASKED" },
      { name: "gh issue view", source: "always allow" },
      { name: "make check", source: "written by hand" },
    ]),
    onRemoveKitAllowed: (run) => removed.push(run),
  });

  const row = (command: string) => list("Allowlist").getByRole("button", { name: new RegExp(`^${command}`) });
  await expect.element(row("grep -n")).toBeVisible();
  await expect.element(row("grep -n").getByText("Retro item", { exact: true })).toBeVisible();
  await expect.element(row("gh issue view").getByText("Always allow", { exact: true })).toBeVisible();
  await expect.element(row("make check").getByText("Written by hand", { exact: true })).toBeVisible();

  await row("grep -n").click();
  await detail().getByRole("button", { name: "Remove grep -n" }).click();
  await expect.poll(() => row("grep -n").elements().length).toBe(0);
  expect(removed).toEqual(["grep -n"]);
  // The others stand.
  await expect.element(row("gh issue view")).toBeVisible();
  await expect.element(row("make check")).toBeVisible();
});

test("an allowlist that was read and holds nothing lists nothing", async () => {
  await kit("Allowlist", { kitInventory: allowlistRead([]) });
  await expect.element(list("Allowlist")).toBeVisible();
  expect(list("Allowlist").getByRole("listitem").elements()).toHaveLength(0);
});

test("an allowlist file that will not read keeps its not read and why", async () => {
  await kit("Allowlist", { kitInventory: NOT_READ_ALLOWLIST });
  await expect.element(list("Allowlist").getByText(/allowed-commands would not read/)).toBeVisible();
  expect(page.getByRole("button", { name: /^Remove/ }).elements()).toHaveLength(0);
});
