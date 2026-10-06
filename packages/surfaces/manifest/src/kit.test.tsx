// The Kit surface, through `App` — the MCP servers a person has connected, and
// which of them a Drone dispatched against the picked repository is handed.
// #1275. A rail surface at `⌘8` since the owner reversed its first placement as
// a Manifest tab.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import {
  allowlistRead,
  DRIFT_GONE,
  GH_ISSUE_VIEW,
  KIT_SERVERS,
  manifesting,
  mount,
  NOT_READ_ALLOWLIST,
  pressable,
  unmountAfterEach,
} from "@armada/desktop/mock";
import type { Manifesting } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Manifest only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "manifest"] } as const;

/** The Kit surface, by the rail. */
async function kit(options: Manifesting = {}): Promise<void> {
  mount(manifesting(options), SLICES);
  await page.getByRole("button", { name: "Kit", exact: true }).click();
}

/**
 * #1275's own claim, end to end and without a terminal: a person adds an MCP
 * server, sees that it reaches no Drone, allows it for this repository, and the
 * row says a Drone dispatched here gets it.
 */
test("a server added in Bridge reaches no Drone until this repository allows it", async () => {
  await kit();
  await expect.element(page.getByText(/Nothing in your Kit yet/)).toBeVisible();

  await userEvent.fill(page.getByRole("textbox", { name: "Name" }), "nexus");
  await userEvent.fill(page.getByRole("textbox", { name: "Program" }), "npx -y @scope/server");
  await page.getByRole("button", { name: "Add" }).click();

  await expect.element(page.getByText("npx -y @scope/server")).toBeVisible();
  await expect.element(page.getByText("Does not")).toBeVisible();

  await userEvent.selectOptions(page.getByRole("combobox", { name: "Here: nexus" }), "extended");
  await expect.element(page.getByText("Gets it")).toBeVisible();

  // And taking the server out takes this repository's word with it.
  await page.getByRole("button", { name: "Remove nexus" }).click();
  await expect.element(page.getByText(/Nothing in your Kit yet/)).toBeVisible();
});

/**
 * **Both tiers on one screen**, which is why Kit is one surface rather than a
 * machine-wide list and a per-repository one: the column that says what a Drone
 * gets is only readable beside the two answers it is over.
 */
test("Kit's default and this repository's word are read side by side", async () => {
  await kit({ alwaysAllowed: [GH_ISSUE_VIEW], drift: DRIFT_GONE, kitServers: KIT_SERVERS });

  // Kit leaves `tracker` off and this repository allowed it; Kit turned
  // `nexus` on everywhere and this repository withheld it.
  await expect.element(page.getByRole("combobox", { name: "Here: tracker" })).toHaveValue("extended");
  await expect.element(page.getByRole("combobox", { name: "In Kit: nexus" })).toHaveValue("yes");
  await expect.element(page.getByRole("combobox", { name: "Here: nexus" })).toHaveValue("restricted");
  expect(page.getByText("Gets it").elements()).toHaveLength(1);
});

/**
 * Kit is machine-wide and its second tier is a repository's, so on All
 * repositories there is nothing to narrow. The surface asks for one rather than
 * drawing a control that answers for nobody — the Manifest surface's own
 * arrangement.
 */
test("All repositories asks for one rather than drawing a tier that answers for nobody", async () => {
  await kit({ kitServers: KIT_SERVERS, picked: false });
  await expect.element(page.getByText(/Pick a repository to see what its Drones are handed/)).toBeVisible();

  // **The ask takes the servers' place and not the screen.** What a person
  // already has is this machine's, so it answers with no repository picked —
  // #1491. The second tier is the only half that needs one.
  await expect.element(page.getByText("What you already have")).toBeVisible();
  await expect.element(page.getByText("humanizer", { exact: true })).toBeVisible();
});

/**
 * #1491's own claim: the screen opens showing what this person already has,
 * rather than asking them to type it in again. The counts are read from their
 * harness's own home, and a file that would not read is named rather than
 * quietly left out of them.
 */
test("Kit opens on the setup a person already works with", async () => {
  await kit();

  await expect.element(page.getByText("What you already have")).toBeVisible();
  await expect.element(page.getByText("humanizer", { exact: true })).toBeVisible();
  await expect.element(page.getByText("code-simplifier@official")).toBeVisible();
  await expect.element(page.getByText(/would not read/)).toBeVisible();

  // A kind nothing reads yet says so rather than drawing as empty, which is
  // what made a full home directory read as an empty Kit.
  await expect.element(page.getByText(/Not read yet/).first()).toBeVisible();
});

/**
 * **Seeing a server is not granting one.** A server the person connected
 * outside Armada is drawn, and the Kit list beside it is still empty — so no
 * Drone dispatched here is handed it, and allowing it stays a second act.
 */
