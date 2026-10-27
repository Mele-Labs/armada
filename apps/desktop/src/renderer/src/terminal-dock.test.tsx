// A command run in a Session's view opens a docked terminal that xterm draws, with the runner contract
// carrying the terminal both ways: the size a run starts at, the keys typed, the size after a fit, a stop.
// The pane is mounted over a runner of the test's own, so what reached it is what is asserted.

import { DockedTerminal, Prose, ProseRunner, ProseTerminal, forgetKeptRuns, useCodeRun, useTerminalDock, type CodeRunEvent, type CodeRunner } from "@armada/components";
import { createRoot, type Root } from "react-dom/client";
import { useState } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  // The pane and its run are kept by the Job they are of, which every test here names alike.
  forgetKeptRuns();
});

/** What the test's runner was asked, and the way to speak as the program on the other end of it. */
function program() {
  const started: { command: string; size: { cols: number; rows: number } | undefined }[] = [];
  const stops = vi.fn();
  const written = vi.fn();
  const resized = vi.fn();
  let on: (event: CodeRunEvent) => void = () => undefined;
  const runner: CodeRunner = (command, hear, size) => {
    started.push({ command, size });
    on = hear;
    return { stop: () => (stops(), hear({ kind: "end", code: null, stopped: true })), write: written, resize: resized };
  };
  return { runner, started, stops, written, resized, prints: (text: string) => on({ kind: "out", text }), exits: (code: number) => on({ kind: "end", code }) };
}

const SOURCE = "Check it:\n\n```sh\nmake test\n```";

/** A Session's view in little: a thread with a block, the dock under it, and a box to leave focus in. */
function View({ runner }: { runner: CodeRunner }) {
  const dock = useTerminalDock(useCodeRun(runner, "dock:s1"), "dock:s1");
  const [width, setWidth] = useState(720);
  return (
    <ProseTerminal.Provider value={{ dock }}>
      <ProseRunner.Provider value={runner}>
        <div style={{ display: "flex", flexDirection: "column", width, height: 480 }}>
          <div style={{ flex: 1, minHeight: 0 }}>
            <Prose text={SOURCE} />
          </div>
          <DockedTerminal dock={dock} />
        </div>
        <button type="button" onClick={() => setWidth(400)}>
          Narrower
        </button>
      </ProseRunner.Provider>
    </ProseTerminal.Provider>
  );
}

function view(runner: CodeRunner) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(<View runner={runner} />);
  return { root, show: () => root.render(<View runner={runner} />), leave: () => root.render(<p>Another Job</p>) };
}

const pane = () => page.getByRole("group", { name: "Terminal", exact: true });

test("Run opens the pane docked, starts the run at the size the pane fits, and gives the screen the keys", async () => {
  const p = program();
  view(p.runner);
  expect(document.querySelector(".armada-terminal")).toBeNull();
  await page.getByRole("button", { name: "Run", exact: true }).click();

  await expect.element(pane()).toBeVisible();
  await expect.poll(() => p.started.length).toBe(1);
  const size = p.started[0]!.size;
  expect(p.started[0]!.command).toBe("make test");
  expect(size?.cols).toBeGreaterThan(20);
  expect(size?.rows).toBeGreaterThan(2);
  await expect.element(pane().getByRole("img", { name: "Running" })).toBeVisible();
  // Focus is in the terminal as it opens, with no click.
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");

  await userEvent.keyboard("ls");
  await expect.poll(() => p.written.mock.calls.map((call) => call[0]).join("")).toBe("ls");
  await userEvent.keyboard("{Enter}");
  expect(p.written).toHaveBeenLastCalledWith("\r");
  await userEvent.keyboard("{Control>}c{/Control}");
  expect(p.written).toHaveBeenLastCalledWith("\u0003");
});

test("what the program prints is drawn with its colours, and a key the terminal takes does not reach the window's shortcuts", async () => {
  const p = program();
  const shortcut = vi.fn();
  window.addEventListener("keydown", shortcut);
  try {
    view(p.runner);
    await page.getByRole("button", { name: "Run", exact: true }).click();
    await expect.poll(() => p.started.length).toBe(1);
    p.prints("\u001b[32mbuilt\u001b[39m \u001b[31mfailed\u001b[39m\r\n");
    await expect.element(pane().getByText("built")).toBeVisible();
    const colour = (word: string) => getComputedStyle([...document.querySelectorAll(".xterm-rows span")].find((one) => one.textContent === word)!).color;
    expect(colour("built")).not.toBe(colour("failed"));
    expect(colour("built")).not.toBe("rgb(0, 0, 0)");

    await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
    shortcut.mockClear();
    await userEvent.keyboard("jg?/{Escape}{Tab}{ArrowUp}{Enter}");
    expect(shortcut.mock.calls.map((call) => (call[0] as KeyboardEvent).key)).toEqual([]);
    await userEvent.keyboard("{Meta>}k{/Meta}");
    expect(shortcut).toHaveBeenCalled();
    // Once the run has ended the keys are the window's again.
    p.exits(0);
    await expect.element(pane().getByRole("img", { name: "Passed" })).toBeVisible();
    shortcut.mockClear();
    await userEvent.keyboard("j");
    expect(shortcut.mock.calls.map((call) => (call[0] as KeyboardEvent).key)).toEqual(["j"]);
  } finally {
    window.removeEventListener("keydown", shortcut);
  }
});

