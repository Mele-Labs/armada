// Kill on job detail confirms by being held (the design system contract, "A hold
// confirms in place"). These pin what `Acts` sends for each way a hold ends, on
// fake timers so the length of the hold is the token's and not the test's wait.

import { afterEach, beforeEach, expect, test, vi, type Mock } from "vitest";
import { page } from "vitest/browser";

import type { JobDetail as JobWhole, JobSpend, JobSummary } from "@armada/protocol";
import { Acts, type ConfirmableAct, type HeldAct } from "./Acts";
import { mount, unmount } from "./mounted";

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  unmount();
});

// A running Job offers `kill_job` alone, which is the kill drawn as a control of its own — with a
// drone too: ending the Drone and leaving the Job open is the Drones sheet's act (owner, 29 Sep 2026).
// Held for money it offers `kill_job` as a split button's face and the raise behind the caret.
const DRONE = "01M1D0X0000016YK5S0JXBXDQ5";

/** A Job stopped at its cost cap, and what it has spent. */
const HELD_FOR_MONEY: Partial<JobSummary> = { status: "queued", queued_reason: "over_budget", budget_hold: "cost_cap" };
const SPENT: JobSpend = {
  cost_micros: 5_280_000,
  cost_cap_micros: 5_000_000,
  turns: 41,
  turn_cap: 300,
  ran_ms: 900_000,
  drones: 6,
};

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    ...over,
  };
}

let asked: Mock<(act: ConfirmableAct, jobId: string) => void>;
let killed: Mock<(act: HeldAct, jobId: string) => void>;

beforeEach(() => {
  asked = vi.fn<(act: ConfirmableAct, jobId: string) => void>();
  killed = vi.fn<(act: HeldAct, jobId: string) => void>();
});

function draw(over: Partial<JobSummary> = {}, spend?: JobSpend): void {
  const summary = job(over);
  const whole: JobWhole | null =
    spend === undefined
      ? null
      : { job: summary, created_at: summary.created_at, steps: [], acceptance_criteria: [], dependencies: [], spend };
  mount(
    <Acts
      job={summary}
      whole={whole}
      render="working"
      acting={false}
      approving={false}
      stale={false}
      onAct={asked}
      onActHeld={killed}
      onApprove={() => {}}
      onReport={async () => ({ ok: true })}
      reporting={false}
      onReporting={() => {}}
      onRaiseCap={() => {}}
      raising={false}
      onRaising={() => {}}
      onRaiseTurnCap={() => {}}
      raisingTurns={false}
      onRaisingTurns={() => {}}
      onCopied={() => {}}
    />,
  );
}

/** The drawn control, once it is on the page, with timers faked from here on. */
async function held(name: string): Promise<{ button: HTMLElement; hold: number }> {
  await expect.element(page.getByRole("button", { name })).toBeInTheDocument();
  const button = page.getByRole("button", { name }).element() as HTMLElement;
  const hold = parseFloat(getComputedStyle(button).getPropertyValue("--duration-hold"));
  expect(hold).toBeGreaterThan(0);
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  return { button, hold };
}

function pointer(button: HTMLElement, type: string): void {
  button.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, isPrimary: true }));
}

function key(button: HTMLElement, type: string, repeat = false): void {
  button.dispatchEvent(new KeyboardEvent(type, { bubbles: true, cancelable: true, key: " ", repeat }));
}

test("letting go before the hold is up kills nothing and asks nothing", async () => {
  draw();
  const { button, hold } = await held("Hold to kill job");
  pointer(button, "pointerdown");
  vi.advanceTimersByTime(hold - 1);
  pointer(button, "pointerup");
  vi.advanceTimersByTime(hold * 2);
  expect(killed).not.toHaveBeenCalled();
  expect(asked).not.toHaveBeenCalled();
});

test("leaving the control mid-hold cancels it", async () => {
  draw();
  const { button, hold } = await held("Hold to kill job");
  pointer(button, "pointerdown");
  vi.advanceTimersByTime(hold / 2);
  // React derives `onPointerLeave` from `pointerout` naming where the pointer went, as a browser sends it.
  button.dispatchEvent(new PointerEvent("pointerout", { bubbles: true, isPrimary: true, relatedTarget: document.body }));
  vi.advanceTimersByTime(hold * 2);
  expect(killed).not.toHaveBeenCalled();
});

test("holding Space for the whole duration kills once, through auto-repeat, with no dialog", async () => {
  draw();
  const { button, hold } = await held("Hold to kill job");
  key(button, "keydown");
  for (let at = 0; at < 20; at += 1) {
    vi.advanceTimersByTime(hold / 10);
    key(button, "keydown", true);
  }
  key(button, "keyup");
  expect(killed).toHaveBeenCalledTimes(1);
  expect(killed).toHaveBeenCalledWith("kill_job", "01M130Y1380016YK5S0JXBXDQ5");
  expect(asked).not.toHaveBeenCalled();
});

