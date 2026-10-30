// What a fake's slices share: the state it publishes, the one way to change it, and the few
// things more than one surface reads or writes. Built once per window by `fake.ts`.

import type { KitAllowedCommand, WorktreesHeld } from "@armada/protocol";

import type { BridgeState } from "../../../shared/bridge";
import { unanswered } from "./moment";
import type { Scenario } from "./moment";

export type Fleet = {
  scenario: Scenario;
  state: () => BridgeState;
  /** Publish a change — on a microtask, because main's reach a window over IPC, never inside the call. */
  publish: (change: Partial<BridgeState>) => void;
  /** Hear every published state. */
  listen: (onState: (state: BridgeState) => void) => () => void;
  /** The pool as acts on it left it, read again after each as main does. */
  held: { get: () => WorktreesHeld | undefined; set: (held: WorktreesHeld | undefined) => void };
  /** What Kit's allowlist holds on this machine. */
  kit: { get: () => KitAllowedCommand[]; set: (allowed: KitAllowedCommand[]) => void };
  refused: (route: string) => { ok: false; outcome: ReturnType<typeof unanswered> };
  unread: (route: string) => { state: "failed"; outcome: ReturnType<typeof unanswered> };
};

/** The surfaces a fake is composed of, one `shared/api/<name>.ts` each. */
export type SliceName =
  | "core"
  | "studios"
  | "manifest"
  | "setup"
  | "workflows"
  | "helm"
  | "settings"
  | "overview"
  | "cleanup"
  | "dispatch"
  | "checks"
  | "lessons"
  | "sessions"
  | "sleep"
  | "accounts"
  | "planUsage"
  | "modelDownshift"
  | "projects"
  | "bugReports"
  | "lifecycles"
  | "agentSignIn"
  | "clashes"
  | "reviews"
  | "territory"
  | "timeline"
  | "actFailed"
  | "bayHistory"
  | "sessionSystem"
  | "usageSummary"
  | "issues"
  | "autopilot"
  | "sketchShare"
  | "attachments"
  | "files"
  | "flightControls"
  | "onboarding"
  | "requests"
  | "sharing"
  | "logbook"
  | "teaching"
  | "pullRequestNotes";

/**
 * One surface's part of a fake. `api` is the members its `shared/api/<name>.ts` declares, answered
 * from the scenario; `state` is what that surface's fields hold when nothing has read them yet,
 * which a window mounted without the slice keeps. `scenarios` are the moments only this surface's
 * Fleet makes, listed by `scenario.ts` where it lists the rest.
 */
export type Slice<A, S> = {
  name: SliceName;
  api: (scenario: Scenario, fleet: Fleet) => A;
  state: S;
  scenarios?: () => Scenario[];
  /**
   * What this surface's Fleet would have answered on connecting, where the scenario's own state
   * says nothing — a read main makes once per connection. Applied only when the slice is mounted.
   */
  seeded?: (state: BridgeState) => Partial<S>;
};

/** The fixed answers a read with nothing behind it gives. */
export const nothing = { state: "none" } as const;

/** One window's fleet, over `initial` as the state it opens on. */
export function fleetOf(scenario: Scenario, initial: BridgeState): Fleet {
  let state = initial;
  const listeners = new Set<(state: BridgeState) => void>();
  let held = scenario.held;
  let kit: KitAllowedCommand[] = [];

  function publish(change: Partial<BridgeState>): void {
    state = { ...state, ...change };
    const now = state;
    queueMicrotask(() => listeners.forEach((listener) => listener(now)));
  }

  const refused = (route: string) => ({ ok: false, outcome: unanswered(route) }) as const;

  return {
    scenario,
    state: () => state,
    publish,
    listen: (onState) => {
      listeners.add(onState);
      return () => listeners.delete(onState);
    },
    held: { get: () => held, set: (next) => void (held = next) },
    kit: { get: () => kit, set: (next) => void (kit = next) },
    refused,
    unread: (route) => ({ state: "failed", outcome: unanswered(route) }) as const,
  };
}
