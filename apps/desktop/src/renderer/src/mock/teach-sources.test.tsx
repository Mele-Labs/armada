// Teach opens from everything it is offered on: selected words, a command, a Fleet notice, a plan task, a Check, a pull request, a
// panel, a row and the title bar. Each one opens the Teach panel on its own source. The walks `teach-anything` and `teach-mods` play
// most of them end to end; this holds each, and the two they do not reach.

import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { TeachHereContext, provideTeaching } from "../teach";
import { usePullRequestMenu } from "../pull-request-menu";
import { forgetPanel, usePanel } from "../shared-panel/store";
import { mount, onScreen, unmountAfterEach } from "./testing";
import { kit, toSessions } from "./sessions/walk-kit";
import { ARMADA_TITLE, INSTALL, KEY_MESSAGE, PAYMENTS_TITLE, TASK } from "./sessions/teach";
import { button, deckRow, dialog, inside, region, role, text, walkThrough } from "./walk";
import type { Step } from "./walk";

unmountAfterEach();
afterEach(() => forgetPanel());

const panel = dialog("Teach");
const section = (name: string) => inside(panel, region(name, { exact: true }));
const menuItem = role("menuitem", "Teach");

/** The steps from the Cockpit to the thing Teach is pressed on, and the caption the panel opens with. */
function reaching(): [string, Step[], string][] {
  const { thread } = kit(false);
  const key = inside(thread, text(KEY_MESSAGE.slice(0, 40)));
  const now = region("Now", { exact: true });
  const first: Step[] = [toSessions, { press: deckRow(ARMADA_TITLE), say: "" }];
  return [
    ["words", [...first, { select: "STORE-1234", in: key, say: "" }, { press: inside(role("toolbar", "Selection"), button("Teach")), say: "" }], "Words"],
    ["command", [...first, { press: inside(thread, button(/Tool calls/)), say: "" }, { context: inside(thread, text(INSTALL)), say: "" }, { press: menuItem, say: "" }], "Command"],
    ["notice", [...first, { key: "Shift+F10", on: inside(thread, button(/commits behind main/)), say: "" }, { press: menuItem, say: "" }], "Notice"],
    ["task", [...first, { press: inside(now, button("Whole plan")), say: "" }, { press: inside(now, button("Show the clock", { exact: true })), say: "" }, { hover: inside(now, text(TASK)), say: "" }, { press: inside(now, button(`Teach on task ${TASK}`)), say: "" }], "Task"],
    ["check", [...first, { context: inside(thread, button(/Check desktop_unit, failed/)), say: "" }, { press: menuItem, say: "" }], "Check"],
    ["panel", [...first, { key: "Shift+F10", on: inside(region("Field", { exact: true }), button("Expand")), say: "" }, { press: menuItem, say: "" }], "Panel Field"],
    ["row", [toSessions, { context: deckRow(PAYMENTS_TITLE), say: "" }, { press: menuItem, say: "" }], "Row"],
  ];
}

for (const [source, steps, caption] of reaching()) {
  test(`Teach on ${source} opens the Teach panel on it`, async () => {
    mount("teach-anything");
    await onScreen();
    await walkThrough([...steps, { look: section(caption), say: "" }]);
  });
}

const rightPressTitleBar = (): void => {
  const bar = document.querySelector(".armada-title-bar")!;
  bar.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, composed: true, view: window, clientX: 40, clientY: 10, button: 2 }));
};

test("Teach on the title bar opens the Teach panel on it", async () => {
  mount("teach-anything");
  await onScreen();
  rightPressTitleBar();
  await userEvent.click(page.getByRole("menuitem", { name: "Teach" }));
  await expect.element(page.getByRole("region", { name: "Title bar", exact: true })).toBeVisible();
});

test("a window that offers no Teach has no entry on the title bar", async () => {
  mount("teach-anything");
  await onScreen();
  act(() => provideTeaching(false));
  rightPressTitleBar();
  await expect.element(page.getByRole("menuitem", { name: "Teach" })).not.toBeInTheDocument();
});

test("Teach on a pull request opens the Teach panel on it, in the Job's repository", async () => {
  provideTeaching(true);
  const shown: unknown[] = [];
  function Probe() {
    const pull = usePullRequestMenu("armada", 12, "Round once");
    shown.push(usePanel().open);
    return (
      <>
        <button type="button" {...pull.on}>
          Pull request
        </button>
        {pull.menu}
      </>
    );
  }
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  await act(async () =>
    root.render(
      <TeachHereContext.Provider value={{ sessionId: "01JOB", jobTitle: "Round", manifestId: "armada", root: "/code/armada", repository: "armada" }}>
        <Probe />
      </TeachHereContext.Provider>,
    ),
  );
  await act(async () => void host.querySelector("button")!.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, composed: true, view: window, clientX: 20, clientY: 20, button: 2 })));
  await userEvent.click(page.getByRole("menuitem", { name: "Teach" }));
  await vi.waitFor(() => expect(shown.at(-1)).toMatchObject({ kind: "teach", source: "pull_request", sessionId: "01JOB", manifestId: "armada", root: "/code/armada" }));
  await act(async () => root.unmount());
  host.remove();
});
