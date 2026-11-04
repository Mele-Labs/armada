// What the Usage page's Time view and Burn per hour draw, read from Fleet through `window.armada`: the window's time, the 48
// hours with the builds that ran in them, and each build's tokens as a summary over its run. `read` is the page's one re-read,
// so these ride on the summary's cadence and add no timer of their own.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { UsageBuilds, UsageHours, UsageTime } from "@armada/components";
import type { UsageBreakdown, UsageHours as WireHours } from "@armada/protocol";

import { buildIdOf, buildsOf, hoursOf, isPlaced, runOf, timeOf } from "./usage-time-wire";

/** `pending` while the first answer is awaited, which draws a panel's shape; nothing when Fleet has given none. */
type Held<T> = T | "pending" | undefined;

export function useTimeReads(): {
  time: Held<UsageTime>;
  builds: Held<UsageBuilds>;
  hours: Held<UsageHours>;
  /** Asks Fleet again for all three. */
  read: () => void;
} {
  const [time, setTime] = useState<Held<UsageTime>>(undefined);
  const [wire, setWire] = useState<Held<WireHours>>(undefined);
  const [usages, setUsages] = useState<ReadonlyMap<string, UsageBreakdown>>(new Map());
  const kept = useRef(new Map<string, UsageBreakdown>());

  const read = useCallback(() => {
    // Nothing held yet draws its shape; an answer that fails returns it to nothing, and one already held stays.
    setTime((held) => held ?? "pending");
    setWire((held) => held ?? "pending");
    void window.armada.getUsageTime().then((answer) => {
      if (answer.ok) setTime(timeOf(answer.value));
      else setTime((held) => (held === "pending" ? undefined : held));
    });
    void window.armada.getUsageHours().then((answer) => {
      if (answer.ok) setWire(answer.value);
      else setWire((held) => (held === "pending" ? undefined : held));
    });
  }, []);

  // A build that has ended has a run that never changes, so its tokens are read once; the one running is read each time.
  // One at a time, newest first: the builds are many and the store is the owner's.
  useEffect(() => {
    if (typeof wire !== "object") return;
    let live = true;
    const tz = -new Date().getTimezoneOffset();
    void (async () => {
      for (const one of wire.builds.filter(isPlaced).reverse()) {
        const id = buildIdOf(one);
        if (kept.current.has(id) && one.until !== undefined) continue;
        const answer = await window.armada.getUsageSummary(tz, runOf(one, wire.until));
        if (!live) return;
        if (!answer.ok) continue;
        kept.current.set(id, answer.value.total);
        setUsages(new Map(kept.current));
      }
    })();
    return () => {
      live = false;
    };
  }, [
    wire,
  ]);

  const hours = useMemo(
    () => (typeof wire === "object" ? hoursOf(wire) : wire),
    [
      wire,
    ],
  );
  const builds = useMemo(
    () => (typeof wire === "object" ? buildsOf(wire, usages) : wire),
    [
      wire,
      usages,
    ],
  );
  return {
    time,
    builds,
    hours,
    read,
  };
}
