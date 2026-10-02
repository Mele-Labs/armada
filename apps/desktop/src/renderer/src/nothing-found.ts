// A read-in that came back with nothing to place is said in a toast. The owner, 2 Oct 2026: "there
// should be a toast notification or something saying nothing was found." Its Finding still says
// what the scout said; the toast is for a person looking at a Zone with only the Finding in it.
//
// **Said on the change, never on a Studio opened afterwards.** A read-in Bridge never saw reading
// is one nobody is waiting on, and a toast for it would be news about the past.

import { useEffect, useRef } from "react";
import type { Studio, StudioNode } from "@armada/protocol";

export const NOTHING_FOUND = "The read-in found nothing.";

/** A Finding a read-in made: one a scout was handed a source for. */
const readIn = (node: StudioNode): node is Extract<StudioNode, { kind: "finding" }> =>
  node.kind === "finding" && (node.sources ?? []).length > 0;

/** The read-ins in `studio` whose scout is still reading. */
export function stillReading(studio: Studio): ReadonlySet<string> {
  return new Set(studio.nodes.filter(readIn).flatMap((node) => (node.ended === undefined ? [node.id] : [])));
}

/**
 * Whether a read-in that was still reading has come back answered with nothing beside its Finding
 * in its Zone. **Answered only**: a stopped or failed scout says so on its Finding, and "found
 * nothing" would be a claim about a source nobody finished reading.
 */
export function cameBackEmpty(reading: ReadonlySet<string>, studio: Studio): boolean {
  return studio.nodes.some(
    (node) =>
      reading.has(node.id) &&
      node.kind === "finding" &&
      node.ended?.outcome === "answered" &&
      node.within !== undefined &&
      !studio.nodes.some((other) => other.id !== node.id && other.within === node.within),
  );
}

/** Say `NOTHING_FOUND` when a read-in on the open Studio comes back with nothing. */
export function useNothingFound(studio: Studio | null, onSaid: (sentence: string) => void): void {
  const reading = useRef<ReadonlySet<string>>(new Set());
  useEffect(() => {
    if (studio === null) {
      reading.current = new Set();
      return;
    }
    if (cameBackEmpty(reading.current, studio)) onSaid(NOTHING_FOUND);
    reading.current = stillReading(studio);
  }, [studio, onSaid]);
}
