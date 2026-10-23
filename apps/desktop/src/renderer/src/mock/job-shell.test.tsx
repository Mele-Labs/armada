// The Job's shell, docked: Terminal in Flight control opens it at the pane's size, keys reach it, Hide
// leaves it running, End shell stops it and New shell opens another. In the Cockpit it docks in a panel
// of its own under the Job, and a block Run docks in the Job. Through `App` on `session-shell`, whose
// Fleet holds a fake zsh. The whole story is the walk `job-shell`.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { jobTerminalPlaceOf } from "../job-terminal-place";
import { closeTerminalWindow, s322SessionShell, SHELL_JOB, shellCalls } from "./scenarios/session-shell";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const pane = () => page.getByRole("group", { name: "Terminal", exact: true });
const thread = () => page.getByRole("region", { name: "Thread" });
const terminal = () => page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Terminal", exact: true });
// The Cockpit's panel for the shell: a pop-out drawn on the same page has a pane of its own, which this leaves out.
const docked = () => page.getByRole("region", { name: "Terminal", exact: true }).getByRole("group", { name: "Terminal", exact: true });
// The Job's terminal window, which the mock draws on the page as a dialog named as the real window's title.
const popOut = () => page.getByRole("dialog", { name: "Terminal: Deploy the store", exact: true });
const PROMPT = "slot-1 %";

async function open() {
  mount(s322SessionShell);
  await onScreen();
  const view = await toSession(/Deploy the store/);
  await expect.element(thread()).toBeVisible();
  return view;
}

test("Terminal opens the Job's shell at the pane's size, docked under the Job, and keys reach it", async () => {
  const view = await open();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  await terminal().click();

  await expect.element(pane()).toBeVisible();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  const asked = shellCalls()!.opened;
  expect(asked).toHaveLength(1);
  // The size the pane fitted, and not Fleet's 80 by 24.
  expect(asked[0]!.cols).toBeGreaterThan(0);
  expect([asked[0]!.cols, asked[0]!.rows]).not.toEqual([80, 24]);
  expect(pane().element().getBoundingClientRect().top).toBeGreaterThanOrEqual(view.element().getBoundingClientRect().bottom);

  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await userEvent.keyboard("ls{Enter}");
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();
});

test("Ctrl+C in the shell reaches it as the interrupt byte", async () => {
  await open();
  await terminal().click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");

  await userEvent.keyboard("{Control>}c{/Control}");
  await expect.poll(() => shellCalls()!.written).toContain("\u0003");
  await expect.element(pane().getByText("^C")).toBeVisible();
});

test("Terminal is Flight control's and not the header's, lit while the shell shows, and a second press hides it with the shell still running", async () => {
  const view = await open();
  expect(view.getByRole("button", { name: "Terminal", exact: true }).query()).toBeNull();
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "false");
  await terminal().click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "true");

  await terminal().click();
  await expect.element(pane()).not.toBeInTheDocument();
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "false");
  expect(shellCalls()!.stopped).toEqual([]);
});

