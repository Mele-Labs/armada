// The Job's shell as a Terminal tab: + opens it at the pane's size, keys reach it, End shell stops it and New shell opens another, and ⌥⇧0 brings the tab forward or opens one. A shell the agent opens
// draws a Terminal tab, closing the last tab puts the shell away with it still running, and Move to window takes it to a window of its own. Through `App` on `session-shell`, whose Fleet holds a fake zsh.
// The whole story is the walk `job-shell`.

import { beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetPanes } from "../job-panes/store";
import { closeTerminalWindow, s322SessionShell, SHELL_JOB, shellCalls } from "./scenarios/session-shell";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();
// The panes each Job keeps survive a restart, so a test starts with none kept.
beforeEach(() => forgetPanes());

const pane = () => page.getByRole("group", { name: "Terminal", exact: true });
const threadPane = () => page.getByRole("region", { name: "Pane: Thread", exact: true });
const terminalPane = () => page.getByRole("region", { name: /^Pane: (Terminal|zsh)/ });
// The Job's terminal window, which the mock draws on the page as a dialog named as the real window's title.
const popOut = () => page.getByRole("dialog", { name: "Terminal: Deploy the store", exact: true });
const PROMPT = "slot-1 %";
const PRINTED = "~/Development/armada/.armada/slots/slot-1";
const SERVED = /Local:.*localhost:5173/;

async function open() {
  mount(s322SessionShell);
  await onScreen();
  await toSession(/Deploy the store/);
  await expect.element(threadPane()).toBeVisible();
}

/** + on the thread's pane, then Terminal. */
async function addTerminal() {
  await threadPane().getByRole("button", { name: "Add tab" }).click();
  await page.getByRole("menuitem", { name: "Terminal" }).click();
}

test("a Terminal tab opens the Job's shell at the pane's size, and keys reach it", async () => {
  await open();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  await addTerminal();

  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  const asked = shellCalls()!.opened;
  expect(asked).toHaveLength(1);
  // The size the pane fitted, and not Fleet's 80 by 24.
  expect([asked[0]!.cols, asked[0]!.rows]).not.toEqual([80, 24]);
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await userEvent.keyboard("ls{Enter}");
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();
});

test("Ctrl+C in the shell reaches it as the interrupt byte", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await userEvent.keyboard("{Control>}c{/Control}");
  await expect.poll(() => shellCalls()!.written).toContain("\u0003");
  await expect.element(pane().getByText("^C")).toBeVisible();
});

test("End shell stops the shell, and New shell opens another with a fresh screen", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("pwd{Enter}");
  await expect.element(pane().getByText(PRINTED, { exact: true })).toBeVisible();

  await pane().getByRole("button", { name: "End shell" }).click();
  await expect.element(pane().getByText("Shell ended")).toBeVisible();
  expect(shellCalls()!.stopped).toEqual(["shell-1"]);
  await pane().getByRole("button", { name: "New shell" }).click();
  await expect.element(pane().getByRole("button", { name: "End shell" })).toBeVisible();
  await expect.poll(() => shellCalls()!.opened.length).toBe(2);
  expect(pane().element().textContent).not.toContain(PRINTED);
});

test("⌥⇧0 opens a Terminal tab where the Job has none, and brings the pane forward where it has one", async () => {
  await open();
  expect(terminalPane().query()).toBeNull();
  await userEvent.keyboard("{Alt>}{Shift>}[Digit0]{/Shift}{/Alt}");
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  const tabs = () => page.getByRole("tab", { name: /^(Terminal|zsh)/ }).elements().length;
  expect(tabs()).toBe(1);

  await threadPane().getByRole("tab", { name: "Thread" }).click();
  await expect.element(threadPane()).toHaveAttribute("data-focused", "true");
  await userEvent.keyboard("{Alt>}{Shift>}[Digit0]{/Shift}{/Alt}");
  await expect.element(terminalPane()).toHaveAttribute("data-focused", "true");
  // Brought forward, not opened again.
  expect(tabs()).toBe(1);
  expect(shellCalls()!.opened).toHaveLength(1);
});

test("the agent opening the Job's shell draws a Terminal tab, and the shell ending says so", async () => {
  await open();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  timePasses();

  await expect.element(pane()).toBeVisible();
  await expect.element(pane().getByText(SERVED)).toBeVisible();
  await expect.element(pane().getByRole("button", { name: "End shell" })).toBeVisible();
  expect(shellCalls()!.opened).toEqual([]);
  await userEvent.click(pane().getByRole("textbox", { name: "Terminal input" }));
  await userEvent.keyboard("{Control>}c{/Control}");
  await expect.poll(() => shellCalls()!.written).toContain("\u0003");
  await userEvent.keyboard("exit{Enter}");
  await expect.element(pane().getByText("Shell ended")).toBeVisible();
});

test("closing the Terminal tab puts the shell away with it still running, and a new tab shows it whole", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("pwd{Enter}");
  await expect.element(pane().getByText(PRINTED, { exact: true })).toBeVisible();

  await page.getByRole("button", { name: /^Close (Terminal|zsh)/ }).click();
  await expect.element(terminalPane()).not.toBeInTheDocument();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  expect(shellCalls()!.stopped).toEqual([]);

  await addTerminal();
  await expect.element(pane().getByText(PRINTED, { exact: true })).toBeVisible();
  expect(shellCalls()!.opened).toHaveLength(1);
});

test("Move to window takes the shell to a window of its own, its tab says where it is, and Move to the pane or the window's Dock brings it back", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("ls{Enter}");
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();

  await page.getByRole("tab", { name: /^(Terminal|zsh)/ }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Move to window" }).click();
  await expect.element(popOut().getByText(/CONTRIBUTING\.md/)).toBeVisible();
  // One screen sizes the shell: the pane has none while the window has it.
  expect(terminalPane().getByRole("textbox", { name: "Terminal input" }).query()).toBeNull();
  await expect.poll(() => popOut().element().contains(document.activeElement) && document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  expect(shellCalls()!.windows).toEqual([SHELL_JOB]);

  await terminalPane().getByRole("button", { name: "Move to the pane" }).click();
  await expect.element(popOut()).not.toBeInTheDocument();
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();

  await page.getByRole("tab", { name: /^(Terminal|zsh)/ }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Move to window" }).click();
  await expect.element(popOut()).toBeVisible();
  await popOut().getByRole("button", { name: "Dock", exact: true }).click();
  await expect.element(popOut()).not.toBeInTheDocument();
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();

  // The window's own close puts the shell away, and the tab shows it again.
  await page.getByRole("tab", { name: /^(Terminal|zsh)/ }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Move to window" }).click();
  await expect.element(popOut()).toBeVisible();
  closeTerminalWindow(SHELL_JOB);
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();
  expect(shellCalls()!.stopped).toEqual([]);
  expect(shellCalls()!.started).toHaveLength(1);
});