test("a server Armada can see is not a server a Drone gets", async () => {
  await kit();

  await expect.element(page.getByText("gitnexus", { exact: true })).toBeVisible();
  await expect.element(page.getByText(/A drone here is handed none of them/)).toBeVisible();
  await expect.element(page.getByText(/Nothing in your Kit yet/)).toBeVisible();
  expect(page.getByText("Gets it").elements()).toHaveLength(0);

  // The row says what it is at — enough to tell two servers apart — and never
  // the arguments, the query or the environment that follow it. #1491.
  await expect.element(page.getByText("gitnexus-mcp")).toBeVisible();
});

/**
 * **Two homes, and the screen says which is which** — the owner's decision, 18
 * Sep. Above is his own, which Armada reads and he edits where it lives; below
 * is Armada's own, which is changed here.
 */
test("Kit names the home each half came from", async () => {
  await kit();

  await expect.element(page.getByText("What you already have")).toBeVisible();
  await expect.element(page.getByText("An agent CLI")).toBeVisible();
  await expect.element(page.getByText("What Armada holds")).toBeVisible();
  await expect.element(page.getByText(/edited where it lives/)).toBeVisible();
});

/**
 * **A person reaches the bottom of Kit by the wheel.** The shell's mount is
 * bounded and never scrolls, so a screen that names no scroller of its own
 * clips — and a press in a test has Playwright scroll that clipped box for it,
 * which is how Kit's last row went out of reach at an 856px window while every
 * test above stayed green.
 */
test("at a short window, the wheel brings Kit's last control on screen", async () => {
  await page.viewport(1440, 856);
  await kit({ kitServers: KIT_SERVERS });
  const own = page.getByRole("region", { name: "What Armada holds" });
  await expect.element(own.getByRole("combobox", { name: "Here: nexus" })).toBeInTheDocument();
  const controls = own.element().querySelectorAll("button, select, input");
  const last = controls[controls.length - 1]!;
  expect(pressable(last)).toBe(false);

  await userEvent.wheel(page.getByText("What you already have"), { delta: { y: 400 }, times: 10 });
  await expect.poll(() => pressable(last)).toBe(true);
});

/** The allowlist's own row on the Kit page. */
const allowlist = () => page.getByRole("region", { name: "Allowlist" });

/**
 * Fleet reads the allowlist now, so the row draws each command with where it
 * came from in words, and a Remove that takes one out: the row goes, and what
 * Fleet was asked for is the line as it was spelled.
 */
test("the allowlist draws each command with where it came from, and Remove takes one out", async () => {
  const removed: string[] = [];
  await kit({
    kitInventory: allowlistRead([
      { name: "grep -n", source: "retro item 01M2LESSON3GREPASKED" },
      { name: "gh issue view", source: "always allow" },
      { name: "make check", source: "written by hand" },
    ]),
    onRemoveKitAllowed: (run) => removed.push(run),
  });

  const row = (command: string) => allowlist().getByRole("listitem").filter({ hasText: command });
  await expect.element(row("grep -n")).toBeVisible();
  await expect.element(row("grep -n").getByText("Retro item", { exact: true })).toBeVisible();
  await expect.element(row("gh issue view").getByText("Always allow", { exact: true })).toBeVisible();
  await expect.element(row("make check").getByText("Written by hand", { exact: true })).toBeVisible();
  // A command is the machine's own value, so it is drawn in the mono face.
  expect(row("grep -n").element().querySelector(".mono")?.textContent).toBe("grep -n");

  await allowlist().getByRole("button", { name: "Remove grep -n" }).click();
  await expect.poll(() => allowlist().getByRole("listitem").filter({ hasText: "grep -n" }).elements().length).toBe(0);
  expect(removed).toEqual(["grep -n"]);
  // The others stand.
  await expect.element(row("gh issue view")).toBeVisible();
  await expect.element(row("make check")).toBeVisible();
});

/** **No count beside the list, and nothing standing in for an empty one.** */
test("the allowlist draws no count, and an empty one draws no sentence", async () => {
  await kit({ kitInventory: allowlistRead([{ name: "grep -n", source: "always allow" }]) });
  await expect.element(allowlist().getByRole("listitem")).toBeVisible();
  expect(allowlist().element().querySelector(".armada-kit-setup__count")).toBeNull();
});

test("an allowlist that was read and holds nothing draws its name and nothing under it", async () => {
  await kit({ kitInventory: allowlistRead([]) });
  await expect.element(allowlist().getByRole("heading", { name: "Allowlist" })).toBeVisible();
  expect(allowlist().getByRole("listitem").elements()).toHaveLength(0);
  expect(allowlist().element().querySelector("p")).toBeNull();
});

test("an allowlist file that will not read keeps its not read and why", async () => {
  await kit({ kitInventory: NOT_READ_ALLOWLIST });
  await expect.element(allowlist().getByText(/Not read yet — allowed-commands would not read/)).toBeVisible();
  expect(allowlist().getByRole("button", { name: /^Remove/ }).elements()).toHaveLength(0);
});
