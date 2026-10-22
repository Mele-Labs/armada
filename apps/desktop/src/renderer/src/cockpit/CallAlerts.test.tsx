// The alert for a call that has come while the owner is elsewhere: it names the call, a press takes him to
// it, and it says nothing for what was already waiting, for what the cockpit is already showing, or for
// what has been dealt with by the time it would have stood.

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { createRoot, type Root } from "react-dom/client";
import { act } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import "../styles/index.css";
import type { BridgeState } from "../../../shared/bridge";
import { connected } from "../mock/moment";
import { SessionsFrom } from "../sessions-draft";
import { ALERT_MS, CallAlerts, settle } from "./CallAlerts";
import { forgetDismissals } from "./dismissed";
import { forgetViewed } from "./viewed";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
const SAVED = { ...settle };

beforeEach(() => {
  forgetDismissals();
  // A Session another file opened stays viewed in `localStorage`, and a call older than that view is not one.
  forgetViewed();
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

/** A Session with one question standing on it since `since`. */
const waiting = (n: number, title: string, since = "2026-10-10T09:00:00Z"): Session =>
  ({
    id: `s${n}`,
    title,
    turn: { state: "idle" },
    lastTurnAt: since,
    rows: [],
    attachments: [],
    waitingFor: [{ id: `q${n}`, text: "Which one?", since, source: "agent" }],
  }) as unknown as Session;
const read: BridgeState = { ...connected([], []), readAt: Date.now() };

function draw(initial: Session[], onShow = vi.fn(), onCockpit = false) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const show = (sessions: Session[], atCockpit = onCockpit) => {
    const held = { get: () => sessions, subscribe: () => () => undefined } as unknown as SessionsDraft;
    return act(() =>
      root.render(
        <SessionsFrom held={held}>
          <CallAlerts state={read} picked={null} onCockpit={atCockpit} onShow={onShow} />
        </SessionsFrom>,
      ),
    );
  };
  show(initial);
  return { show, onShow };
}

const alert = (name: string | RegExp) => page.getByRole("button", { name });

test("a call that comes names itself, and pressing it takes the owner to that call", async () => {
  const { show, onShow } = draw([]);
  const session = waiting(1, "Fix the login");
  await show([session]);
  await alert("“Fix the login” needs you.").click();
  expect(onShow).toHaveBeenCalledWith("session:s1:q1");
  await expect.poll(() => alert(/needs you/).query()).toBeNull();
});

test("a call that was already waiting when Bridge read is not news", async () => {
  const { show } = draw([waiting(1, "Fix the login")]);
  await show([waiting(1, "Fix the login")]);
  expect(alert(/needs you/).query()).toBeNull();
});

test("several that come together are one alert, landing on the one that has waited longest", async () => {
  const { show, onShow } = draw([]);
  await show([waiting(1, "First", "2026-10-10T09:00:00Z"), waiting(2, "Second", "2026-10-10T08:00:00Z")]);
  await alert("2 calls need you.").click();
  expect(onShow).toHaveBeenCalledWith("session:s2:q2");
});

test("says nothing while the cockpit is what is on screen, because its card is already in front", async () => {
  const { show } = draw([], vi.fn(), true);
  await show([waiting(1, "Fix the login")]);
  expect(alert(/needs you/).query()).toBeNull();
});

test("goes when the call is dealt with elsewhere, and when it has stood unpressed long enough", async () => {
  const { show } = draw([]);
  const session = waiting(1, "Fix the login");
  await show([session]);
  await expect.element(alert(/needs you/)).toBeVisible();
  await show([]);
  expect(alert(/needs you/).query()).toBeNull();

  vi.useFakeTimers();
  await show([session]);
  await show([session, waiting(2, "Tidy the retry cap")]);
  await act(() => vi.advanceTimersByTime(ALERT_MS + 50));
  expect(alert(/needs you/).query()).toBeNull();
});