test("Hide leaves the shell running and Terminal brings it back whole; End shell stops it, and New shell opens another", async () => {
  await open();
  await terminal().click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("pwd{Enter}");
  await expect.element(pane().getByText("~/Development/armada/.armada/slots/slot-1", { exact: true })).toBeVisible();

  await pane().getByRole("button", { name: "Hide" }).click();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  expect(shellCalls()!.stopped).toEqual([]);
  await terminal().click();
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

test("In the Cockpit a block Run docks in the Job and the shell keeps its own panel: two panes, no tabs, and the Run works as before", async () => {
  const view = await open();
  await terminal().click();
  const shell = () => page.getByRole("region", { name: "Terminal", exact: true }).getByRole("group", { name: "Terminal", exact: true });
  await expect.element(shell().getByText(PROMPT)).toBeVisible();

  await thread().getByRole("group", { name: /wrangler deploy/ }).getByRole("button", { name: "Run", exact: true }).click();
  const run = view.getByRole("group", { name: "Terminal", exact: true });
  await expect.element(run.getByText("Attempting to login via OAuth...")).toBeVisible();
  await expect.element(shell().getByRole("button", { name: "End shell" })).toBeVisible();
  expect(run.getByRole("tablist").query()).toBeNull();
  expect(shell().getByRole("tablist").query()).toBeNull();
  await run.getByRole("button", { name: "Stop" }).click();
  await expect.element(run.getByRole("img", { name: "Stopped" })).toBeVisible();
  await run.getByRole("button", { name: "Close" }).click();
  await expect.element(run).not.toBeInTheDocument();
  await expect.element(shell().getByRole("button", { name: "End shell" })).toBeVisible();
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

  await toSession(/Deploy the store/);
  expect(document.querySelector(".armada-terminal")).toBeNull();
  await terminal().click();
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
  await terminal().click();

  await expect.element(panel()).toBeVisible();
  await expect.element(panel().getByText(PROMPT)).toBeVisible();
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "true");
  expect(view.getByRole("group", { name: "Terminal", exact: true }).query()).toBeNull();
  // The panel closes itself: the pane offers no Hide of its own.
  expect(panel().getByRole("button", { name: "Hide" }).query()).toBeNull();
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await userEvent.keyboard("pwd{Enter}");
  await expect.element(panel().getByText("~/Development/armada/.armada/slots/slot-1", { exact: true })).toBeVisible();

  await panel().getByRole("button", { name: /^Close/ }).click();
  await expect.element(panel()).not.toBeInTheDocument();
  expect(shellCalls()!.stopped).toEqual([]);
  await terminal().click();
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

test("Side panel: Ctrl+C in the shell reaches it as the interrupt byte", async () => {
  await openInSidePanel();
  await terminal().click();
  await expect.element(panel().getByText(PROMPT)).toBeVisible();
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await userEvent.keyboard("{Control>}c{/Control}");
  await expect.poll(() => shellCalls()!.written).toContain("\u0003");
});

test("Agent's running command: Ctrl+C reaches it as the interrupt byte", async () => {
  await open();
  timePasses();
  await expect.element(pane().getByText(SERVED)).toBeVisible();
  await userEvent.click(pane().getByRole("textbox", { name: "Terminal input" }));
  await userEvent.keyboard("{Control>}c{/Control}");
  await expect.poll(() => shellCalls()!.written).toContain("\u0003");
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
  await terminal().click();
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

  await panel().getByRole("button", { name: "Dock", exact: true }).click();
  await expect.element(panel()).not.toBeInTheDocument();
  await expect.element(pane().getByText(printed, { exact: true })).toBeVisible();
  await expect.poll(() => pane().element().contains(document.activeElement) && document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await expect.poll(placed).toBe("docked");
  // One shell throughout, never stopped.
  expect(shellCalls()!.opened).toHaveLength(1);
  expect(shellCalls()!.stopped).toEqual([]);
});

async function openInWindow() {
  const app = mount(s322SessionShell);
  await onScreen();
  await app.api.saveSettings({ "terminal.openJobTerminalIn": "window" });
  const view = await toSession(/Deploy the store/);
  await expect.element(thread()).toBeVisible();
  return { app, view };
}

test("Window: Terminal opens the Job's shell in a window of its own, lit while it is open, a second press brings it to the front, and closing it puts the shell away", async () => {
  await openInWindow();
  await terminal().click();
  await expect.element(popOut()).toBeVisible();
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "true");
  // A press brings the window forward rather than hiding the shell, and the tile says so.
  const tip = () => {
    const ids = terminal().element().getAttribute("aria-describedby") ?? "";
    return ids.split(" ").map((id) => document.getElementById(id)?.textContent ?? "").join(" ").trim();
  };
  await expect.poll(tip).toBe("Bring the Job's terminal window to the front");
  // One place at a time: nothing docks and no panel opens.
  expect(docked().query()).toBeNull();
  expect(panel().query()).toBeNull();

  // The window opening the shell is heard by this one, which asks for the window again: main raises it.
  const asked = shellCalls()!.windows.length;
  await terminal().click();
  await expect.poll(() => shellCalls()!.windows.length).toBe(asked + 1);
  expect(new Set(shellCalls()!.windows)).toEqual(new Set([SHELL_JOB]));
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "true");

  closeTerminalWindow(SHELL_JOB);
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "false");
  expect(shellCalls()!.stopped).toEqual([]);
});

test("Window: the docked pane and the side panel each open the shell in a window, putting their own away, and save where it opens", async () => {
  const app = mount(s322SessionShell);
  await onScreen();
  await toSession(/Deploy the store/);
  const placed = async () => jobTerminalPlaceOf((await app.api.state()).settings);
  await terminal().click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();

  await pane().getByRole("button", { name: "Open in a window" }).click();
  await expect.element(docked()).not.toBeInTheDocument();
  await expect.poll(() => shellCalls()!.windows).toEqual([SHELL_JOB]);
  await expect.poll(placed).toBe("window");

  closeTerminalWindow(SHELL_JOB);
  await app.api.saveSettings({ "terminal.openJobTerminalIn": "side panel" });
  await expect.poll(placed).toBe("side panel");
  await terminal().click();
  await expect.element(panel().getByText(PROMPT)).toBeVisible();
  await panel().getByRole("button", { name: "Open in a window" }).click();
  await expect.element(panel()).not.toBeInTheDocument();
  await expect.poll(() => shellCalls()!.windows).toEqual([SHELL_JOB, SHELL_JOB]);
  await expect.poll(placed).toBe("window");
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "true");
  expect(shellCalls()!.started).toEqual(["shell-1"]);
});

