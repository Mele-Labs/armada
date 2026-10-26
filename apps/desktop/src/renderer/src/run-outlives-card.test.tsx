// A run a card started belongs to what asked for it, not to the card: the card leaving the screen leaves
// the run going, and one that comes back under the same key finds it. Only Stop, or a start, ends it.

import { RunnableBlock, type CodeRunEvent, type CodeRunner } from "@armada/components";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

/** The program on the other end of the runner: what it was asked and the way to speak as it. */
function program() {
  const stops = vi.fn();
  const started: string[] = [];
  let on: (event: CodeRunEvent) => void = () => undefined;
  const runner: CodeRunner = (command, hear) => {
    started.push(command);
    on = hear;
    return { stop: () => (stops(), hear({ kind: "end", code: null, stopped: true })) };
  };
  return { runner, stops, started, say: (event: CodeRunEvent) => on(event) };
}

function card(runner: CodeRunner, keep: string | undefined) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const show = () =>
    root.render(
      <RunnableBlock command="armada login" runner={runner} {...(keep === undefined ? {} : { keep })}>
        <code>armada login</code>
      </RunnableBlock>,
    );
  show();
  return { show, leave: () => root.render(<p>Another Job</p>) };
}

const press = (name: string) => page.getByRole("button", { name, exact: true });

test("a card that leaves the screen leaves its run going, and one that comes back shows it still running", async () => {
  const p = program();
  const one = card(p.runner, "waiting-run:s1:w1");
  await press("Run").click();
  p.say({ kind: "out", text: "Open the browser to sign in\r\n" });
  await expect.element(press("Stop")).toBeVisible();

  one.leave();
  await expect.element(page.getByText("Another Job")).toBeVisible();
  expect(p.stops).not.toHaveBeenCalled();

  one.show();
  await expect.element(press("Stop")).toBeVisible();
  await expect.element(page.getByRole("log", { name: "Output" })).toHaveTextContent("Open the browser to sign in");
  expect(p.started).toEqual(["armada login"]);

  await press("Stop").click();
  expect(p.stops).toHaveBeenCalledTimes(1);
  await expect.element(press("Run again")).toBeVisible();
});

test("a run that ended while its card was away is found ended, with its exit", async () => {
  const p = program();
  const one = card(p.runner, "waiting-run:s1:w2");
  await press("Run").click();
  one.leave();
  p.say({ kind: "end", code: 1 });
  one.show();
  await expect.element(press("Run again")).toBeVisible();
  await expect.element(page.getByRole("img", { name: "Failed" })).toBeVisible();
});

test("a card with no key has nothing that could find its run again, so leaving stops it", async () => {
  const p = program();
  const one = card(p.runner, undefined);
  await press("Run").click();
  one.leave();
  await expect.poll(() => p.stops.mock.calls.length).toBe(1);
});

test("Run again starts the command again and shows it running", async () => {
  const p = program();
  card(p.runner, "waiting-run:s1:w3");
  await press("Run").click();
  p.say({ kind: "end", code: 1 });
  await press("Run again").click();
  expect(p.started).toEqual(["armada login", "armada login"]);
  await expect.element(press("Stop")).toBeVisible();
});

test.each([
  ["an exit code", { kind: "end", code: 1 }, "Exit 1"],
  ["a stop", { kind: "end", code: null, stopped: true }, "Stopped"],
  ["a signal", { kind: "end", code: null, stopped: false }, "Ended by a signal"],
] as const)("a run that ended with %s says so on its mark", async (_how, end, said) => {
  const p = program();
  card(p.runner, undefined);
  await press("Run").click();
  p.say(end);
  await expect.element(page.getByRole("img", { name: "Failed" })).toBeVisible();
  // The bubble stays in the document while closed: the mark's wrapper is described by it.
  const wrapper = document.querySelector(".armada-prose__run-mark")?.parentElement;
  const bubble = document.getElementById(wrapper?.getAttribute("aria-describedby") ?? "");
  expect(bubble?.textContent).toBe(said);
});
