// What each Session has built, read once off the Sessions the window holds: the `build` Fleet carries on a Session's record, at the
// clock. A Session with none has no entry, and nothing new is drawn for it. #2301.

import { useMemo } from "react";

import { useSessions } from "../../sessions-draft";
import { useFleetEvents } from "../fleet-events";
import { CLOCK_MS, quantized } from "./field";
import { buildInOf, type BuildIn } from "./build";

export function useBuilds(now: number): ReadonlyMap<string, BuildIn> {
  const sessions = useSessions();
  const events = useFleetEvents();
  const clock = quantized(now, CLOCK_MS);
  return useMemo(
    () =>
      new Map(
        sessions.flatMap((one) => {
          if (one.build === undefined) return [];
          const build = buildInOf(one.id, one.build, clock, events);
          return [[build.key, build] as const];
        }),
      ),
    [sessions, events, clock],
  );
}