test("Window: a shell moved back from its window docks, or opens in the side panel, with all it printed", async () => {
  const app = mount(s322SessionShell);
  await onScreen();
  await toSession(/Deploy the store/);
  await terminal().click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("pwd{Enter}");
  const printed = "~/Development/armada/.armada/slots/slot-1";
  await expect.element(pane().getByText(printed, { exact: true })).toBeVisible();

  await pane().getByRole("button", { name: "Open in a window" }).click();
  await expect.element(docked()).not.toBeInTheDocument();
  await app.api.moveTerminalBack(SHELL_JOB, "docked");
  await expect.element(pane().getByText(printed, { exact: true })).toBeVisible();
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "true");

  await pane().getByRole("button", { name: "Open in a window" }).click();
  await expect.element(docked()).not.toBeInTheDocument();
  await app.api.moveTerminalBack(SHELL_JOB, "side panel");
  await expect.element(panel().getByText(printed, { exact: true })).toBeVisible();
  expect(shellCalls()!.started).toEqual(["shell-1"]);
  expect(shellCalls()!.stopped).toEqual([]);
});

test("Window: a shell that ended in its window moves back ended, docked or in the side panel, with what it printed, and no new shell opens", async () => {
  const app = mount(s322SessionShell);
  await onScreen();
  await toSession(/Deploy the store/);
  await terminal().click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("pwd{Enter}");
  const printed = "~/Development/armada/.armada/slots/slot-1";
  await expect.element(pane().getByText(printed, { exact: true })).toBeVisible();
  await pane().getByRole("button", { name: "Open in a window" }).click();
  await expect.element(docked()).not.toBeInTheDocument();
  // End shell, pressed in the window.
  await app.api.stopSessionRun("shell-1");

  await app.api.moveTerminalBack(SHELL_JOB, "docked");
  await expect.element(pane().getByText("Shell ended")).toBeVisible();
  await expect.element(pane().getByText(printed, { exact: true })).toBeVisible();
  // A window opened on a shell that has ended is a new document: it opens a new shell, as main's does, and that one ends in its turn.
  await pane().getByRole("button", { name: "Open in a window" }).click();
  await expect.element(docked()).not.toBeInTheDocument();
  await expect.poll(() => shellCalls()!.started).toEqual(["shell-1", "shell-2"]);
  await expect.element(popOut().getByRole("button", { name: "End shell" })).toBeVisible();
  await app.api.stopSessionRun("shell-2");
  await expect.element(popOut().getByText("Shell ended")).toBeVisible();
  await app.api.moveTerminalBack(SHELL_JOB, "side panel");
  await expect.element(panel().getByText("Shell ended")).toBeVisible();
  expect(popOut().query()).toBeNull();
});

test("Window: the pop-out is drawn as a window of its own with the docked shell's output, keeps the shell running when closed, and shows all it printed when opened again", async () => {
  await open();
  await terminal().click();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("ls{Enter}");
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();

  await pane().getByRole("button", { name: "Open in a window" }).click();
  await expect.element(popOut().getByText(/CONTRIBUTING\.md/)).toBeVisible();
  await expect.element(docked()).not.toBeInTheDocument();
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "true");
  await expect.poll(() => popOut().element().contains(document.activeElement) && document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");

  await userEvent.keyboard("pwd{Enter}");
  const printed = "~/Development/armada/.armada/slots/slot-1";
  await expect.element(popOut().getByText(printed, { exact: true })).toBeVisible();

  // A second press brings the one window forward: no second window, and the shell is not put away.
  await terminal().click();
  await expect.element(popOut()).toBeVisible();
  expect(page.getByRole("dialog", { name: /^Terminal: / }).elements()).toHaveLength(1);

  await popOut().getByRole("button", { name: "Close window" }).click();
  await expect.element(popOut()).not.toBeInTheDocument();
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "false");
  expect(shellCalls()!.stopped).toEqual([]);

  await terminal().click();
  await expect.element(popOut().getByText(/CONTRIBUTING\.md/)).toBeVisible();
  await expect.element(popOut().getByText(printed, { exact: true })).toBeVisible();

  // Moving back closes the window, as main does.
  await popOut().getByRole("button", { name: "Dock", exact: true }).click();
  await expect.element(popOut()).not.toBeInTheDocument();
  await expect.element(docked().getByText(printed, { exact: true })).toBeVisible();
  expect(shellCalls()!.stopped).toEqual([]);
});

test("Window: the agent opening the Job's shell opens its window, and docks nothing", async () => {
  await openInWindow();
  timePasses();
  await expect.poll(() => shellCalls()!.windows).toEqual([SHELL_JOB]);
  await expect.element(terminal()).toHaveAttribute("aria-pressed", "true");
  expect(docked().query()).toBeNull();
});
