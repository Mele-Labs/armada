// The dock's count is the Cockpit's calls: what he has already been in, and a walk he has open, are
// not calls, so they are not counted either. A Job with two items calling is one Job calling.

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { createRoot, type Root } from "react-dom/client";
import { act } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import type { BridgeState } from "../../../shared/bridge";
import { connected } from "../mock/moment";
import { SessionsFrom } from "../sessions-draft";
import { forgetDismissals } from "./dismissed";
import { DockCount } from "./DockCount";
import { forgetViewed, markViewed } from "./viewed";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const WALK = "http://localhost:5394/?walk=flight-control-head";
const roots: { root: Root; host: HTMLElement }[] = [];
let dockCount: ReturnType<typeof vi.fn>;

beforeEach(() => {
  forgetDismissals();
  forgetViewed();
  dockCount = vi.fn();
  // Main alone knows which walk windows are open; this one is.
  (window as { armada?: unknown }).armada = {
    dockCount,
    getSessionWindows: async () => [WALK],
    onSessionWindows: () => () => undefined,
  };
});
afterEach(() => {
  delete (window as { armada?: unknown }).armada;
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const job = (id: string, waitingFor: Session["waitingFor"]): Session =>
  ({ id, title: id, turn: { state: "idle" }, lastTurnAt: "2026-10-26T13:00:00Z", rows: [], attachments: [], waitingFor }) as unknown as Session;

/** The two Jobs the badge counted on 26 Oct while Needs you was empty. */
const restart = job("dev-mode", [
  { id: "restart-for-runs", text: "Restart Fleet onto main so it has app runs", since: "2026-10-26T13:07:02Z", source: "agent", act: { kind: "run", target: "scripts/preview --restart" } },
]);
const walk = job("flight-control", [{ id: `walk:${WALK}`, text: WALK, since: "2026-10-26T13:10:17Z", source: "walk", act: { kind: "walk", target: WALK } }]);

const read: BridgeState = { ...connected([], []), readAt: Date.now() };

async function draw(sessions: Session[], state: BridgeState = read) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const held = { get: () => sessions, subscribe: () => () => undefined } as unknown as SessionsDraft;
  await act(async () =>
    root.render(
      <SessionsFrom held={held}>
        <DockCount state={state} />
      </SessionsFrom>,
    ),
  );
}

const told = () => dockCount.mock.calls.at(-1)?.[0];

test("a Job he has been in since it asked, and a walk he has open, leave the dock at nothing", async () => {
  markViewed("dev-mode", Date.parse("2026-10-26T13:08:00Z"));
  await draw([restart, walk]);
  await expect.poll(told).toBe(0);
});

test("a Job calling counts once, however many of its items call", async () => {
  const asks = job("asks", [
    { id: "q1", text: "Which one?", since: "2026-10-26T13:00:00Z", source: "agent" },
    { id: "q2", text: "And this?", since: "2026-10-26T13:01:00Z", source: "agent" },
  ]);
  await draw([asks, restart]);
  await expect.poll(told).toBe(2);
});

test("nothing is told before Fleet has answered once", async () => {
  await draw([restart], { ...read, readAt: null });
  expect(dockCount).not.toHaveBeenCalled();
});