function reduceMotion(): void {
  const real = window.matchMedia;
  vi.spyOn(window, "matchMedia").mockImplementation((query: string) => {
    if (!query.includes("prefers-reduced-motion")) return real.call(window, query);
    return Object.assign(new EventTarget() as MediaQueryList, { matches: true, media: query, onchange: null });
  });
}

test("under reduced motion the hold is not offered, and a press asks", async () => {
  reduceMotion();
  draw();
  await expect.element(page.getByRole("button", { name: "Hold to kill job" })).not.toBeInTheDocument();
  await page.getByRole("button", { name: "Kill job" }).click();
  expect(asked).toHaveBeenCalledWith("kill_job", "01M130Y1380016YK5S0JXBXDQ5");
  expect(killed).not.toHaveBeenCalled();
});

// A drone on the Job changes nothing here: the hold ends the Job.

test("with a drone on the Job, the hold kills the Job and never offers the Drone's kill", async () => {
  draw({ assigned_drone: DRONE });
  const { button, hold } = await held("Hold to kill job");
  expect(document.body.textContent).not.toContain("Hold to kill drone");
  key(button, "keydown");
  for (let at = 0; at < 20; at += 1) {
    vi.advanceTimersByTime(hold / 10);
    key(button, "keydown", true);
  }
  key(button, "keyup");
  expect(killed).toHaveBeenCalledTimes(1);
  expect(killed).toHaveBeenCalledWith("kill_job", "01M130Y1380016YK5S0JXBXDQ5");
  expect(asked).not.toHaveBeenCalled();
});

// The same hold, on a split button's face: a Job held for money.

test("letting go of the split button's face before the hold is up kills nothing and asks nothing", async () => {
  draw(HELD_FOR_MONEY, SPENT);
  const { button, hold } = await held("Hold to kill job");
  pointer(button, "pointerdown");
  vi.advanceTimersByTime(hold - 1);
  pointer(button, "pointerup");
  vi.advanceTimersByTime(hold * 2);
  expect(killed).not.toHaveBeenCalled();
  expect(asked).not.toHaveBeenCalled();
});

test("leaving the split button's face mid-hold cancels it", async () => {
  draw(HELD_FOR_MONEY, SPENT);
  const { button, hold } = await held("Hold to kill job");
  pointer(button, "pointerdown");
  vi.advanceTimersByTime(hold / 2);
  button.dispatchEvent(new PointerEvent("pointerout", { bubbles: true, isPrimary: true, relatedTarget: document.body }));
  vi.advanceTimersByTime(hold * 2);
  expect(killed).not.toHaveBeenCalled();
  expect(asked).not.toHaveBeenCalled();
});

test("holding Space on the split button's face kills the job once, with no dialog", async () => {
  draw(HELD_FOR_MONEY, SPENT);
  const { button, hold } = await held("Hold to kill job");
  key(button, "keydown");
  for (let at = 0; at < 20; at += 1) {
    vi.advanceTimersByTime(hold / 10);
    key(button, "keydown", true);
  }
  key(button, "keyup");
  expect(killed).toHaveBeenCalledTimes(1);
  expect(killed).toHaveBeenCalledWith("kill_job", "01M130Y1380016YK5S0JXBXDQ5");
  expect(asked).not.toHaveBeenCalled();
});

test("the caret opens the menu and never holds, and what is behind it kills nothing", async () => {
  draw(HELD_FOR_MONEY, SPENT);
  const { hold } = await held("Hold to kill job");
  const caret = page.getByRole("button", { name: "Everything else this job can do" }).element() as HTMLElement;
  pointer(caret, "pointerdown");
  key(caret, "keydown");
  vi.advanceTimersByTime(hold * 2);
  expect(killed).not.toHaveBeenCalled();
  expect(asked).not.toHaveBeenCalled();
  key(caret, "keyup");
  pointer(caret, "pointerup");
  vi.useRealTimers();
  caret.click();
  await expect.element(page.getByRole("menu")).toBeVisible();
  await page.getByRole("menuitem", { name: "Raise the cost cap" }).click();
  expect(asked).not.toHaveBeenCalled();
  expect(killed).not.toHaveBeenCalled();
});

test("under reduced motion the split button's face asks rather than holds", async () => {
  reduceMotion();
  draw(HELD_FOR_MONEY, SPENT);
  await expect.element(page.getByRole("button", { name: "Hold to kill job" })).not.toBeInTheDocument();
  await page.getByRole("button", { name: "Kill job", exact: true }).click();
  expect(asked).toHaveBeenCalledWith("kill_job", "01M130Y1380016YK5S0JXBXDQ5");
  expect(killed).not.toHaveBeenCalled();
});
