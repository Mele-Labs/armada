// Whether the Cockpit is powering on: Fleet not answering yet, or answering a moment ago. A restart reads connected, unreachable,
// not running, connecting, starting, connected, and every state between the first and the last is the deck coming online rather
// than a fault. Past a grace a Fleet still down is a fault, and the fault draws once.

import { createContext, useContext, useEffect, useRef, useState } from "react";
import type { Connection } from "@armada/protocol";

/** How long a Fleet that is down reads as one coming back, counted from Bridge opening or from the last time it answered. */
export const POWER_GRACE_MS = 30_000;
/** The least a power-on lasts, on every launch, even where Fleet answers at once. */
export const POWER_LEAST_MS = 1_000;
/** How long the panels take to fill once Fleet answers: the last panel's delay and its fade. `power.css` staggers inside it. */
export const POWER_FILL_MS = 1_400;

/**
 * Standby: the panels are drawn and waiting. Filling: Fleet answered and the panels take their data in turn. Kept: Bridge opened on the picture
 * it kept on disk, so the panels are already full and Fleet not answering yet is no fault; it draws as live, and holds the banner as standby does.
 */
export type Power = "standby" | "filling" | "kept" | "live" | "fault";

/**
 * The instants the reading is taken from: Bridge opening, the connection last dropping, and the current connection starting.
 * `kept` is whether Bridge opened on the picture from disk and has not yet reached Fleet: there is no power-on to run.
 */
export type PowerClock = { opened: number; dropped: number | null; since: number | null; kept?: boolean };

const COMING_BACK: ReadonlySet<Connection["state"]> = new Set(["reading", "connecting", "starting"]);
const DOWN: ReadonlySet<Connection["state"]> = new Set(["not_running", "unreachable"]);

/** Where Fleet stands as the deck draws it, at `now`. */
export function powerOf(state: Connection["state"], now: number, clock: PowerClock): Power {
  const waiting = clock.kept === true ? "kept" : "standby";
  if (state === "connected") {
    if (clock.kept === true) return "live";
    const live = Math.max(clock.since ?? now, clock.opened + POWER_LEAST_MS);
    if (now < live) return "standby";
    return now < live + POWER_FILL_MS ? "filling" : "live";
  }
  if (COMING_BACK.has(state)) return waiting;
  if (DOWN.has(state)) return now < Math.max(clock.opened, clock.dropped ?? clock.opened) + POWER_GRACE_MS ? waiting : "fault";
  return "fault";
}

/** The next instant the reading moves by the clock alone, or none. */
export function nextPowerAt(state: Connection["state"], now: number, clock: PowerClock): number | undefined {
  const marks: number[] = [];
  if (state === "connected") {
    if (clock.kept === true) return undefined;
    const live = Math.max(clock.since ?? now, clock.opened + POWER_LEAST_MS);
    marks.push(live, live + POWER_FILL_MS);
  } else if (DOWN.has(state)) {
    marks.push(Math.max(clock.opened, clock.dropped ?? clock.opened) + POWER_GRACE_MS);
  }
  return marks.filter((at) => at > now).sort((a, b) => a - b)[0];
}

/** The reading for the window's one connection, moved on by its own timer at each instant the clock alone changes it. */
export function usePower(connection: Connection, drawnFromDisk = false): Power {
  const clock = useRef<PowerClock>({ opened: Date.now(), dropped: null, since: null });
  const [, tick] = useState(0);
  const now = Date.now();
  const held = clock.current;
  // Only the launch is kept: once Fleet has answered or dropped, a later reconnect powers on as it always did.
  if (drawnFromDisk && held.since === null && held.dropped === null) held.kept = true;
  if (connection.state === "connected") {
    if (held.since === null) held.since = now;
  } else if (held.since !== null) {
    held.kept = false;
    held.since = null;
    held.dropped = now;
  }
  const power = powerOf(connection.state, now, held);
  const next = nextPowerAt(connection.state, now, held);
  useEffect(() => {
    if (next === undefined) return;
    const timer = window.setTimeout(() => tick((was) => was + 1), Math.max(0, next - Date.now()) + 16);
    return () => window.clearTimeout(timer);
  }, [next, power]);
  return power;
}

/** The reading, handed from the window to the deck. Live where nothing provides it: a deck drawn alone is not powering on. */
export const PowerContext = createContext<Power>("live");
export const usePowerOn = (): Power => useContext(PowerContext);
/** Whether the deck is drawing its power-on: waiting on Fleet, or taking its data in turn. */
export const powering = (power: Power): power is "standby" | "filling" => power === "standby" || power === "filling";
