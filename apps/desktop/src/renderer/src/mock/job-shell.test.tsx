// The Job's shell, docked where a block's run docks: Terminal in the Job's header opens it at the pane's
// size, keys reach it, Hide leaves it running, End shell stops it and New shell opens another. Beside a
// block Run the two are tabs. Through `App` on `session-shell`, whose Fleet holds a fake zsh. The whole
// story is the walk `job-shell`.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { jobTerminalPlaceOf } from "../job-terminal-place";
import { s322SessionShell, shellCalls } from "./scenarios/session-shell";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const pane = () => page.getByRole("group", { name: "Terminal", exact: true });
const thread = () => page.getByRole("region", { name: "Thread" });
const PROMPT = "slot-1 %";

async function open() {
  mount(s322SessionShell);
  await onScreen();
  const view = await toSession(/Deploy the store/);
  await expect.element(thread()).toBeVisible();
  return view;
}

test("Terminal opens the Job's shell at the pane's size, docked above the box, and keys reach it", async () => {
  const view = await open();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  await view.getByRole("button", { name: "Terminal", exact: true }).click();

  await expect.element(pane()).toBeVisible();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  const asked = shellCalls()!.opened;
  expect(asked).toHaveLength(1);
  // The size the pane fitted, and not Fleet's 80 by 24.
  expect(asked[0]!.cols).toBeGreaterThan(0);
  expect([asked[0]!.cols, asked[0]!.rows]).not.toEqual([80, 24]);
  const box = page.getByRole("textbox", { name: "Message" }).element().getBoundingClientRect();
  expect(pane().element().getBoundingClientRect().bottom).toBeLessThanOrEqual(box.top);

  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await userEvent.keyboard("ls{Enter}");
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();
});

test("Hide leaves the shell running and Terminal brings it back whole; End shell stops it, and New shell opens another", async () => {
  const view = await open();
  await view.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("pwd{Enter}");
  await expect.element(pane().getByText("~/Development/armada/.armada/slots/slot-1", { exact: true })).toBeVisible();

  await pane().getByRole("button", { name: "Hide" }).click();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  expect(shellCalls()!.stopped).toEqual([]);
  await view.getByRole("button", { name: "Terminal", exact: true }).click();
  // The same shell, with what it printed before the pane was put away.
  await expect.element(pane().getByText("~/Development/armada/.armada/slots/slot-1", { exact: true })).toBeVisible();
  expect(shellCalls()!.opened).toHaveLength(1);

  await pane().getByRole("button", { name: "End shell" }).click();
  await expect.element(pane().getByText("Shell ended")).toBeVisible();
  expect(shellCalls()!.stopped).toEqual(["shell-1"]);
  await pane().getByRole("button", { name: "New shell" }).click();
  await expect.element(pane().getByRole("button", { name: "End shell" })).toBeVisible();
  await expect.poll(() => shellCalls()!.opened.length).toBe(2);
  // A fresh screen: the first shell's lines are gone.
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  expect(pane().element().textContent).not.toContain("~/Development/armada/.armada/slots/slot-1");
});

test("A block Run beside the shell: two tabs, the run's named by its command cut short with the whole command on hover, and the Run works as before", async () => {
  const view = await open();
  await view.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();

  await thread().getByRole("group", { name: /wrangler deploy/ }).getByRole("button", { name: "Run", exact: true }).click();
  const shellTab = pane().getByRole("tab", { name: "Terminal" });
  const runTab = pane().getByRole("tab", { name: /^cd ~\/Development.*…$/ });
  await expect.element(runTab).toHaveAttribute("aria-selected", "true");
  await expect.element(pane().getByText("Attempting to login via OAuth...")).toBeVisible();
  await expect.element(runTab).toHaveAccessibleName("cd ~/Development/armada/.armada…");
  await expect.element(runTab).toHaveAccessibleDescription("cd ~/Development/armada/.armada/slots/slot-1/apps/store && pnpm exec wrangler login && pnpm exec wrangler deploy");

  await shellTab.click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await expect.element(pane().getByRole("button", { name: "End shell" })).toBeVisible();
  await runTab.click();
  await pane().getByRole("button", { name: "Stop" }).click();
  await expect.element(pane().getByRole("img", { name: "Stopped" })).toBeVisible();
  // Closing the run leaves the shell, alone, with no tabs.
  await pane().getByRole("button", { name: "Close" }).click();
  await expect.element(pane().getByRole("button", { name: "End shell" })).toBeVisible();
  expect(pane().getByRole("tablist").query()).toBeNull();
});

const SERVED = /Local:.*localhost:5173/;

test("The agent opening the Job's shell shows its pane in front, and the shell ending says so", async () => {
  await open();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  timePasses();

  await expect.element(pane()).toBeVisible();
  await expect.element(pane().getByText(SERVED)).toBeVisible();
  await expect.element(pane().getByRole("button", { name: "End shell" })).toBeVisible();
  expect(shellCalls()!.opened).toEqual([]);
  await userEvent.click(pane().getByRole("textbox", { name: "Terminal input" }));
  await userEvent.keyboard("{Control>}c{/Control}");
  await userEvent.keyboard("exit{Enter}");
  await expect.element(pane().getByText("Shell ended")).toBeVisible();
});

