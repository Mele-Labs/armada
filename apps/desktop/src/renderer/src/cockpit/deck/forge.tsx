// How old the merge queue's reading is, and whether the forge has stopped answering, as main's light says it. Fleet sends both per
// repository (`readAt`, `quietSince`) and the queue is one light across them, so the stalest read and every quiet repository speak.

import { Ticking, timeFormat, timeSaid, type TimeFormat } from "@armada/components";
import type { MergeLineView } from "@armada/screens";

import { spanOf } from "./bays";

export type Forge = {
  /** The oldest `readAt` among the repositories shown, in ms. Absent where none was sent. */
  readAt?: number;
  /** Each repository whose forge has stopped answering, by name, and since when. */
  quiet: readonly { name: string; since: number }[];
};

export function forgeOf(views: readonly MergeLineView[], nameOf: (root: string) => string): Forge {
  const reads = views.flatMap((view) => (view.hub?.readAt === undefined ? [] : [Date.parse(view.hub.readAt)])).filter((at) => !Number.isNaN(at));
  const quiet = views.flatMap((view) => {
    const since = view.hub?.quietSince === undefined ? Number.NaN : Date.parse(view.hub.quietSince);
    return Number.isNaN(since) ? [] : [{ name: nameOf(view.root), since }];
  });
  return { ...(reads.length === 0 ? {} : { readAt: Math.min(...reads) }), quiet };
}

/** `<forge> not answering since 18:40:00`, in the owner's time format, naming each repository only where more than one is served. */
export function quietSaid(forge: Forge, several: boolean, format: TimeFormat = timeFormat()): string | undefined {
  if (forge.quiet.length === 0) return undefined;
  if (!several) return `GitHub not answering since ${timeSaid(forge.quiet[0]!.since, format)}`;
  return `GitHub not answering for ${forge.quiet.map((one) => `${one.name} since ${timeSaid(one.since, format)}`).join(", ")}`;
}

/** `Read 40s ago`, `Read 3m ago`, `Read 2h 05m ago`, from how long ago in ms. */
export function readSaid(ms: number): string {
  if (ms < 5000) return "Read just now";
  return `Read ${ms < 60_000 ? `${Math.floor(ms / 1000)}s` : spanOf(ms)} ago`;
}

/** The age of the reading, moving as it is looked at. */
export function ReadAgo({ at }: { at: number }) {
  return <Ticking step={5000}>{(now) => readSaid(Math.max(0, now - at))}</Ticking>;
}
