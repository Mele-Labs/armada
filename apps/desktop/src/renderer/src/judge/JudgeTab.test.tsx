// The Judge tab: what judging is and the box that asks, always; above them what was asked and how the reading stands. Browser: a rendering and a press are what are read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { JudgeReading } from "@armada/protocol";

import { JudgeTab } from "./JudgeTab";
import { askForNotes, provideJudge, putReadings, type JudgeApi } from "./store";
import "../styles/index.css";

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const JOB = "job-1";
const TITLE = "Switch sign-in";
const ARTIFACT = {
  kind: "job",
  id: JOB,
  title: TITLE,
} as const;

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
let judge: JudgeApi;
beforeEach(() => {
  localStorage.clear();
  judge = {
    ask: vi.fn(),
    send: vi.fn(),
    dismiss: vi.fn(),
    markRead: vi.fn(),
  };
  provideJudge(judge);
});
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  provideJudge(undefined);
});

function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<JudgeTab job={JOB} title={TITLE} />));
  roots.push({
    root,
    host,
  });
  return host;
}

const reading = (more: Partial<JudgeReading>): JudgeReading => ({
  id: "r1",
  artifact: ARTIFACT,
  prompt: "",
  state: "done",
  asked_at: "2026-10-31T14:00:00Z",
  read: false,
  notes: [],
  ...more,
});
const box = (host: HTMLElement) =>
  host.querySelector<HTMLElement>('[role="textbox"][aria-label="Verify that"]')!;
const run = (host: HTMLElement) => host.querySelector<HTMLButtonElement>('button[type="submit"]')!;

test("with no reading it says what judging is and holds the box, and nothing else", () => {
  const host = mount();
  expect(host.textContent).toContain("A Judge reads this Job once");
  expect(box(host)).not.toBeNull();
  expect(run(host).textContent).toContain("Judge");
  expect(host.querySelector("dl")).toBeNull();
});

test("a window with no judge to ask draws nothing", () => {
  provideJudge(undefined);
  expect(mount().textContent).toBe("");
});

test("Judge on an empty box asks what was missed, about the whole Job", async () => {
  const host = mount();
  await act(async () => run(host).click());
  expect(judge.ask).toHaveBeenCalledWith({
    job: JOB,
    artifact: ARTIFACT,
    prompt: "",
  });
});

test("a reading says what was asked and draws its shape, and the box cannot ask again", () => {
  act(() =>
    putReadings(JOB, [
      reading({
        prompt: "existing sign-ins survive",
        state: "reading",
      }),
    ]),
  );
  const host = mount();
  expect(host.querySelector("dl")!.textContent).toContain("Verify that");
  expect(host.querySelector("dl")!.textContent).toContain("existing sign-ins survive");
  expect(
    host.querySelector('[role="status"][aria-label="Judge reading"] .armada-skeleton'),
  ).not.toBeNull();
  expect(run(host).disabled).toBe(true);
});

test("an empty prompt reads as the question it asked", () => {
  act(() =>
    putReadings(JOB, [
      reading({
        state: "reading",
      }),
    ]),
  );
  expect(mount().querySelector("dl")!.textContent).toContain("What did we miss?");
});

test("a failed reading says why, and the box can ask again", () => {
  act(() =>
    putReadings(JOB, [
      reading({
        state: "failed",
        failed: "Fleet restarted while it was reading.",
      }),
    ]),
  );
  const host = mount();
  expect(host.textContent).toContain("Fleet restarted while it was reading.");
  expect(host.querySelector('[role="status"]')).toBeNull();
  expect(run(host).disabled).toBe(false);
});

test("a clean reading says nothing was missed", () => {
  act(() =>
    putReadings(JOB, [
      reading({}),
    ]),
  );
  expect(mount().textContent).toContain("Nothing missed");
});

test("every note of the reading stands, and drawing them reads them", () => {
  act(() =>
    putReadings(JOB, [
      reading({
        notes: [
          {
            id: "n1",
            claim: "No task names the cookie.",
            where: "Plan",
            about: "T2",
          },
          {
            id: "n2",
            claim: "A lifetime of zero never expires.",
            where: "tokens.rs:41",
            about: "tokens.rs:41",
          },
        ],
      }),
    ]),
  );
  const host = mount();
  expect(host.querySelector('[aria-label^="Finding: No task names the cookie"]')).not.toBeNull();
  expect(
    host.querySelector('[aria-label^="Finding: A lifetime of zero never expires"]'),
  ).not.toBeNull();
  expect(judge.markRead).toHaveBeenCalledWith("r1");
});

test("the box takes the keyboard when the Judge's press asked, and not when the tab came back by itself", () => {
  const host = mount();
  expect(document.activeElement).not.toBe(box(host));
  act(() => askForNotes(JOB, ARTIFACT));
  expect(document.activeElement).toBe(box(host));
});
