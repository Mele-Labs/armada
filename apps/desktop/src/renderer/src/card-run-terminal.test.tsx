// A Needs you card holds its run itself, by the waiting item's key, and hands it to the block. A command
// that asks for something (a passphrase, a yes or no) has to be answerable there, so the held run is drawn
// on a terminal that takes keys while it runs, and keeps what it printed once it ends.

import { RunnableBlock, forgetKeptRuns, useCodeRun, type CodeRunEvent, type CodeRunner } from "@armada/components";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  forgetKeptRuns();
});

function program() {
  const written = vi.fn();
  const resized = vi.fn();
  let on: (event: CodeRunEvent) => void = () => undefined;
  const runner: CodeRunner = (_command, hear) => {
    on = hear;
    return { stop: () => hear({ kind: "end", code: null, stopped: true }), write: written, resize: resized };
  };
  return { runner, written, resized, say: (event: CodeRunEvent) => on(event) };
}

/** The card in little: its own run, held by the item's key, passed to the block as `CallCard` passes it. */
function Card({ runner }: { runner: CodeRunner }) {
  const run = useCodeRun(runner, "waiting-run:s1:w1");
  return (
    <div style={{ width: 640 }}>
      <RunnableBlock command="unlock-key" run={run}>
        <code>unlock-key</code>
      </RunnableBlock>
    </div>
  );
}

function card(runner: CodeRunner) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(<Card runner={runner} />);
}

const pane = () => page.getByRole("group", { name: "Terminal", exact: true });

test("keys typed into the card's run while it runs reach the run, and what it printed stays once it ends", async () => {
  const p = program();
  card(p.runner);
  await page.getByRole("button", { name: "Run", exact: true }).click();
  p.say({ kind: "out", text: "Passphrase: " });

  await expect.element(pane().getByText("Passphrase:")).toBeVisible();
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  // The run started before the screen had a size, so the screen tells it the one it fits.
  await expect.poll(() => p.resized.mock.calls.length).toBeGreaterThan(0);
  expect(p.resized.mock.calls[0]![0]).toBeGreaterThan(20);

  await userEvent.keyboard("open sesame{Enter}");
  await expect.poll(() => p.written.mock.calls.map((call) => call[0]).join("")).toBe("open sesame\r");

  p.say({ kind: "out", text: "\r\nIdentity added\r\n" });
  p.say({ kind: "end", code: 0 });
  await expect.element(pane().getByRole("img", { name: "Passed" })).toBeVisible();
  await expect.element(pane().getByText("Identity added")).toBeVisible();
  await expect.element(pane().getByText("Passphrase:")).toBeVisible();
  // Ended, the screen takes no more keys.
  const before = p.written.mock.calls.length;
  document.querySelector<HTMLElement>('[aria-label="Terminal input"]')!.focus();
  await userEvent.keyboard("x");
  expect(p.written.mock.calls.length).toBe(before);
});
