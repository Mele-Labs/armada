// The alert for a call that has come while the owner is elsewhere: it names the call, a press takes him to
// it, and it says nothing for what was already waiting, for what the cockpit is already showing, or for
// what has been dealt with by the time it would have stood.

import { featureRunning } from "@armada/jobs/fake";
import type { JobSummary } from "@armada/protocol";
import { createRoot, type Root } from "react-dom/client";
import { act } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import "../styles/index.css";
import type { BridgeState } from "../../../shared/bridge";
import { asRow } from "../mock/holding";
import { connected } from "../mock/moment";
import { ALERT_MS, CallAlerts, settle } from "./CallAlerts";
import { forgetDismissals } from "./dismissed";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
const SAVED = { ...settle };

beforeEach(() => {
  forgetDismissals();
  settle.ms = 0;
});
afterEach(() => {
  Object.assign(settle, SAVED);
  vi.useRealTimers();
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const waiting = (n: number, title: string): JobSummary => {
  const { reclaimed_at: _cleared, ...job } = { ...asRow(featureRunning(), 90 + n, `alert-${n}`, title).job, status: "awaiting_review" as const };
  return job as JobSummary;
};
const read = (jobs: JobSummary[]): BridgeState => ({ ...connected(jobs, [], []), readAt: Date.now() });

function draw(initial: JobSummary[], onShow = vi.fn(), onCockpit = false) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const show = (jobs: JobSummary[], atCockpit = onCockpit) =>
    act(() => root.render(<CallAlerts state={read(jobs)} picked={null} nowViews={undefined} onCockpit={atCockpit} onShow={onShow} />));
  show(initial);
  return { show, onShow };
}

const alert = (name: string | RegExp) => page.getByRole("button", { name });

test("a call that comes names itself, and pressing it takes the owner to that call", async () => {
  const { show, onShow } = draw([]);
  const job = waiting(1, "Fix the login");
  await show([job]);
  await alert("“Fix the login” needs you.").click();
  expect(onShow).toHaveBeenCalledWith(job.id);
  await expect.poll(() => alert(/needs you/).query()).toBeNull();
});

test("a call that was already waiting when Bridge read is not news", async () => {
  const { show } = draw([waiting(1, "Fix the login")]);
  await show([waiting(1, "Fix the login")]);
  expect(alert(/needs you/).query()).toBeNull();
});

test("several that come together are one alert, landing on the one that has waited longest", async () => {
  const { show, onShow } = draw([]);
  const first = { ...waiting(1, "First"), started_at: "2026-10-10T09:00:00Z" };
  const second = { ...waiting(2, "Second"), started_at: "2026-10-10T08:00:00Z" };
  await show([first, second]);
  await alert("2 calls need you.").click();
  expect(onShow).toHaveBeenCalledWith(second.id);
});

test("says nothing while the cockpit is what is on screen, because its card is already in front", async () => {
  const { show } = draw([], vi.fn(), true);
  await show([waiting(1, "Fix the login")]);
  expect(alert(/needs you/).query()).toBeNull();
});

test("goes when the call is dealt with elsewhere, and when it has stood unpressed long enough", async () => {
  const { show } = draw([]);
  const job = waiting(1, "Fix the login");
  await show([job]);
  await expect.element(alert(/needs you/)).toBeVisible();
  await show([]);
  expect(alert(/needs you/).query()).toBeNull();

  vi.useFakeTimers();
  await show([job]);
  await show([job, waiting(2, "Tidy the retry cap")]);
  await act(() => vi.advanceTimersByTime(ALERT_MS + 50));
  expect(alert(/needs you/).query()).toBeNull();
});