test("A shell the agent opened while another Job was open draws whole once Terminal shows it", async () => {
  mount(s322SessionShell);
  await onScreen();
  await toSession(/Store migration spike/);
  timePasses();
  // The server's first lines, before it prints more and they scroll off the pane.
  await new Promise((done) => window.setTimeout(done, 1000));
  // Not this Job's shell: its view docks nothing.
  expect(document.querySelector(".armada-terminal")).toBeNull();

  const view = await toSession(/Deploy the store/);
  expect(document.querySelector(".armada-terminal")).toBeNull();
  await view.getByRole("button", { name: "Terminal", exact: true }).click();
  // Handed the shell already open, which printed everything before this pane was drawn.
  await expect.element(pane().getByText(SERVED)).toBeVisible();
  await expect.element(pane().getByText(/VITE v6\.3\.5/)).toBeVisible();
  expect(shellCalls()!.opened).toEqual([]);
});

const panel = () => page.getByRole("dialog", { name: "Terminal", exact: true });

async function openInSidePanel() {
  const app = mount(s322SessionShell);
  await onScreen();
  await app.api.saveSettings({ "terminal.openJobTerminalIn": "side panel" });
  const view = await toSession(/Deploy the store/);
  await expect.element(thread()).toBeVisible();
  return view;
}

test("Side panel: Terminal opens the shell in the side panel, nothing docks under the thread, and Close keeps it running with its output", async () => {
  const view = await openInSidePanel();
  await view.getByRole("button", { name: "Terminal", exact: true }).click();

  await expect.element(panel()).toBeVisible();
  await expect.element(panel().getByText(PROMPT)).toBeVisible();
  expect(view.getByRole("group", { name: "Terminal", exact: true }).query()).toBeNull();
  // The panel closes itself: the pane offers no Hide of its own.
  expect(panel().getByRole("button", { name: "Hide" }).query()).toBeNull();
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await userEvent.keyboard("pwd{Enter}");
  await expect.element(panel().getByText("~/Development/armada/.armada/slots/slot-1", { exact: true })).toBeVisible();

  await panel().getByRole("button", { name: /^Close/ }).click();
  await expect.element(panel()).not.toBeInTheDocument();
  expect(shellCalls()!.stopped).toEqual([]);
  await view.getByRole("button", { name: "Terminal", exact: true }).click();
  // The same shell, with what it printed before the panel closed.
  await expect.element(panel().getByText("~/Development/armada/.armada/slots/slot-1", { exact: true })).toBeVisible();
  expect(shellCalls()!.opened).toHaveLength(1);

  await panel().getByRole("button", { name: "End shell" }).click();
  await expect.element(panel().getByText("Shell ended")).toBeVisible();
  expect(shellCalls()!.stopped).toEqual(["shell-1"]);
  await panel().getByRole("button", { name: "New shell" }).click();
  await expect.element(panel().getByRole("button", { name: "End shell" })).toBeVisible();
  await expect.poll(() => shellCalls()!.opened.length).toBe(2);
});

test("Side panel: the agent opening the Job's shell opens the side panel on it, and a block Run still docks", async () => {
  const view = await openInSidePanel();
  timePasses();
  await expect.element(panel().getByText(SERVED)).toBeVisible();
  expect(view.getByRole("group", { name: "Terminal", exact: true }).query()).toBeNull();
  await panel().getByRole("button", { name: /^Close/ }).click();
  await expect.element(panel()).not.toBeInTheDocument();

  await thread().getByRole("group", { name: /wrangler deploy/ }).getByRole("button", { name: "Run", exact: true }).click();
  await expect.element(pane().getByText("Attempting to login via OAuth...")).toBeVisible();
  expect(pane().getByRole("tablist").query()).toBeNull();
});

test("The terminal's toggle moves the open shell between docked and the side panel with all it printed, keeps the focus in it, and saves where it opens", async () => {
  const app = mount(s322SessionShell);
  await onScreen();
  const view = await toSession(/Deploy the store/);
  await view.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("pwd{Enter}");
  const printed = "~/Development/armada/.armada/slots/slot-1";
  await expect.element(pane().getByText(printed, { exact: true })).toBeVisible();
  const placed = async () => jobTerminalPlaceOf((await app.api.state()).settings);

  await pane().getByRole("button", { name: "Move to the side panel" }).click();
  await expect.element(panel().getByText(printed, { exact: true })).toBeVisible();
  expect(view.getByRole("group", { name: "Terminal", exact: true }).query()).toBeNull();
  await expect.poll(() => panel().element().contains(document.activeElement) && document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await expect.poll(placed).toBe("side panel");

  await panel().getByRole("button", { name: "Dock under the thread" }).click();
  await expect.element(panel()).not.toBeInTheDocument();
  await expect.element(pane().getByText(printed, { exact: true })).toBeVisible();
  await expect.poll(() => pane().element().contains(document.activeElement) && document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await expect.poll(placed).toBe("docked");
  // One shell throughout, never stopped.
  expect(shellCalls()!.opened).toHaveLength(1);
  expect(shellCalls()!.stopped).toEqual([]);
});
