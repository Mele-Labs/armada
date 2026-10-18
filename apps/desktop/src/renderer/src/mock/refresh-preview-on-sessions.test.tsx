// A restart a Session hands the owner is one key on the Session's own page, as it is on its call card:
// it reads as its label, copies nothing, and runs the build switch once. An ordinary command beside it
// keeps its block. Through `App` on `refresh-preview-on-sessions`, with the Session's runner held so a
// test says when and how a run ends.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { CodeRunEvent } from "@armada/components";

import { s241RefreshPreviewOnSessions } from "./scenarios/refresh-preview-on-sessions";
import type { SessionsStore } from "./sessions/script";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const RESTART = "scripts/preview --restart";
const TEST = "pnpm test";

type Board = Parameters<NonNullable<NonNullable<typeof s241RefreshPreviewOnSessions.draft>["sessions"]>>[0];

const needs = () => page.getByRole("region", { name: "Needs you" });
const TEST_ROW = /^Open Run the tests/;
const key = () => needs().getByRole("button", { name: "Refresh preview" });

/**
 * The Session's page, with its Needs you panel open when `list` is set. The restart is a key on the panel's band, pressed folded; the ordinary
 * command is in the list. The runner is read when the page draws, so what it does is held before that: every run it is asked for waits in
 * `runs` for the test to end it, and a clean exit settles nothing, so the key stays for a second press. `runner` false is a Session with no
 * way to run a command.
 */
async function open({ runner, list = false }: { runner: boolean; list?: boolean }) {
  const runs: { command: string; on: (event: CodeRunEvent) => void }[] = [];
  const scenario = {
    ...s241RefreshPreviewOnSessions,
    draft: {
      ...s241RefreshPreviewOnSessions.draft,
      sessions: (board: Board) => {
        const store = s241RefreshPreviewOnSessions.draft!.sessions!(board);
        if (!runner) {
          const { run: _run, ...rest } = store;
          return rest as SessionsStore;
        }
        vi.spyOn(store, "run").mockImplementation((_id, command, on) => {
          runs.push({ command, on });
          return { stop: () => undefined };
        });
        vi.spyOn(store, "answerWaiting").mockImplementation(() => undefined);
        return store;
      },
    },
  };
  mount(scenario);
  await onScreen();
  await toSession(/Pin the store clock/);
  if (list) await userEvent.click(needs().getByRole("button", { name: /^Needs you/ }));
  return runs;
}

/** A press the key ignores is still a press: it is held with `aria-disabled`, which leaves it focusable and in the tab order. */
async function press() {
  key().element().focus();
  await userEvent.keyboard("{Enter}");
}

test("a restart reads its label, with no command to read, no Command fact and nothing copied", async () => {
  await open({ runner: true });
  const copied = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
  await expect.element(key()).toBeVisible();
  expect(needs().getByRole("group", { name: RESTART }).elements()).toHaveLength(0);
  // What hovering the row shows is the command alone: no card with a Command fact.
  const said = document.getElementById(key().element().getAttribute("aria-describedby") ?? "");
  expect(said?.textContent).toBe(RESTART);
  await userEvent.click(key());
  expect(copied).not.toHaveBeenCalled();
});

test("pressing it runs the restart once, not again while it runs or after it passes, and again after a failure", async () => {
  const runs = await open({ runner: true });

  await userEvent.click(key());
  await expect.poll(() => runs.length).toBe(1);
  expect(runs[0]?.command).toBe(RESTART);
  await expect.element(needs().getByRole("img", { name: "Running" })).toBeVisible();
  await press();
  expect(runs).toHaveLength(1);

  runs[0]!.on({ kind: "end", code: 1 });
  await expect.element(needs().getByRole("img", { name: "Exit 1" })).toBeVisible();
  await userEvent.click(key());
  await expect.poll(() => runs.length).toBe(2);

  runs[1]!.on({ kind: "end", code: 0 });
  await expect.element(needs().getByRole("img", { name: "Passed" })).toBeVisible();
  await press();
  expect(runs).toHaveLength(2);
});

test("an ordinary command beside it keeps its command and its Run", async () => {
  const runs = await open({ runner: true, list: true });
  const block = needs().getByRole("group", { name: TEST });
  await expect.element(block.getByText(TEST)).toBeVisible();
  await userEvent.click(block.getByRole("button", { name: "Run" }));
  await expect.poll(() => runs.length).toBe(1);
  expect(runs[0]?.command).toBe(TEST);
});

test("with nothing to run a command by, the restart shows its label and does nothing, and the command is still copied", async () => {
  await open({ runner: false });
  const copied = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);
  await expect.element(key()).toBeVisible();
  await press();
  expect(copied).not.toHaveBeenCalled();
  await userEvent.click(needs().getByRole("button", { name: /^Needs you/ }));
  // In the list, the command's own row is the press that copies.
  await userEvent.click(needs().getByRole("button", { name: TEST_ROW }));
  expect(copied).toHaveBeenCalledWith(TEST);
});
