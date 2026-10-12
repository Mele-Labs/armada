// The toast that says a call has come for the owner while he is somewhere else in Bridge, and takes
// him to it. A call the cockpit deals is what the Cockpit draws as a card over the glass; this is the
// same set read from outside, so an alert and the card it opens are one fact and cannot disagree.
// Mock only, as the Dashboard is.

import { useEffect, useRef, useState } from "react";
import { Toast } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";

import type { BridgeState } from "../../../shared/bridge";
import { useItems, type Hosts } from "../Dashboard";
import { alertOf, arrived, type Alert, type Held } from "./arrivals";

/** How long an alert stands unpressed. A call is still there on the glass when it goes. */
export const ALERT_MS = 15_000;

/**
 * How long after the first reading nothing is told. **Sessions arrive on their own channel**, a little
 * after the Jobs do, so a Session's question standing since Friday would otherwise read as arriving at
 * once on the heels of the seeding reading. An object so a test can shorten it.
 */
export const settle = { ms: 1500 };

/** The most that stand at once; the oldest goes when a fourth comes. */
const MOST = 3;

/** `useItems` draws acts it never calls here, and a stable object keeps its memo. */
const NO_HOSTS: Hosts = { onOpen: () => {}, onOpenSession: () => {}, onOpenLink: () => {} };
const NONE: ReadonlySet<string> = new Set();

type Standing = Alert & { until: number };

export function CallAlerts({
  state,
  picked,
  nowViews,
  onCockpit,
  onShow,
}: {
  state: BridgeState;
  picked: RepositorySummary | null;
  nowViews: Readonly<Record<string, CallView>> | undefined;
  /** The cockpit is what is on screen, and its card is already in front: nothing to point at. */
  onCockpit: boolean;
  /** Bring this call forward on the cockpit. */
  onShow: (key: string) => void;
}) {
  const calls = useItems("command-central", state, picked, nowViews, NO_HOSTS, NONE);
  const [standing, setStanding] = useState<readonly Standing[]>([]);
  const held = useRef<Held>(null);
  const first = useRef<number | null>(null);
  const here = useRef(onCockpit);
  here.current = onCockpit;

  // **Not before Fleet has answered once**: Bridge publishes state before it is connected to anything, and
  // seeding off an empty one would make the first real reading look like every call arriving at once.
  useEffect(() => {
    if (state.readAt === null) return;
    const at = Date.now();
    first.current ??= at;
    const settled = at - first.current >= settle.ms;
    const fresh = settled && !here.current ? arrived(held.current, calls) : [];
    held.current = new Set(calls.map((one) => one.key));
    const told = alertOf(fresh);
    setStanding((was) => {
      const live = new Set(calls.map((one) => one.key));
      // One that no longer stands has been answered or dismissed somewhere, and says nothing now.
      const kept = was.filter((one) => one.keys.some((key) => live.has(key)));
      return told === null ? kept : [...kept, { ...told, until: at + ALERT_MS }].slice(-MOST);
    });
  }, [calls, state.readAt]);

  // The cockpit coming up puts the card in front, which is what an alert was pointing at.
  useEffect(() => {
    if (onCockpit) setStanding([]);
  }, [onCockpit]);

  useEffect(() => {
    if (standing.length === 0) return;
    const next = Math.min(...standing.map((one) => one.until));
    const timer = window.setTimeout(() => setStanding((was) => was.filter((one) => one.until > Date.now())), Math.max(0, next - Date.now()) + 10);
    return () => window.clearTimeout(timer);
  }, [standing]);

  return (
    <>
      {standing.map((one) => (
        <Toast
          key={one.key}
          {...(one.status === undefined ? {} : { status: one.status })}
          onPress={() => {
            setStanding((was) => was.filter((other) => other !== one));
            onShow(one.key);
          }}
        >
          {one.sentence}
        </Toast>
      ))}
    </>
  );
}
