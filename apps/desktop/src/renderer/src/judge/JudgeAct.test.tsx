// The Judge act: a balance on an artifact's row that opens a small box asking what to verify, and reads the state of the last reading on its icon. Browser: a press, a key
// and a rendering are what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { JudgeReading } from "@armada/protocol";

import { JudgeAct } from "./JudgeAct";
import { provideJudge, putReadings, type JudgeApi, type JudgeArtifact } from "./store";
import "../styles/index.css";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const JOB = "job-1";
const ARTIFACT: JudgeArtifact = { kind: "pull_request", id: "2418", title: "#2418" };
/** A box keeps what was typed in it for the artifact, so a test that needs an empty one asks about another. */
const OTHER: JudgeArtifact = { kind: "pull_request", id: "2419", title: "#2419" };

const roots: { root: Root; host: HTMLElement }[] = [];
const api = (): JudgeApi => ({ ask: vi.fn(), send: vi.fn(), dismiss: vi.fn(), markRead: vi.fn() });
let judge: JudgeApi;
beforeEach(() => {
  judge = api();
  provideJudge(judge);
});
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  provideJudge(undefined);
});

function mount(artifact = ARTIFACT) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<JudgeAct job={JOB} artifact={artifact} label={`Judge ${artifact.title}`} />));
  roots.push({ root, host });
  return host;
}

const reading = (more: Partial<JudgeReading>): JudgeReading => ({ id: "r1", artifact: ARTIFACT, prompt: "", state: "done", asked_at: "2026-10-31T14:00:00Z", read: false, notes: [], ...more });
const press = (host: HTMLElement) => host.querySelector<HTMLButtonElement>("button[aria-haspopup='dialog']")!;
const popover = () => document.body.querySelector<HTMLElement>('[role="dialog"][aria-label="Judge"]');
const field = () => popover()!.querySelector<HTMLTextAreaElement>("textarea")!;
const mark = (host: HTMLElement, name: string) => host.querySelector(`[role="img"][aria-label="${name}"]`);

function type(box: HTMLTextAreaElement, text: string) {
  act(() => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(box, text);
    box.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
const key = (target: Element | Window, name: string) => act(() => void target.dispatchEvent(new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true })));

test("the button, inside its Tooltip, opens the Verify that box", () => {
  const host = mount();
  expect(popover()).toBeNull();
  act(() => press(host).click());
  expect(popover()).not.toBeNull();
  expect(popover()!.textContent).toContain("Verify that");
  expect(press(host).getAttribute("aria-expanded")).toBe("true");
});

test("Judge asks the api with the artifact and the trimmed prompt, and closes the box", () => {
  const host = mount();
  act(() => press(host).click());
  type(field(), "  existing sign-ins survive the switch  ");
  act(() => [...popover()!.querySelectorAll("button")].find((one) => one.textContent === "Judge")!.click());
  expect(judge.ask).toHaveBeenCalledWith({ job: JOB, artifact: ARTIFACT, prompt: "existing sign-ins survive the switch" });
  expect(popover()).toBeNull();
});

test("Enter in the box runs the same", () => {
  const host = mount(OTHER);
  act(() => press(host).click());
  type(field(), "the token lifetime is tested");
  key(field(), "Enter");
  expect(judge.ask).toHaveBeenCalledWith({ job: JOB, artifact: OTHER, prompt: "the token lifetime is tested" });
  expect(popover()).toBeNull();
});

test("an empty box asks what was missed", () => {
  const host = mount({ kind: "commit", id: "a41c9e2", title: "a41c9e2" });
  act(() => press(host).click());
  key(field(), "Enter");
  expect(judge.ask).toHaveBeenCalledWith({ job: JOB, artifact: { kind: "commit", id: "a41c9e2", title: "a41c9e2" }, prompt: "" });
});

test("Esc closes the box and asks nothing", () => {
  const host = mount();
  act(() => press(host).click());
  type(field(), "something");
  key(window, "Escape");
  expect(popover()).toBeNull();
  expect(judge.ask).not.toHaveBeenCalled();
});

test("a window with no judge to ask draws nothing", () => {
  provideJudge(undefined);
  const host = mount();
  expect(press(host)).toBeNull();
});

test("the icon reads the last reading: a mark while it reads", () => {
  act(() => putReadings(JOB, [reading({ state: "reading" })]));
  const host = mount();
  expect(mark(host, "Judge reading")).not.toBeNull();
});

test("the icon reads the last reading: a dot while its notes are unread", () => {
  act(() => putReadings(JOB, [reading({ notes: [{ id: "n1", claim: "No ceiling" }] })]));
  const host = mount();
  expect(mark(host, "Notes to read")).not.toBeNull();
  expect(mark(host, "Judge reading")).toBeNull();
});

test("the icon reads the last reading: a check after a clean one", () => {
  act(() => putReadings(JOB, [reading({})]));
  const host = mount();
  expect(mark(host, "Nothing missed")).not.toBeNull();
  expect(mark(host, "Notes to read")).toBeNull();
});

test("the icon reads the last reading: notes opened, or all settled, leave the plain balance", () => {
  act(() => putReadings(JOB, [reading({ read: true, notes: [{ id: "n1", claim: "No ceiling" }] })]));
  const host = mount();
  expect(host.querySelectorAll('[role="img"]')).toHaveLength(0);
});

test("no count is drawn, whatever the reading holds", () => {
  act(() => putReadings(JOB, [reading({ notes: [{ id: "n1", claim: "One" }, { id: "n2", claim: "Two" }, { id: "n3", claim: "Three" }] })]));
  const host = mount();
  expect(host.textContent).not.toMatch(/\d/);
});
