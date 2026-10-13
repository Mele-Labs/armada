// What fills each Session's context window. The harness reports it; the wire shape is not settled, so this
// is the view-model's own seam, in the camelCase the wire will have: nothing sets it in the app yet, and the
// mock does. Keyed by Session id.

import type { ContextBreakdown } from "@armada/components";

let held: ReadonlyMap<string, ContextBreakdown> = new Map();

export const contextBreakdowns = {
  of: (id: string): ContextBreakdown | undefined => held.get(id),
  set(entries: ReadonlyMap<string, ContextBreakdown>): void {
    held = entries;
  },
};