test("a fit that changes the size tells the run", async () => {
  const p = program();
  view(p.runner);
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect.poll(() => p.started.length).toBe(1);
  const was = p.started[0]!.size!;
  await page.getByRole("button", { name: "Narrower" }).click();
  await expect.poll(() => p.resized.mock.calls.length).toBeGreaterThan(0);
  const [cols, rows] = p.resized.mock.calls.at(-1)!;
  expect(cols).toBeLessThan(was.cols);
  expect(rows).toBe(was.rows);
});

test("Stop ends the run, and Run again stops nothing running and starts it afresh at the same size", async () => {
  const p = program();
  view(p.runner);
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect.poll(() => p.started.length).toBe(1);
  p.prints("one\r\n");
  await expect.element(pane().getByText("one")).toBeVisible();

  await pane().getByRole("button", { name: "Stop" }).click();
  expect(p.stops).toHaveBeenCalledTimes(1);
  await expect.element(pane().getByRole("img", { name: "Stopped" })).toBeVisible();
  // Once it has ended a key goes nowhere.
  p.written.mockClear();
  await userEvent.keyboard("x");
  expect(p.written).not.toHaveBeenCalled();

  await pane().getByRole("button", { name: "Run again" }).click();
  await expect.poll(() => p.started.length).toBe(2);
  expect(p.started[1]).toEqual(p.started[0]);
  await expect.element(pane().getByRole("img", { name: "Running" })).toBeVisible();
  // The screen starts over: nothing the first run printed is left on it.
  expect(pane().element().textContent).not.toContain("one");
});

test("Close puts the pane away once the run has ended, and the block's mark brings the same run back", async () => {
  const p = program();
  view(p.runner);
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect.poll(() => p.started.length).toBe(1);
  p.prints("kept\r\n");
  p.exits(2);
  await expect.element(pane().getByRole("img", { name: "Failed" })).toBeVisible();
  expect(pane().getByRole("button", { name: "Stop" }).elements()).toHaveLength(0);

  await pane().getByRole("button", { name: "Close" }).click();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  await page.getByRole("button", { name: "Show terminal" }).click();
  await expect.element(pane()).toBeVisible();
  await expect.element(pane().getByText("kept")).toBeVisible();
  await expect.element(pane().getByRole("img", { name: "Failed" })).toBeVisible();
  expect(p.started).toHaveLength(1);
});

test("leaving the Job leaves the pane's run going, and coming back finds the pane with it still running", async () => {
  const p = program();
  const job = view(p.runner);
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect.element(pane().getByRole("img", { name: "Running" })).toBeVisible();
  p.prints("one-time code 4F2A\r\n");

  job.leave();
  await expect.element(page.getByText("Another Job")).toBeVisible();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  expect(p.stops).not.toHaveBeenCalled();

  job.show();
  await expect.element(pane().getByRole("img", { name: "Running" })).toBeVisible();
  await expect.poll(() => document.querySelector(".armada-xterm")?.textContent ?? "").toContain("one-time code 4F2A");
  // The run is the one that was started: asked for once, and the keys reach it.
  expect(p.started).toHaveLength(1);
  await expect.poll(() => document.activeElement?.getAttribute("aria-label")).toBe("Terminal input");
  await userEvent.keyboard("ls");
  await expect.poll(() => p.written.mock.calls.map((call) => call[0]).join("")).toBe("ls");
  expect(p.stops).not.toHaveBeenCalled();
});

test("a run that ended while its Job was left is found ended, with its exit", async () => {
  const p = program();
  const job = view(p.runner);
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect.poll(() => p.started.length).toBe(1);
  job.leave();
  await expect.element(page.getByText("Another Job")).toBeVisible();
  p.exits(1);
  job.show();
  await expect.element(pane().getByRole("img", { name: "Failed" })).toBeVisible();
  expect(p.started).toHaveLength(1);
});
